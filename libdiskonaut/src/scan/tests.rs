use ::std::fs::File;
use ::std::io::Write;
use ::std::path::PathBuf;

use super::{ScanItem, ScanOptions, scan_folder, scan_into_tree};

fn temp_scan_dir(name: &str) -> PathBuf {
    tempfile::Builder::new()
        .prefix(&format!("diskonaut_scan_test_{name}"))
        .tempdir()
        .expect("create temp dir")
        .keep()
}

#[test]
fn scan_folder_finds_files() {
    let dir = temp_scan_dir("finds_files");
    let file_path = dir.join("a.txt");
    let mut file = File::create(&file_path).expect("create file");
    file.write_all(b"hello").expect("write file");

    let options = ScanOptions {
        parallel: false,
        show_apparent_size: false,
    };
    let entries: Vec<_> = scan_folder(&dir, options)
        .filter_map(|item| match item {
            ScanItem::Entry { path, .. } => Some(path),
            ScanItem::ReadError => None,
        })
        .collect();

    assert!(
        entries.iter().any(|p| p == &file_path),
        "expected walk to include {file_path:?}, got {entries:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scan_into_tree_aggregates_sizes() {
    let dir = temp_scan_dir("aggregates");
    let file_path = dir.join("data.bin");
    let mut file = File::create(&file_path).expect("create file");
    file.write_all(&[0u8; 1024]).expect("write file");

    let options = ScanOptions {
        parallel: false,
        show_apparent_size: true,
    };
    let (tree, failed) = scan_into_tree(&dir, options);

    assert_eq!(failed, 0);
    assert!(tree.get_total_size() >= 1024);

    let _ = std::fs::remove_dir_all(&dir);
}

/// Hard links share one copy of the data: like `du`, the tree counts it
/// once. Counting every link made a folder of three links to a 64K file
/// show 192K, and deleting one link "freed" space that wasn't freed.
#[test]
fn hard_links_are_counted_once() {
    let dir = temp_scan_dir("hard_links");
    let original = dir.join("a");
    File::create(&original)
        .and_then(|mut f| f.write_all(&[1u8; 64 * 1024]))
        .expect("write file");
    std::fs::create_dir(dir.join("sub")).expect("create sub");
    std::fs::hard_link(&original, dir.join("b")).expect("link b");
    std::fs::hard_link(&original, dir.join("sub").join("c")).expect("link c");

    for show_apparent_size in [true, false] {
        let options = ScanOptions {
            parallel: true,
            show_apparent_size,
        };
        let (tree, failed) = scan_into_tree(&dir, options);
        assert_eq!(failed, 0);
        let one_copy = 64 * 1024;
        assert!(
            tree.get_total_size() >= one_copy && tree.get_total_size() < 2 * one_copy,
            "apparent={show_apparent_size}: {}",
            tree.get_total_size()
        );
        // All three names are still listed.
        assert_eq!(tree.get_total_descendants(), 4);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
