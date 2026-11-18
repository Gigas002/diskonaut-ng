use super::*;

#[test]
fn default_config_has_expected_keybinds() {
    let kb = DiskonautConfig::default().keybinds().unwrap();
    assert_eq!(kb.delete, KeyBinding::char('d'));
    assert_eq!(kb.quit, KeyBinding::char('q'));
}

#[test]
fn parses_example_config() {
    let toml = r#"
version = 1

[base]
apparent-size = false

[keybinds]
delete = "x"
move-left = "left"
"#;
    let cfg: DiskonautConfig = toml::from_str(toml).unwrap();
    assert_eq!(cfg.version, CONFIG_VERSION);
    assert!(!cfg.base.apparent_size);
    let kb = cfg.keybinds().unwrap();
    assert_eq!(kb.delete, KeyBinding::char('x'));
    assert_eq!(
        kb.move_left,
        KeyBinding::key(ratatui::crossterm::event::KeyCode::Left)
    );
    assert_eq!(kb.move_right, KeyBinding::char('l'));
}

#[test]
fn rejects_unsupported_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "version = 2\n").unwrap();
    let err = DiskonautConfig::load(Some(&path)).unwrap_err();
    assert!(matches!(err, ConfigError::UnsupportedVersion(2)));
}

#[test]
fn invalid_keybind_returns_error() {
    let cfg: DiskonautConfig = toml::from_str(
        r#"
version = 1

[keybinds]
delete = "not-a-real-key-name-here"
"#,
    )
    .unwrap();
    let err = cfg.keybinds().unwrap_err();
    assert!(matches!(err, ConfigError::InvalidKeybind { .. }));
}

fn key(code: ratatui::crossterm::event::KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

/// Terminals report an uppercase letter or a shifted symbol with SHIFT set
/// (`D` is Char('D') + SHIFT): a binding to "D" or "?" never fired.
#[test]
fn shifted_characters_match_their_bindings() {
    let cfg: DiskonautConfig =
        toml::from_str("version = 1\n[keybinds]\ndelete = \"D\"\nquit = \"?\"\n").unwrap();
    let kb = cfg.keybinds().unwrap();
    assert!(
        kb.delete
            .matches_event(&key(KeyCode::Char('D'), KeyModifiers::SHIFT))
    );
    assert!(
        kb.delete
            .matches_event(&key(KeyCode::Char('D'), KeyModifiers::NONE))
    );
    assert!(
        !kb.delete
            .matches_event(&key(KeyCode::Char('d'), KeyModifiers::NONE))
    );
    assert!(kb.is_quit(&key(KeyCode::Char('?'), KeyModifiers::SHIFT)));
    // Ctrl still has to match.
    assert!(!kb.delete.matches_event(&key(
        KeyCode::Char('D'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT
    )));
    // And a non-character key with Shift is a different chord.
    let enter = KeyBinding::key(KeyCode::Enter);
    assert!(!enter.matches_event(&key(KeyCode::Enter, KeyModifiers::SHIFT)));
}

/// "Ctrl+C" and "ENTER" are the same keys as "ctrl+c" and "enter".
#[test]
fn key_names_ignore_case() {
    assert_eq!(
        KeyBinding::parse("Ctrl+c").unwrap(),
        KeyBinding::ctrl_char('c')
    );
    assert_eq!(
        KeyBinding::parse("CTRL+x").unwrap(),
        KeyBinding::ctrl_char('x')
    );
    assert_eq!(KeyBinding::parse("Shift+a").unwrap(), KeyBinding::char('A'));
    assert_eq!(
        KeyBinding::parse("ctrl+C").unwrap(),
        KeyBinding::ctrl_char('c')
    );
}

/// A misspelled key (`apparent_size` for `apparent-size`) used to do
/// nothing, silently.
#[test]
fn unknown_keys_are_rejected() {
    for toml in [
        "version = 1\n[base]\napparent_size = true\n",
        "version = 1\n[keybinds]\ndelete-file = \"x\"\n",
        "version = 1\n[keybind]\ndelete = \"x\"\n",
    ] {
        let err = toml::from_str::<DiskonautConfig>(toml).unwrap_err();
        assert!(err.to_string().contains("unknown field"), "{err}");
    }
}

/// The example config documents every key and stays loadable.
#[test]
fn the_example_config_loads() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../example/config.toml");
    let cfg = DiskonautConfig::load(Some(&path)).unwrap();
    assert_eq!(cfg.keybinds().unwrap(), Keybinds::default());
}
