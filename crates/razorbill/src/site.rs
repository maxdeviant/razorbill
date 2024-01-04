use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use thiserror::Error;
use walkdir::WalkDir;

use crate::content::{Page, ParsePageError};
use crate::html::HtmlElement;

#[derive(Error, Debug)]
pub enum LoadSiteError {
    #[error("failed to walk content directory: {0}")]
    Io(#[from] walkdir::Error),

    #[error("failed to parse page: {0}")]
    ParsePage(#[from] ParsePageError),
}

#[derive(Error, Debug)]
pub enum RenderSiteError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("render error: {0}")]
    RenderPage(#[from] std::fmt::Error),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
enum TemplateKind {
    Index,
    Section,
    Page,
    Custom(String),
}

pub struct Site {
    root_path: PathBuf,
    content_path: PathBuf,
    output_path: PathBuf,
    templates: HashMap<TemplateKind, Box<dyn Fn(&Page) -> HtmlElement>>,
    pages: Vec<Page>,
}

impl Site {
    pub fn builder() -> SiteBuilder<()> {
        SiteBuilder::new()
    }

    pub fn load(&mut self) -> Result<(), LoadSiteError> {
        let walker = WalkDir::new(&self.content_path)
            .follow_links(true)
            .into_iter();

        let mut pages = Vec::new();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            let Some(filename) = entry.file_name().to_str() else {
                continue;
            };

            if !path.is_dir() && (!filename.ends_with(".md") || filename.starts_with(".")) {
                continue;
            }

            if !path.is_dir() {
                pages.push(Page::from_path(&self.content_path, path)?);

                continue;
            }
        }

        self.pages = pages;

        Ok(())
    }

    pub fn add_template(
        &mut self,
        name: impl Into<String>,
        template: impl Fn(&Page) -> HtmlElement + 'static,
    ) {
        self.templates
            .insert(TemplateKind::Custom(name.into()), Box::new(template));
    }

    pub fn render(&mut self) -> Result<(), RenderSiteError> {
        for page in &self.pages {
            let output_dir = self
                .output_path
                .join(PathBuf::from_str(&page.path.0.trim_start_matches("/")).unwrap());

            fs::create_dir_all(&output_dir)?;

            let output_path = output_dir.join("index.html");
            let mut output_file = File::create(&output_path)?;

            let page_template = self
                .templates
                .get(&TemplateKind::Page)
                .expect("no page template set");

            let rendered = page_template(page).render_to_string()?;

            output_file.write_all(rendered.as_bytes())?;

            println!("Wrote {:?}", output_path);
        }

        Ok(())
    }
}

// pub struct

pub struct WithRootPath {
    root_path: PathBuf,
}

pub struct WithTemplates {
    root_path: PathBuf,
    index_template: Box<dyn Fn() -> HtmlElement>,
    section_template: Box<dyn Fn() -> HtmlElement>,
    page_template: Box<dyn Fn(&Page) -> HtmlElement>,
}

pub struct SiteBuilder<T> {
    state: T,
}

impl SiteBuilder<()> {
    pub fn new() -> Self {
        Self { state: () }
    }

    pub fn root(self, root_path: impl AsRef<Path>) -> SiteBuilder<WithRootPath> {
        SiteBuilder {
            state: WithRootPath {
                root_path: root_path.as_ref().to_owned(),
            },
        }
    }
}

impl SiteBuilder<WithRootPath> {
    pub fn templates(
        self,
        index: impl Fn() -> HtmlElement + 'static,
        section: impl Fn() -> HtmlElement + 'static,
        page: impl Fn(&Page) -> HtmlElement + 'static,
    ) -> SiteBuilder<WithTemplates> {
        SiteBuilder {
            state: WithTemplates {
                root_path: self.state.root_path,
                index_template: Box::new(index),
                section_template: Box::new(section),
                page_template: Box::new(page),
            },
        }
    }
}

impl SiteBuilder<WithTemplates> {
    pub fn build(self) -> Site {
        let root_path = self.state.root_path;

        let mut templates = HashMap::new();
        // templates.insert(TemplateKind::Index, self.state.index_template);
        // templates.insert(TemplateKind::Section, self.state.section_template);
        templates.insert(TemplateKind::Page, self.state.page_template);

        Site {
            root_path: root_path.to_owned(),
            content_path: root_path.join("content"),
            output_path: root_path.join("public"),
            templates,
            pages: Vec::new(),
        }
    }
}
