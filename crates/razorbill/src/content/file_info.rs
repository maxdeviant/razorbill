use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub parent: PathBuf,
    pub components: Vec<String>,
}

impl FileInfo {
    pub fn components(root_path: impl AsRef<Path>, path: impl AsRef<Path>) -> Vec<String> {
        let path = path.as_ref();
        path.strip_prefix(root_path)
            .unwrap_or(path)
            .parent()
            .unwrap()
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .collect()
    }
}
