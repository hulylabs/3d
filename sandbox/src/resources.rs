use std::env;
use std::path::{Path, PathBuf};

pub(super) const CONTENT_ROOT_FOLDER_NAME: &str = "assets";

#[must_use]
pub(super) fn get_resource_path(file_name: impl AsRef<Path>) -> PathBuf {
    let exe_path = env::current_exe().unwrap();
    let exe_directory = exe_path.parent().unwrap();
    exe_directory.join(file_name)
}

#[must_use]
pub(super) fn path_to_bitmap(file_name: &str) -> PathBuf {
    let image_path = Path::new(CONTENT_ROOT_FOLDER_NAME).join(file_name);
    get_resource_path(&image_path)
}
