use build::copy_directory_to_output;

const ASSETS_FOLDER_NAME: &str = "assets";

const OUT_DIRECTORY_UP_LEVEL: usize = 3;

fn main() {
    copy_directory_to_output(ASSETS_FOLDER_NAME, OUT_DIRECTORY_UP_LEVEL).unwrap();
}
