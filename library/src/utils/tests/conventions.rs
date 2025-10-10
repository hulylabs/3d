#[cfg(test)]
pub(crate) mod tests {
    use std::path::PathBuf;

    #[must_use]
    pub(crate) fn test_folder_path() -> PathBuf {
        PathBuf::from("tests")
    }

    #[must_use]
    pub(crate) fn out_folder_path() -> PathBuf {
        test_folder_path().join("out")
    }

    #[must_use]
    pub(crate) fn data_folder_path() -> PathBuf {
        test_folder_path().join("data")
    }
}