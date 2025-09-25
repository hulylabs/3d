#[cfg(test)]
pub(crate) mod tests {
    use std::{env, fs};
    use std::path::{Path, PathBuf};
    use crate::utils::file_system::ensure_folders_exist;

    #[must_use]
    fn do_we_have_cli_flag_on(flag: &str) -> bool {
        let arguments: Vec<String> = env::args().collect();
        let flag_variants = [
            flag,
            &format!("--{}", flag),
            &format!("-{}", flag),
        ];

        arguments.iter().any(|argument| flag_variants.contains(&argument.as_str()))
    }

    #[must_use]
    pub(crate) fn make_new_reference_mode() -> bool {
        do_we_have_cli_flag_on("make_new_reference")
    }

    pub(crate) fn copy_to_reference<FilePath: AsRef<Path>>(
        source_path: FilePath,
        destination_path: FilePath,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ensure_folders_exist(&destination_path)?;
        fs::copy(&source_path, &destination_path)?;
        Ok(())
    }

    #[must_use]
    pub(crate) fn add_suffix_to_filename(path: &PathBuf, suffix: &str) -> PathBuf {
        let mut new_path = path.clone();

        if let Some(stem) = path.file_stem() {
            let new_filename = if let Some(ext) = path.extension() {
                format!("{}{}.{}", stem.to_string_lossy(), suffix, ext.to_string_lossy())
            } else {
                format!("{}{}", stem.to_string_lossy(), suffix)
            };
            new_path.set_file_name(new_filename);
        }

        new_path
    }

    #[test]
    fn test_add_suffix_to_filename() {
        let actual_path = add_suffix_to_filename(&PathBuf::from("test.png"), "_test");
        assert_eq!(actual_path.to_string_lossy(), "test_test.png");

        let actual_path = add_suffix_to_filename(&PathBuf::from("foo").join("test.png"), "_test");
        assert_eq!(actual_path.to_string_lossy(), PathBuf::from("foo").join("test_test.png").to_string_lossy());
    }
}