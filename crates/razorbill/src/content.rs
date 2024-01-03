mod front_matter;

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

pub use front_matter::*;

#[derive(Debug)]
pub struct Page {
    pub meta: PageFrontMatter,
    pub slug: String,
    pub raw_content: String,
}

#[derive(Debug, Deserialize)]
pub struct PageFrontMatter {
    pub title: Option<String>,
    pub slug: Option<String>,
}

#[derive(Error, Debug)]
pub enum ParsePageError {
    #[error("failed to read page: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid front matter in '{filepath}'")]
    InvalidFrontMatter { filepath: PathBuf },
}

impl Page {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ParsePageError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)?;

        Self::parse(&contents, path)
    }

    pub fn parse(text: &str, filepath: &Path) -> Result<Self, ParsePageError> {
        let (front_matter, content) =
            parse_front_matter::<PageFrontMatter>(text).ok_or_else(|| {
                ParsePageError::InvalidFrontMatter {
                    filepath: filepath.to_owned(),
                }
            })?;

        let slug = front_matter
            .slug
            .clone()
            .unwrap_or_else(|| filepath.file_stem().unwrap().to_string_lossy().to_string());

        Ok(Self {
            meta: front_matter,
            slug,
            raw_content: content.to_string(),
        })
    }
}
