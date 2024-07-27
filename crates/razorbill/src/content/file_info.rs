use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_file_info() {
        let file = FileInfo::new("content", "content/_index.md");
        assert_eq!(
            file,
            FileInfo {
                path: PathBuf::from("content/_index.md"),
                parent: PathBuf::from("content"),
                components: vec![]
            }
        );

        let file = FileInfo::new("content", "content/a/b/c/d/_index.md");
        assert_eq!(
            file,
            FileInfo {
                path: PathBuf::from("content/a/b/c/d/_index.md"),
                parent: PathBuf::from("content/a/b/c/d"),
                components: vec!["a".into(), "b".into(), "c".into(), "d".into()]
            }
        );

        let file = FileInfo::new("some/other/path", "some/other/path/blog/hello-world.md");
        assert_eq!(
            file,
            FileInfo {
                path: PathBuf::from("some/other/path/blog/hello-world.md"),
                parent: PathBuf::from("some/other/path/blog"),
                components: vec!["blog".into()]
            }
        );
    }
}
