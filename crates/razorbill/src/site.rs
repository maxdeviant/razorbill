use std::collections::HashMap;
use std::convert::Infallible;
use std::fs::{self, File};
use std::io::{self, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use once_cell::sync::Lazy;
use thiserror::Error;
use tokio::net::TcpListener;
use walkdir::WalkDir;

use crate::content::{Page, ParsePageError, ParseSectionError, Section, SectionPath};
use crate::html::HtmlElement;
use crate::render::{PageToRender, SectionToRender};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum TemplateKey {
    Default,
    Custom(String),
}

struct Templates {
    pub index: Box<dyn Fn(&SectionToRender) -> HtmlElement>,
    pub section: HashMap<TemplateKey, Box<dyn Fn(&SectionToRender) -> HtmlElement>>,
    pub page: HashMap<TemplateKey, Box<dyn Fn(&PageToRender) -> HtmlElement>>,
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

#[derive(Error, Debug)]
pub enum ServeSiteError {
    #[error("async IO error: {0}")]
    AsyncIo(#[from] tokio::io::Error),
}

static SITE_CONTENT: Lazy<Arc<RwLock<HashMap<String, String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub struct Site {
    root_path: PathBuf,
    content_path: PathBuf,
    output_path: PathBuf,
    templates: Templates,
    sections: HashMap<PathBuf, Section>,
    pages: HashMap<PathBuf, Page>,
    is_serving: bool,
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

        for section in sections {
            self.sections.insert(section.file.path.clone(), section);
        }

        for page in pages {
            self.pages.insert(page.file.path.clone(), page);
        }

        for (path, page) in self.pages.iter_mut() {
            let parent_section_path = page.file.parent.join("_index.md");

            if let Some(parent_section) = self.sections.get_mut(&parent_section_path) {
                parent_section.pages.push(path.clone());
            }
        }

        Ok(())
    }

    pub fn render(&mut self) -> Result<(), RenderSiteError> {
        for section in self.sections.values() {
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

            let section_template = if section.path == SectionPath("/_index".to_string()) {
                &self.templates.index
            } else {
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

                section_template
            };

            let pages = section
                .pages
                .iter()
                .map(|page| self.pages.get(page).unwrap())
                .map(|page| PageToRender {
                    title: &page.meta.title,
                    slug: &page.slug,
                    path: &page.path.0,
                    raw_content: &page.raw_content,
                })
                .collect::<Vec<_>>();

            let section = SectionToRender {
                title: &section.meta.title,
                path: &section.path.0,
                raw_content: &section.raw_content,
                pages,
            };

            let rendered = section_template(&section).render_to_string()?;

            output_file.write_all(rendered.as_bytes())?;
        }

        for page in self.pages.values() {
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

            let page = PageToRender {
                title: &page.meta.title,
                slug: &page.slug,
                path: &page.path.0,
                raw_content: &page.raw_content,
            };

            let rendered = page_template(&page).render_to_string()?;

            output_file.write_all(rendered.as_bytes())?;

            println!("Wrote {:?}", output_path);
        }

        Ok(())
    }

    fn render_in_memory(&mut self) -> Result<(), RenderSiteError> {
        for section in self.sections.values() {
            let section_template = if section.path == SectionPath("/_index".to_string()) {
                &self.templates.index
            } else {
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

                section_template
            };

            let rendered = section_template(&SectionToRender::from_section(&section, &self.pages))
                .render_to_string()?;

            SITE_CONTENT
                .write()
                .unwrap()
                .insert(section.path.0.replace("/_index", "/"), rendered);
        }

        for page in self.pages.values() {
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

            let rendered = page_template(&PageToRender::from_page(&page)).render_to_string()?;

            SITE_CONTENT
                .write()
                .unwrap()
                .insert(page.path.0.clone(), rendered);
        }

        Ok(())
    }

    pub async fn serve(&mut self) -> Result<(), ServeSiteError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

        let listener = TcpListener::bind(addr).await?;

        fn empty() -> BoxBody<Bytes, hyper::Error> {
            Empty::<Bytes>::new()
                .map_err(|never| match never {})
                .boxed()
        }

        fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
            Full::new(chunk.into())
                .map_err(|never| match never {})
                .boxed()
        }

        async fn hello(
            req: Request<hyper::body::Incoming>,
        ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, Infallible> {
            match (req.method(), req.uri().path()) {
                (&Method::GET, path) => {
                    if let Some(content) = SITE_CONTENT.read().unwrap().get(path) {
                        return Ok(Response::builder()
                            .header(header::CONTENT_TYPE, "text/html")
                            .status(StatusCode::OK)
                            .body(full(content.to_owned()))
                            .unwrap());
                    }

                    let mut not_found = Response::new(empty());
                    *not_found.status_mut() = StatusCode::NOT_FOUND;
                    Ok(not_found)
                }
                _ => {
                    let mut not_found = Response::new(empty());
                    *not_found.status_mut() = StatusCode::NOT_FOUND;
                    Ok(not_found)
                }
            }
        }

        self.is_serving = true;

        self.render_in_memory().unwrap();

        loop {
            let (stream, _) = listener.accept().await?;

            let io = TokioIo::new(stream);

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(hello))
                    .await
                {
                    eprintln!("Error serving connection: {err:?}");
                }
            });
        }
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
        index: impl Fn(&SectionToRender) -> HtmlElement + 'static,
        section: impl Fn(&SectionToRender) -> HtmlElement + 'static,
        page: impl Fn(&PageToRender) -> HtmlElement + 'static,
    ) -> SiteBuilder<WithTemplates> {
        SiteBuilder {
            state: WithTemplates {
                root_path: self.state.root_path,
                templates: Templates {
                    index: Box::new(index),
                    section: HashMap::from_iter([(
                        TemplateKey::Default,
                        Box::new(section) as Box<dyn Fn(&SectionToRender) -> HtmlElement>,
                    )]),
                    page: HashMap::from_iter([(
                        TemplateKey::Default,
                        Box::new(page) as Box<dyn Fn(&PageToRender) -> HtmlElement>,
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
        template: impl Fn(&SectionToRender) -> HtmlElement + 'static,
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
        template: impl Fn(&PageToRender) -> HtmlElement + 'static,
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
            sections: HashMap::new(),
            pages: HashMap::new(),
            is_serving: false,
        }
    }
}
