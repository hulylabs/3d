use std::{env, fs};
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

use glob::Pattern;

pub fn copy_directory_content_to_output(local_path: &str, out_directory_up_level: usize, filter: &str) -> std::io::Result<()> {
    let out_directory = env::var("OUT_DIR")
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let target_directory = PathBuf::from(out_directory)
        .ancestors()
        .nth(out_directory_up_level)
        .unwrap()
        .to_path_buf();
    
    println!("cargo:info=destination = {:?}", target_directory);

    {let absolute_path = fs::canonicalize(Path::new(local_path))?;
        println!("cargo:info=source = {:?}", absolute_path);}

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
    println!("cargo:info=destination {} = {:?}", local_path, destination);

    {let absolute_path = fs::canonicalize(Path::new(local_path))?;
        println!("cargo:info=source {} = {:?}", local_path, absolute_path);}

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
        } else {
            if file_pattern.as_ref().map_or(true, |pattern| pattern.matches(&entry)) {
                fs::copy(entry.path(), destination_path)?;
            }
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