use std::env;
use std::path::{Path, PathBuf};
use fs_extra::dir::{copy, CopyOptions};

const ASSETS_FOLDER_NAME: &str = "assets";

const OUT_DIRECTORY_UP_LEVEL: usize = 3;

fn main() {
    let copy_source = Path::new(ASSETS_FOLDER_NAME);

    let out_directory = env::var("OUT_DIR")
        .map_err(|e| std::io::Error::other(e))
        .expect("failed to retrieve output directory of the build procedure");

    let copy_target = PathBuf::from(out_directory)
        .ancestors()
        .nth(OUT_DIRECTORY_UP_LEVEL)
        .unwrap()
        .to_path_buf();

    let mut options = CopyOptions::new();
    options.overwrite = true;
    copy(copy_source, copy_target.clone(), &options)
        .unwrap_or_else(|_| panic!("failed to copy folder {copy_source:?} into {copy_target:?}"));
}
