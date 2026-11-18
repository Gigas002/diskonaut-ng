use ::std::ffi::OsString;
use ::std::fs;
use ::std::io::Write;
use ::std::path::PathBuf;

use crate::model::{FileOrFolder, FileTree, Folder};

#[test]
fn folder_add_file_updates_size() {
    let mut folder = Folder::from(OsString::from("root"));
    folder.add_file(PathBuf::from("a.txt"), 100);
    folder.add_file(PathBuf::from("b.txt"), 250);
    assert_eq!(folder.size, 350);
    assert_eq!(folder.num_descendants, 2);
}

#[test]
fn folder_nested_path() {
    let mut folder = Folder::from(OsString::from("root"));
    folder.add_file(PathBuf::from("sub/file.txt"), 42);
    assert_eq!(folder.size, 42);
    let sub = folder.path(vec!["sub".into()]).expect("subfolder exists");
    match sub {
        FileOrFolder::Folder(subfolder) => {
            assert_eq!(subfolder.size, 42);
        }
        FileOrFolder::File(_) => panic!("expected folder"),
    }
}

#[test]
fn file_tree_delete_path() {
    let dir = tempfile::Builder::new()
        .prefix("diskonaut_model_test_delete")
        .tempdir()
        .expect("mkdir")
        .keep();
    let file_path = dir.join("gone.txt");
    let mut f = fs::File::create(&file_path).expect("create");
    f.write_all(b"x").expect("write");

    let metadata = fs::metadata(&file_path).expect("metadata");
    let mut tree = FileTree::new(Folder::new(&dir), dir.clone(), true);
    tree.add_entry(&metadata, &file_path);
    assert_eq!(tree.get_total_descendants(), 1);

    let to_delete = crate::FileToDelete {
        path_in_filesystem: dir.clone(),
        path_to_file: vec!["gone.txt".into()],
        file_type: crate::tiles::FileType::File,
        num_descendants: None,
        size: metadata.len().into(),
    };
    tree.delete_file(&to_delete);
    assert_eq!(tree.get_total_descendants(), 0);

    let _ = fs::remove_dir_all(&dir);
}

/// The parallel walk can report a folder after entries inside it. Adding the
/// folder then must not replace it with an empty one: 744 folders of a scan
/// of ~/.cargo/registry showed fewer files than their size and counts said.
#[test]
fn a_folder_reported_after_its_contents_keeps_them() {
    let mut root = Folder::from(OsString::from("root"));
    root.add_file(PathBuf::from("sub/a.txt"), 10);
    root.add_folder(PathBuf::from("sub/deeper"));
    root.add_file(PathBuf::from("sub/deeper/b.txt"), 5);
    root.add_folder(PathBuf::from("sub"));

    let Some(FileOrFolder::Folder(sub)) = root.path(vec!["sub".into()]) else {
        panic!("sub is a folder");
    };
    assert_eq!(sub.contents.len(), 2);
    assert_eq!(sub.size, 15);
    let Some(FileOrFolder::Folder(deeper)) = sub.path(vec!["deeper".into()]) else {
        panic!("deeper is a folder");
    };
    assert_eq!(deeper.contents.len(), 1);
    // root: sub, sub/a.txt, sub/deeper, sub/deeper/b.txt; each counted once.
    assert_eq!(root.num_descendants, 4);
    assert_eq!(sub.num_descendants, 3);
}
