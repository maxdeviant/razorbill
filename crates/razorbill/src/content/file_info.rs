use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub parent: PathBuf,
    pub components: Vec<String>,
}

impl FileInfo {
    pub fn new(root_path: impl AsRef<Path>, path: impl AsRef<Path>) -> Self {
        let root_path = root_path.as_ref();
        let path = path.as_ref();
        Self {
            path: path.to_owned(),
            parent: path.parent().unwrap_or(root_path).to_owned(),
            components: Self::components(root_path, path),
        }
    }

    fn components(root_path: impl AsRef<Path>, path: impl AsRef<Path>) -> Vec<String> {
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
