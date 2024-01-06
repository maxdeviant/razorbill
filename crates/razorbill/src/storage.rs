use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use thiserror::Error;

use crate::content::{Page, Section};

pub trait Store {
    type Error: std::error::Error;

    fn store_rendered_section(
        &self,
        section: &Section,
        rendered_html: String,
    ) -> Result<(), Self::Error>;

    fn store_rendered_page(&self, page: &Page, rendered_html: String) -> Result<(), Self::Error>;

    fn store_css(&self, path: &Path, css: String) -> Result<(), Self::Error>;
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

    fn store_rendered_section(
        &self,
        section: &Section,
        rendered_html: String,
    ) -> Result<(), Self::Error> {
        let output_dir = self.output_path.join(
            PathBuf::from_str(
                &section
                    .path
                    .0
                    .trim_end_matches("/_index")
                    .trim_start_matches("/"),
            )
            .unwrap(),
        );

        fs::create_dir_all(&output_dir)?;

        let output_path = output_dir.join("index.html");
        let mut output_file = File::create(&output_path)?;

        output_file.write_all(rendered_html.as_bytes())?;

        Ok(())
    }

    fn store_rendered_page(&self, page: &Page, rendered_html: String) -> Result<(), Self::Error> {
        let output_dir = self
            .output_path
            .join(PathBuf::from_str(&page.path.0.trim_start_matches("/")).unwrap());

        fs::create_dir_all(&output_dir)?;

        let output_path = output_dir.join("index.html");
        let mut output_file = File::create(&output_path)?;

        output_file.write_all(rendered_html.as_bytes())?;

        Ok(())
    }

    fn store_css(&self, path: &Path, css: String) -> Result<(), Self::Error> {
        if let Some(parent) = path.parent() {
            println!("mkdir {parent:?}");
        }

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

    fn store_rendered_section(
        &self,
        section: &Section,
        rendered_html: String,
    ) -> Result<(), Self::Error> {
        self.storage
            .write()
            .map_err(|_| InMemoryStorageError::Poisoned)?
            .insert(section.path.0.replace("/_index", "/"), rendered_html);

        Ok(())
    }

    fn store_rendered_page(&self, page: &Page, rendered_html: String) -> Result<(), Self::Error> {
        self.storage
            .write()
            .map_err(|_| InMemoryStorageError::Poisoned)?
            .insert(page.path.0.clone(), rendered_html);

        Ok(())
    }

    fn store_css(&self, path: &Path, css: String) -> Result<(), Self::Error> {
        self.storage
            .write()
            .map_err(|_| InMemoryStorageError::Poisoned)?
            .insert(format!("/{}", path.to_string_lossy().to_string()), css);

        Ok(())
    }
}
