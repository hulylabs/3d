use std::{env, fs};
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use glob::Pattern;

fn main() {
    if env::var("CARGO_FEATURE_DENOISER").is_ok() {
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
        let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

        match (target_os.as_str(), target_arch.as_str()) {
            ("windows", _) => link_with_oidn_windows_dll(),
            ("macos", "aarch64") => link_with_oidn_macos_arm_dylib(),
            _ => {
                panic!("unsupported target OS for denoiser: {}, {}", target_os, target_arch);
            }
        }
    }
}

const OUT_DIRECTORY_UP_LEVEL: usize = 3;

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

fn cargo_info(text: &str) {
    println!("cargo:info={}", text);
}

fn link_with_oidn_macos_arm_dylib() {
    cargo_info("current OS is macOS on ARM (Apple Silicon)");
    link_with_oidn_library(libraries_macos_arm_folder(), "**/*.dylib");
}

fn link_with_oidn_windows_dll() {
    cargo_info("current OS is Windows: using oidn's DLLs");
    link_with_oidn_library(libraries_windows_folder(), "**/*.dll");
}

fn link_with_oidn_library(libraries_local_path: impl AsRef<Path>, dylib_filter: &str) {
    cargo_emit::rustc_link_lib!(OPEN_IMAGE_DENOISE_LIBRARY_NAME);
    
    let project_directory = env::current_dir().expect("failed to get current directory");
    let compiler_libraries_search_path = project_directory.join(libraries_local_path.as_ref());

    cargo_emit::rustc_link_search!(compiler_libraries_search_path.to_str().expect("project lib path is not valid UTF-8") => "native");
    
    copy_directory_content_to_output(libraries_local_path, OUT_DIRECTORY_UP_LEVEL, dylib_filter).unwrap();
}

pub fn copy_directory_content_to_output(local_path: impl AsRef<Path>, out_directory_up_level: usize, filter: &str) -> std::io::Result<()> {
    let out_directory = env::var("OUT_DIR")
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let target_directory = PathBuf::from(out_directory)
        .ancestors()
        .nth(out_directory_up_level)
        .unwrap()
        .to_path_buf();

    cargo_info(format!("destination = {:?}", target_directory).as_str());

    {let absolute_path = fs::canonicalize(local_path.as_ref())?;
        cargo_info(format!("source = {:?}", absolute_path).as_str());}

    copy_directory(local_path, &target_directory, &Some(PathPattern::new(filter)))?;

    Ok(())
}

pub fn copy_directory_to_output(local_path: &str, out_directory_up_level: usize) -> std::io::Result<()> {
    let out_directory = env::var("OUT_DIR")
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let target_directory = PathBuf::from(out_directory)
        .ancestors()
        .nth(out_directory_up_level)
        .unwrap()
        .to_path_buf();

    let destination = target_directory.join(local_path);

    if destination.exists() {
        fs::remove_dir_all(&destination)?;
    }
    cargo_info(format!("destination {} = {:?}", local_path, destination).as_str());

    {let absolute_path = fs::canonicalize(Path::new(local_path))?;
        cargo_info(format!("source {} = {:?}", local_path, absolute_path).as_str());}

    copy_directory(local_path, &destination, &None)?;

    Ok(())
}

fn copy_directory(source: impl AsRef<Path>, destination: impl AsRef<Path>, file_pattern: &Option<PathPattern>) -> std::io::Result<()> {
    fs::create_dir_all(&destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let destination_path = destination.as_ref().join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(entry.path(), &destination_path, file_pattern)?;
        } else if file_pattern.as_ref().is_none_or(|pattern| pattern.matches(&entry)) {
            fs::copy(entry.path(), destination_path)?;
        }
    }
    Ok(())
}

struct PathPattern {
    pattern: Pattern,
}

impl PathPattern {
    #[must_use]
    fn new(pattern: &str) -> Self {
        Self { pattern: Pattern::new(pattern).unwrap(), }
    }

    #[must_use]
    fn matches(&self, entry: &DirEntry) -> bool {
        let unix_style_path = entry.path().to_string_lossy().replace('\\', "/");
        self.pattern.matches(unix_style_path.as_str())
    }
}