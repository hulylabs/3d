use std::fs;
use std::path::Path;

pub fn ensure_folders_exist(file_path: & impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent_dir) = file_path.as_ref().parent() {
        fs::create_dir_all(parent_dir)?;
    }
    Ok(())
}