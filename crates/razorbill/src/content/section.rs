use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

use serde::Deserialize;
use thiserror::Error;

use crate::content::{
    parse_front_matter, FileInfo, MaybeSortBy, ReadTime, ReadingMetrics, WordCount,
    AVERAGE_ADULT_WPM,
};
use crate::permalink::Permalink;
use crate::SiteConfig;

#[derive(Debug)]
pub struct Section {
    pub meta: SectionFrontMatter,
    pub file: FileInfo,
    pub path: SectionPath,
    pub permalink: Permalink,
    pub raw_content: String,
    pub word_count: WordCount,
    pub read_time: ReadTime,
    pub pages: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, Default, Deserialize)]
pub struct SectionFrontMatter {
    pub title: Option<String>,
    pub template: Option<String>,
    pub page_template: Option<String>,

    #[serde(default)]
    pub sort_by: MaybeSortBy,

    #[serde(default)]
    pub transparent: bool,

    #[serde(default)]
    pub extra: toml::Table,
}

#[derive(Error, Debug)]
pub enum ParseSectionError {
    #[error("failed to read section '{index_path}': {err}")]
    Io {
        err: std::io::Error,
        index_path: PathBuf,
    },

    #[error("invalid front matter in '{filepath}'")]
    InvalidFrontMatter { filepath: PathBuf },
}

impl Section {
    pub fn from_path(
        config: &SiteConfig,
        root_path: impl AsRef<Path>,
        path: impl AsRef<Path>,
    ) -> Result<Option<Self>, ParseSectionError> {
        let path = path.as_ref();
        let index_path = path.join("_index.md");
        let contents = match fs::read_to_string(&index_path) {
            Ok(contents) => Ok(contents),
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    return Ok(None);
                }

                Err(ParseSectionError::Io {
                    err,
                    index_path: index_path.clone(),
                })
            }
        }?;

        Self::parse(config, &contents, root_path, &index_path).map(Some)
    }

    pub fn parse(
        config: &SiteConfig,
        text: &str,
        root_path: impl AsRef<Path>,
        filepath: &Path,
    ) -> Result<Self, ParseSectionError> {
        let root_path = root_path.as_ref();
        let (front_matter, content) =
            parse_front_matter::<SectionFrontMatter>(text).ok_or_else(|| {
                ParseSectionError::InvalidFrontMatter {
                    filepath: filepath.to_owned(),
                }
            })?;

        let file = FileInfo::new(root_path, filepath);
        let path = SectionPath::from_file_path(root_path, &file.path).unwrap();

        let reading_metrics = ReadingMetrics::for_content(&content, AVERAGE_ADULT_WPM);

        Ok(Self {
            meta: front_matter,
            file,
            permalink: Permalink::from_path(config, path.0.as_str()),
            path,
            raw_content: content.to_string(),
            word_count: reading_metrics.word_count,
            read_time: reading_metrics.read_time,
            pages: Vec::new(),
        })
    }
}
