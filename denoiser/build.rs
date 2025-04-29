use std::env;
use build::copy_directory_content_to_output;

const LIBRARIES_WINDOWS_FOLDER: &str = "lib/windows";

const OUT_DIRECTORY_UP_LEVEL: usize = 3;

fn main() {
    println!("cargo:rustc-link-lib=dylib=OpenImageDenoise");
    println!("cargo:rustc-link-lib=dylib=OpenImageDenoise_core");
    
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if "windows" == target_os {
        println!("cargo:info=current OS is Windows: using oidn's DLLs as dylibs");
        
        let project_dir = env::current_dir().expect("failed to get current directory");
        let relative_path = project_dir.join(LIBRARIES_WINDOWS_FOLDER);
        println!("cargo:rustc-link-search=native={}", relative_path.display());
        
        copy_directory_content_to_output(LIBRARIES_WINDOWS_FOLDER, OUT_DIRECTORY_UP_LEVEL, "**/*.dll").unwrap();
    }
}
