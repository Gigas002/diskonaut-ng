use ::std::ffi::{OsStr, OsString};
use ::std::fs::{self, File};
use ::std::io::Write;
use ::std::path::PathBuf;
use ::std::sync::mpsc;

use libdiskonaut::{ScanItem, ScanOptions, scan_folder};
use ratatui::backend::TestBackend;

use super::{App, UiMode};
use crate::config::Keybinds;

fn temp_app_dir(name: &str) -> PathBuf {
    tempfile::Builder::new()
        .prefix(&format!("diskonaut_app_test_{name}"))
        .tempdir()
        .expect("create temp dir")
        .keep()
}

fn app_with_scanned_dir(dir: &PathBuf, width: u16, height: u16) -> App<TestBackend> {
    let (tx, _rx) = mpsc::sync_channel(1);
    let mut app = App::new(
        TestBackend::new(width, height),
        dir.clone(),
        tx,
        true,
        Keybinds::default(),
    );
    let options = ScanOptions {
        parallel: false,
        show_apparent_size: true,
        one_file_system: false,
    };
    for item in scan_folder(dir, options) {
        if let ScanItem::Entry {
            metadata,
            path: entry_path,
        } = item
        {
            app.add_entry_to_base_folder(&metadata, entry_path);
        }
    }
    app.start_ui();
    app
}

#[test]
fn render_marks_screen_too_small_below_minimum_size() {
    let dir = temp_app_dir("too_small");
    let mut app = app_with_scanned_dir(&dir, 40, 10);
    app.render();
    assert!(matches!(app.ui_mode, UiMode::ScreenTooSmall));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn enter_selected_enters_subfolder() {
    let dir = temp_app_dir("enter_subfolder");
    let sub = dir.join("sub");
    fs::create_dir(&sub).expect("create subfolder");
    let mut big_in_sub = File::create(sub.join("large.dat")).expect("create file in sub");
    big_in_sub
        .write_all(&vec![b'x'; 8192])
        .expect("write subfolder data");
    File::create(dir.join("tiny.txt"))
        .expect("create small root file")
        .write_all(b"x")
        .expect("write root file");

    let mut app = app_with_scanned_dir(&dir, 80, 24);
    assert!(
        app.file_tree
            .item_in_current_folder(OsStr::new("sub"))
            .is_some(),
        "scan should register subfolder in tree"
    );
    app.board.move_to_largest_folder();
    let selected = app
        .board
        .currently_selected()
        .expect("a folder tile should be selected");
    assert_eq!(selected.name, OsStr::new("sub"));
    assert_eq!(selected.file_type, libdiskonaut::tiles::FileType::Folder);
    app.handle_enter();

    assert_eq!(app.file_tree.get_current_path(), sub);
    assert_eq!(
        app.file_tree.current_folder_names,
        vec![OsString::from("sub")]
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn prompt_file_deletion_shows_confirmation() {
    let dir = temp_app_dir("delete_confirm");
    let target = dir.join("remove_me.txt");
    File::create(&target).expect("create file");

    let (tx, _rx) = mpsc::sync_channel(1);
    let mut app = App::new(
        TestBackend::new(80, 24),
        dir.clone(),
        tx,
        true,
        Keybinds::default(),
    );
    let meta = fs::metadata(&target).expect("metadata");
    app.add_entry_to_base_folder(&meta, target.clone());
    app.start_ui();
    let file_index = app
        .board
        .tiles
        .iter()
        .position(|t| t.name == OsStr::new("remove_me.txt"))
        .expect("file tile");
    app.board.set_selected_index(&file_index);
    app.prompt_file_deletion();

    assert!(target.exists(), "file should remain until user confirms");
    assert!(matches!(app.ui_mode, UiMode::DeleteFile(_)));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reset_ui_mode_from_error_returns_to_normal() {
    let dir = temp_app_dir("reset_mode");
    let mut app = app_with_scanned_dir(&dir, 80, 24);
    app.ui_mode = UiMode::ErrorMessage("oops".into());
    app.reset_ui_mode();
    assert!(matches!(app.ui_mode, UiMode::Normal));
    let _ = fs::remove_dir_all(&dir);
}

/// Every screen at every size, with names wider in bytes than in columns
/// (Cyrillic, CJK, emoji) and longer than any terminal: drawing must never
/// panic (it would leave the terminal in raw mode).
#[test]
fn drawing_never_panics() {
    let dir = temp_app_dir("drawing_never_panics");
    let names = [
        "a".repeat(200),
        "ж".repeat(100),
        "漢字".repeat(40),
        "🦀".repeat(60),
        "x".to_string(),
    ];
    for (i, name) in names.iter().enumerate() {
        let mut file = File::create(dir.join(name)).expect("create file");
        file.write_all(&vec![0u8; 4096 * (i + 1)]).expect("write");
    }
    fs::create_dir(dir.join("ü".repeat(120))).expect("create folder");
    // Not UTF-8 (Latin-1 "été"): valid on Linux, shown lossily.
    use std::os::unix::ffi::OsStrExt;
    let mut latin1 = File::create(dir.join(OsStr::from_bytes(b"\xe9t\xe9"))).expect("create file");
    latin1.write_all(&[0u8; 8192]).expect("write");
    let latin1_folder = dir.join(OsStr::from_bytes(b"d\xe9j\xe0"));
    fs::create_dir(&latin1_folder).expect("create folder");
    let mut inner = File::create(latin1_folder.join("inside")).expect("create file");
    inner.write_all(&[0u8; 16384]).expect("write");

    let sizes = (1..60u16)
        .step_by(4)
        .flat_map(|w| (1..20u16).step_by(3).map(move |h| (w, h)))
        .chain(
            (60..200u16)
                .step_by(13)
                .flat_map(|w| (20..60u16).step_by(11).map(move |h| (w, h))),
        );
    let mut panics = vec![];
    for (width, height) in sizes {
        let mut app = app_with_scanned_dir(&dir, width, height);
        // Inside the non-UTF-8 folder, for the title's path.
        if width % 2 == 0
            && app
                .file_tree
                .item_in_current_folder(latin1_folder.file_name().unwrap())
                .is_some()
        {
            app.file_tree
                .enter_folder(latin1_folder.file_name().unwrap());
            app.render_and_update_board();
        }
        let tiles = app.board.tiles.len();
        for selected in 0..tiles.max(1) {
            if tiles > 0 {
                app.board.set_selected_index(&selected);
            }
            let file = app.get_file_to_delete();
            let mut modes = vec![
                UiMode::Loading,
                UiMode::Normal,
                UiMode::ErrorMessage("é".repeat(300)),
                UiMode::Exiting { app_loaded: true },
                UiMode::Exiting { app_loaded: false },
            ];
            if let Some(file) = file {
                modes.push(UiMode::DeleteFile(file.clone()));
                modes.push(UiMode::WarningMessage(file));
            }
            for mode in modes {
                let label = format!("{width}x{height} tile {selected} {}", mode_name(&mode));
                app.ui_mode = mode;
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| app.render()));
                if result.is_err() {
                    panics.push(label);
                }
            }
        }
    }
    let _ = fs::remove_dir_all(&dir);
    assert!(
        panics.is_empty(),
        "{} panics, e.g. {:?}",
        panics.len(),
        &panics[..panics.len().min(10)]
    );
}

fn mode_name(mode: &UiMode) -> &'static str {
    match mode {
        UiMode::Loading => "Loading",
        UiMode::Normal => "Normal",
        UiMode::ScreenTooSmall => "ScreenTooSmall",
        UiMode::DeleteFile(_) => "DeleteFile",
        UiMode::ErrorMessage(_) => "ErrorMessage",
        UiMode::Exiting { .. } => "Exiting",
        UiMode::WarningMessage(_) => "WarningMessage",
    }
}

/// The legend and prompts name the configured keys: the legend used to say
/// `<BACKSPACE> - delete`, although delete has been `d` since 0.12.2.
#[test]
fn hints_show_the_configured_keys() {
    let dir = temp_app_dir("hints");
    File::create(dir.join("file"))
        .and_then(|mut f| f.write_all(&[0u8; 4096]))
        .expect("create file");
    let screen = |keybinds: Keybinds, mode: fn(&App<TestBackend>) -> UiMode| {
        let (tx, _rx) = mpsc::sync_channel(1);
        let mut app = App::new(TestBackend::new(160, 30), dir.clone(), tx, true, keybinds);
        for item in scan_folder(&dir, ScanOptions::default()) {
            if let ScanItem::Entry { metadata, path } = item {
                app.add_entry_to_base_folder(&metadata, path);
            }
        }
        app.start_ui();
        app.board.set_selected_index(&0);
        app.ui_mode = mode(&app);
        app.render();
        app.display.rows().join("\n")
    };
    let normal = |_: &App<TestBackend>| UiMode::Normal;
    let delete = |app: &App<TestBackend>| UiMode::DeleteFile(app.get_file_to_delete().unwrap());

    let default = screen(Keybinds::default(), normal);
    assert!(default.contains("<d> - delete"), "{default}");
    assert!(!default.contains("BACKSPACE"), "{default}");

    use crate::config::KeyBinding;
    let keybinds = Keybinds {
        delete: KeyBinding::char('x'),
        quit: KeyBinding::char('Q'),
        confirm: KeyBinding::char('Y'),
        ..Keybinds::default()
    };
    let custom = screen(keybinds.clone(), normal);
    assert!(
        custom.contains("<x> - delete") && custom.contains("<Q> - quit"),
        "{custom}"
    );
    let prompt = screen(keybinds, delete);
    assert!(prompt.contains("(Y/n)"), "{prompt}");
    let _ = fs::remove_dir_all(&dir);
}
