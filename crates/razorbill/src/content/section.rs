use std::path::{Path, PathBuf};
use std::{fmt, fs};

use serde::Deserialize;
use thiserror::Error;

use crate::content::{parse_front_matter, FileInfo};

#[derive(Debug)]
pub struct SectionPath(pub(crate) String);

impl fmt::Display for SectionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SectionPath {
    pub fn from_file_path(
        root_path: impl AsRef<Path>,
        file_path: impl AsRef<Path>,
    ) -> Result<Self, ()> {
        let file_path = file_path.as_ref().strip_prefix(root_path).unwrap();

        let parent = file_path.parent().unwrap().to_str().unwrap();
        let slug = file_path.file_stem().unwrap().to_str().unwrap();

        if parent.is_empty() {
            Ok(Self(format!("/{slug}")))
        } else {
            Ok(Self(format!("/{parent}/{slug}")))
        }
    }
}

#[derive(Debug)]
pub struct Section {
    pub meta: SectionFrontMatter,
    pub file: FileInfo,
    pub path: SectionPath,
    pub raw_content: String,
    pub pages: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
pub struct SectionFrontMatter {
    pub title: Option<String>,
    pub template: Option<String>,
}

#[derive(Error, Debug)]
pub enum ParseSectionError {
    #[error("failed to read section: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid front matter in '{filepath}'")]
    InvalidFrontMatter { filepath: PathBuf },
}

impl Section {
    pub fn from_path(
        root_path: impl AsRef<Path>,
        path: impl AsRef<Path>,
    ) -> Result<Self, ParseSectionError> {
        let path = path.as_ref();
        let index_path = path.join("_index.md");
        let contents = fs::read_to_string(&index_path)?;

        Self::parse(&contents, root_path, &index_path)
    }

    pub fn parse(
        text: &str,
        root_path: impl AsRef<Path>,
        filepath: &Path,
    ) -> Result<Self, ParseSectionError> {
        let (front_matter, content) =
            parse_front_matter::<SectionFrontMatter>(text).ok_or_else(|| {
                ParseSectionError::InvalidFrontMatter {
                    filepath: filepath.to_owned(),
                }
            })?;

        let file = FileInfo {
            path: filepath.to_owned(),
            parent: filepath.parent().unwrap_or(root_path.as_ref()).to_owned(),
        };

        let path = SectionPath::from_file_path(root_path, &file.path).unwrap();

        Ok(Self {
            meta: front_matter,
            file,
            path,
            raw_content: content.to_string(),
            pages: Vec::new(),
        })
    }
}
