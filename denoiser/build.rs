use std::env;
use std::path::{Path, PathBuf};
use build::copy_directory_content_to_output;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    match (target_os.as_str(), target_arch.as_str()) {
        ("windows", _) => link_with_windows_dll(),
        ("macos", "aarch64") => link_with_macos_arm_dylib(),
        _ => {
            panic!("unsupported target OS: {}, {}", target_os, target_arch);
        }
    }
}

const OUT_DIRECTORY_UP_LEVEL: usize = 3;

const CARGO_DYLIB_SEARCH_PATH: &str = "cargo:rustc-link-search=native=";
const CARGO_LINK_WITH_LIBRARY: &str = "cargo:rustc-link-lib=dylib=";

const OPEN_IMAGE_DENOISE_LIBRARY_NAME: &str = "OpenImageDenoise";

const LIBRARIES_FOLDER_NAME: &str = "lib";

#[must_use]
fn libraries_windows_folder() -> PathBuf {
    Path::new(LIBRARIES_FOLDER_NAME).join("windows")
}

#[must_use]
fn libraries_macos_arm_folder() -> PathBuf {
    Path::new(LIBRARIES_FOLDER_NAME).join("mac_arm")
}

fn cargo_command(command: &str, argument: &str) {
    println!("{}{}", command, argument);
}

fn link_with_macos_arm_dylib() {
    println!("cargo:info=current OS is macOS on ARM (Apple Silicon)");
    link_with_oidn_library(&libraries_macos_arm_folder(), "**/*.dylib");
}

fn link_with_windows_dll() {
    println!("cargo:info=current OS is Windows: using oidn's DLLs");
    link_with_oidn_library(&libraries_windows_folder(), "**/*.dll");
}

fn link_with_oidn_library(libraries_local_path: &PathBuf, dylib_filter: &str) {
    cargo_command(CARGO_LINK_WITH_LIBRARY, OPEN_IMAGE_DENOISE_LIBRARY_NAME);

    let project_directory = env::current_dir().expect("failed to get current directory");
    let compiler_libraries_search_path = project_directory.join(libraries_local_path.clone());
    cargo_command(CARGO_DYLIB_SEARCH_PATH, compiler_libraries_search_path.to_str().expect("project lib path is not valid UTF-8"));

    let libraries_folder = libraries_local_path.to_str().expect("windows lib path is not valid UTF-8");
    copy_directory_content_to_output(libraries_folder, OUT_DIRECTORY_UP_LEVEL, dylib_filter).unwrap();
}
