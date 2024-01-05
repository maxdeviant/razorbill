use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use thiserror::Error;
use walkdir::WalkDir;

use crate::content::{Page, ParsePageError, ParseSectionError, Section};
use crate::html::HtmlElement;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum TemplateKey {
    Default,
    Custom(String),
}

struct Templates {
    pub index: Box<dyn Fn() -> HtmlElement>,
    pub section: HashMap<TemplateKey, Box<dyn Fn(&Section) -> HtmlElement>>,
    pub page: HashMap<TemplateKey, Box<dyn Fn(&Page) -> HtmlElement>>,
}

#[derive(Error, Debug)]
pub enum LoadSiteError {
    #[error("failed to walk content directory: {0}")]
    Io(#[from] walkdir::Error),

    #[error("failed to parse section: {0}")]
    ParseSection(#[from] ParseSectionError),

    #[error("failed to parse page: {0}")]
    ParsePage(#[from] ParsePageError),
}

#[derive(Error, Debug)]
pub enum RenderSiteError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("render error: {0}")]
    RenderPage(#[from] std::fmt::Error),

    #[error("template not found: {0:?}")]
    TemplateNotFound(TemplateKey),
}

pub struct Site {
    root_path: PathBuf,
    content_path: PathBuf,
    output_path: PathBuf,
    templates: Templates,
    sections: Vec<Section>,
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
        let mut sections = Vec::new();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            let Some(filename) = entry.file_name().to_str() else {
                continue;
            };

            if !path.is_dir() {
                if !filename.ends_with(".md")
                    || filename.starts_with(".")
                    || filename == "_index.md"
                {
                    continue;
                }

                pages.push(Page::from_path(&self.content_path, path)?);
            } else {
                let section = Section::from_path(&self.content_path, path)?;
                sections.push(section);
            }
        }

        self.sections = sections;
        self.pages = pages;

        Ok(())
    }

    pub fn render(&mut self) -> Result<(), RenderSiteError> {
        for section in &self.sections {
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

            let template_name = section
                .meta
                .template
                .clone()
                .map(TemplateKey::Custom)
                .unwrap_or(TemplateKey::Default);

            let section_template = self
                .templates
                .section
                .get(&template_name)
                .ok_or_else(|| RenderSiteError::TemplateNotFound(template_name))?;

            let rendered = section_template(&section).render_to_string()?;

            output_file.write_all(rendered.as_bytes())?;
        }

        for page in &self.pages {
            let output_dir = self
                .output_path
                .join(PathBuf::from_str(&page.path.0.trim_start_matches("/")).unwrap());

            fs::create_dir_all(&output_dir)?;

            let output_path = output_dir.join("index.html");
            let mut output_file = File::create(&output_path)?;

            let template_name = page
                .meta
                .template
                .clone()
                .map(TemplateKey::Custom)
                .unwrap_or(TemplateKey::Default);

            let page_template = self
                .templates
                .page
                .get(&template_name)
                .ok_or_else(|| RenderSiteError::TemplateNotFound(template_name))?;

            let rendered = page_template(page).render_to_string()?;

            output_file.write_all(rendered.as_bytes())?;

            println!("Wrote {:?}", output_path);
        }

        Ok(())
    }
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

pub struct WithRootPath {
    root_path: PathBuf,
}

impl SiteBuilder<WithRootPath> {
    pub fn templates(
        self,
        index: impl Fn() -> HtmlElement + 'static,
        section: impl Fn(&Section) -> HtmlElement + 'static,
        page: impl Fn(&Page) -> HtmlElement + 'static,
    ) -> SiteBuilder<WithTemplates> {
        SiteBuilder {
            state: WithTemplates {
                root_path: self.state.root_path,
                templates: Templates {
                    index: Box::new(index),
                    section: HashMap::from_iter([(
                        TemplateKey::Default,
                        Box::new(section) as Box<dyn Fn(&Section) -> HtmlElement>,
                    )]),
                    page: HashMap::from_iter([(
                        TemplateKey::Default,
                        Box::new(page) as Box<dyn Fn(&Page) -> HtmlElement>,
                    )]),
                },
            },
        }
    }
}

pub struct WithTemplates {
    root_path: PathBuf,
    templates: Templates,
}

impl SiteBuilder<WithTemplates> {
    pub fn add_section_template(
        mut self,
        name: impl Into<String>,
        template: impl Fn(&Section) -> HtmlElement + 'static,
    ) -> Self {
        self.state
            .templates
            .section
            .insert(TemplateKey::Custom(name.into()), Box::new(template));
        self
    }

    pub fn add_page_template(
        mut self,
        name: impl Into<String>,
        template: impl Fn(&Page) -> HtmlElement + 'static,
    ) -> Self {
        self.state
            .templates
            .page
            .insert(TemplateKey::Custom(name.into()), Box::new(template));
        self
    }

    pub fn build(self) -> Site {
        let root_path = self.state.root_path;

        Site {
            root_path: root_path.to_owned(),
            content_path: root_path.join("content"),
            output_path: root_path.join("public"),
            templates: self.state.templates,
            sections: Vec::new(),
            pages: Vec::new(),
        }
    }
}
