use ::std::collections::HashSet;
use ::std::ffi::{OsStr, OsString};
use ::std::fs::Metadata;
use ::std::path::{Path, PathBuf};

use crate::model::{FileOrFolder, FileToDelete, Folder};

pub struct FileTree {
    pub current_folder_names: Vec<OsString>,
    pub space_freed: u128,
    pub failed_to_read: u64,
    pub path_in_filesystem: PathBuf,
    base_folder: Folder,
    show_apparent_size: bool,
    /// (device, inode) of files with more than one link already counted.
    seen_hard_links: HashSet<(u64, u64)>,
}

impl FileTree {
    pub fn new(base_folder: Folder, path_in_filesystem: PathBuf, show_apparent_size: bool) -> Self {
        FileTree {
            base_folder,
            current_folder_names: Vec::new(),
            path_in_filesystem,
            space_freed: 0,
            failed_to_read: 0,
            show_apparent_size,
            seen_hard_links: HashSet::new(),
        }
    }
    pub fn get_total_size(&self) -> u128 {
        self.base_folder.size
    }
    pub fn get_total_descendants(&self) -> u64 {
        self.base_folder.num_descendants
    }
    pub fn get_current_folder(&self) -> &Folder {
        if self.current_folder_names.is_empty() {
            &self.base_folder
        } else if let Some(FileOrFolder::Folder(current_folder)) =
            self.base_folder.path(self.current_folder_names.clone())
        {
            current_folder
        } else {
            // here we have something in current_folder_names but the last
            // one is somehow not a folder... this is a corrupted state
            unreachable!("couldn't find current folder size")
        }
    }
    pub fn get_current_folder_size(&self) -> u128 {
        self.get_current_folder().size
    }
    pub fn get_current_path(&self) -> PathBuf {
        let mut full_path = PathBuf::from(&self.path_in_filesystem);
        for folder in &self.current_folder_names {
            full_path.push(folder)
        }
        full_path
    }
    pub fn item_in_current_folder(&self, item_name: &OsStr) -> Option<&FileOrFolder> {
        let current_folder = &self.get_current_folder();
        current_folder.path(vec![item_name.to_os_string()])
    }
    pub fn enter_folder(&mut self, folder_name: &OsStr) {
        self.current_folder_names.push(folder_name.to_os_string());
    }
    pub fn leave_folder(&mut self) -> bool {
        // true => succeeded, false => at base folder
        self.current_folder_names.pop().is_some()
    }
    pub fn delete_file(&mut self, file_to_delete: &FileToDelete) {
        let path_to_delete = &file_to_delete.path_to_file;
        self.base_folder.delete_path(path_to_delete);
    }
    pub fn add_entry(&mut self, entry_metadata: &Metadata, entry_full_path: &Path) {
        let base_path_length = self.path_in_filesystem.components().count();
        let mut relative_path = PathBuf::new();
        for dir in entry_full_path.components().skip(base_path_length) {
            relative_path.push(dir);
        }
        // Like du, a file with several hard links takes space once: the first
        // link seen carries its size, the others are listed with none.
        let counted_before = !entry_metadata.is_dir()
            && hard_link_id(entry_metadata).is_some_and(|id| !self.seen_hard_links.insert(id));
        if counted_before {
            self.base_folder.add_file(relative_path, 0);
        } else {
            self.base_folder
                .add_entry(entry_metadata, relative_path, self.show_apparent_size);
        }
    }
}

#[cfg(unix)]
fn hard_link_id(metadata: &Metadata) -> Option<(u64, u64)> {
    use ::std::os::unix::fs::MetadataExt;
    (metadata.nlink() > 1).then(|| (metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn hard_link_id(_metadata: &Metadata) -> Option<(u64, u64)> {
    None
}
