use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use thiserror::Error;

use crate::content::{Page, Section};
use crate::permalink::Permalink;

pub trait Store {
    type Error: std::error::Error;

    fn store_rendered_section(
        &self,
        section: &Section,
        rendered_html: String,
    ) -> Result<(), Self::Error> {
        self.store_content(section.permalink.clone(), rendered_html.clone())
    }

    fn store_rendered_page(&self, page: &Page, rendered_html: String) -> Result<(), Self::Error> {
        self.store_content(page.permalink.clone(), rendered_html.clone())
    }

    fn store_content(&self, permalink: Permalink, content: String) -> Result<(), Self::Error>;

    fn store_static_file(&self, path: &Path, content: String) -> Result<(), Self::Error>;
}

pub struct DiskStorage {
    output_path: PathBuf,
}

impl DiskStorage {
    pub fn new(output_path: PathBuf) -> Self {
        Self { output_path }
    }
}

impl Store for DiskStorage {
    type Error = io::Error;

    fn store_content(&self, permalink: Permalink, content: String) -> Result<(), Self::Error> {
        let output_dir = self
            .output_path
            .join(PathBuf::from_str(permalink.path().trim_start_matches("/")).unwrap());

        fs::create_dir_all(&output_dir)?;

        let output_path = if permalink.path().ends_with('/') {
            output_dir.join("index.html")
        } else {
            output_dir
        };

        let mut output_file = File::create(&output_path)?;
        output_file.write_all(content.as_bytes())?;

        Ok(())
    }

    fn store_static_file(&self, path: &Path, content: String) -> Result<(), Self::Error> {
        let mut output_dir = self.output_path.to_owned();

        if let Some(parent) = path.parent() {
            output_dir.push(parent);
        }

        fs::create_dir_all(&output_dir)?;

        let output_path = output_dir.join(path);
        let mut output_file = File::create(&output_path)?;

        output_file.write_all(content.as_bytes())?;

        Ok(())
    }
}

pub struct InMemoryStorage {
    storage: Arc<RwLock<HashMap<String, String>>>,
}

impl InMemoryStorage {
    pub fn new(storage: Arc<RwLock<HashMap<String, String>>>) -> Self {
        Self { storage }
    }
}

#[derive(Error, Debug)]
pub enum InMemoryStorageError {
    #[error("poisoned")]
    Poisoned,
}

impl Store for InMemoryStorage {
    type Error = InMemoryStorageError;

    fn store_content(&self, permalink: Permalink, content: String) -> Result<(), Self::Error> {
        self.storage
            .write()
            .map_err(|_| InMemoryStorageError::Poisoned)?
            .insert(permalink.path().to_owned(), content);

        Ok(())
    }

    fn store_static_file(&self, path: &Path, css: String) -> Result<(), Self::Error> {
        self.storage
            .write()
            .map_err(|_| InMemoryStorageError::Poisoned)?
            .insert(format!("/{}", path.to_string_lossy().to_string()), css);

        Ok(())
    }
}
