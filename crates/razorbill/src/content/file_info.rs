use std::path::PathBuf;

#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub parent: PathBuf,
}
