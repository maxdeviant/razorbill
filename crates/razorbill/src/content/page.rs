use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fmt, fs};

use auk::Element;
use serde::Deserialize;
use thiserror::Error;

use crate::content::{
    from_toml_datetime, parse_front_matter, FileInfo, ReadTime, ReadingMetrics, WordCount,
};
use crate::permalink::Permalink;
use crate::SiteConfig;

#[derive(Debug)]
pub struct Page {
    pub meta: PageFrontMatter,
    pub file: FileInfo,
    pub path: PagePath,
    pub permalink: Permalink,
    pub ancestors: Vec<PathBuf>,
    pub slug: String,
    pub raw_content: String,
    pub content: Vec<Element>,
    pub word_count: WordCount,
    pub read_time: ReadTime,
}

#[derive(Debug)]
pub struct PagePath(pub(crate) String);

impl fmt::Display for PagePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PagePath {
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

#[derive(Debug, Default, Deserialize)]
pub struct PageFrontMatter {
    pub title: Option<String>,
    pub slug: Option<String>,
    #[serde(default, deserialize_with = "from_toml_datetime")]
    pub date: Option<String>,
    #[serde(default, deserialize_with = "from_toml_datetime")]
    pub updated: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub template: Option<String>,
    #[serde(default)]
    pub taxonomies: HashMap<String, Vec<String>>,

    #[serde(default)]
    pub extra: toml::Table,
}

#[derive(Error, Debug)]
pub enum ParsePageError {
    #[error("failed to read page '{filepath}': {err}")]
    Io {
        err: std::io::Error,
        filepath: PathBuf,
    },

    #[error("invalid front matter in '{filepath}'")]
    InvalidFrontMatter { filepath: PathBuf },
}

impl Page {
    pub fn from_path(
        config: &SiteConfig,
        root_path: impl AsRef<Path>,
        path: impl AsRef<Path>,
    ) -> Result<Self, ParsePageError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|err| ParsePageError::Io {
            err,
            filepath: path.to_owned(),
        })?;

        Self::parse(config, &contents, root_path, path)
    }

    pub fn parse(
        config: &SiteConfig,
        text: &str,
        root_path: impl AsRef<Path>,
        filepath: &Path,
    ) -> Result<Self, ParsePageError> {
        let root_path = root_path.as_ref();
        let (front_matter, content) =
            parse_front_matter::<PageFrontMatter>(text).ok_or_else(|| {
                ParsePageError::InvalidFrontMatter {
                    filepath: filepath.to_owned(),
                }
            })?;

        let file = FileInfo::new(root_path, filepath);
        let slug = front_matter
            .slug
            .clone()
            .unwrap_or_else(|| filepath.file_stem().unwrap().to_string_lossy().to_string());

        let path = PagePath::from_file_path(root_path, &file.path).unwrap();

        let reading_metrics = ReadingMetrics::for_content(&content, config.reading_speed);

        Ok(Self {
            meta: front_matter,
            file,
            permalink: Permalink::from_path(config, path.0.as_str()),
            path,
            ancestors: Vec::new(),
            slug,
            raw_content: content.to_string(),
            content: Vec::new(),
            word_count: reading_metrics.word_count,
            read_time: reading_metrics.read_time,
        })
    }
}
