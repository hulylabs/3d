use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ASSETS_FOLDER_NAME: &str = "assets";

const OUT_DIRECTORY_DEPTH: usize = 3;

fn main() {
    copy_assets_folder_to_output().unwrap();
}

fn copy_assets_folder_to_output() -> std::io::Result<()> {
    let out_directory = env::var("OUT_DIR")
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    let target_directory = PathBuf::from(out_directory)
        .ancestors()
        .nth(OUT_DIRECTORY_DEPTH)
        .unwrap()
        .to_path_buf();
    
    let destination_assets = target_directory.join(ASSETS_FOLDER_NAME);

    if destination_assets.exists() {
        fs::remove_dir_all(&destination_assets)?;
    }
    println!("cargo:info=destination assets = {:?}", destination_assets);

    {let absolute_path = fs::canonicalize(Path::new(ASSETS_FOLDER_NAME))?;
    println!("cargo:info=source assets = {:?}", absolute_path);}
    
    copy_directory(ASSETS_FOLDER_NAME, &destination_assets)?;

    Ok(())
}

fn copy_directory(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let destination_path = destination.as_ref().join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(entry.path(), &destination_path)?;
        } else {
            fs::copy(entry.path(), destination_path)?;
        }
    }
    Ok(())
}