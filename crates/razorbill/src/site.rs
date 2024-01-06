use std::collections::HashMap;
use std::convert::Infallible;
use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::mpsc::unbounded_channel;
use walkdir::WalkDir;

use crate::content::{Page, ParsePageError, ParseSectionError, Section, SectionPath};
use crate::html::HtmlElement;
use crate::render::{PageToRender, SectionToRender};
use crate::storage::{DiskStorage, InMemoryStorage, Store};

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

    #[error("storage error: {0}")]
    Storage(String),
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
        if self.is_serving {
            self.render_to(InMemoryStorage::new(SITE_CONTENT.clone()))
        } else {
            self.render_to(DiskStorage::new(self.output_path.clone()))
        }
    }

    fn render_to(&mut self, storage: impl Store) -> Result<(), RenderSiteError> {
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

            let rendered = section_template(&SectionToRender::from_section(section, &self.pages))
                .render_to_string()?;

            storage
                .store_rendered_section(&section, rendered)
                .map_err(|err| RenderSiteError::Storage(err.to_string()))?;
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

            let rendered = page_template(&PageToRender::from_page(page)).render_to_string()?;

            storage
                .store_rendered_page(&page, rendered)
                .map_err(|err| RenderSiteError::Storage(err.to_string()))?;
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

        async fn handle_request(
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

        self.render().unwrap();

        let (watcher_tx, mut watcher_rx) = unbounded_channel();

        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                let event = result.unwrap();

                watcher_tx.send(event).unwrap();
            },
            notify::Config::default(),
        )
        .unwrap();

        watcher
            .watch(&self.content_path, RecursiveMode::Recursive)
            .unwrap();

        tokio::task::spawn(async move {
            use notify::EventKind;

            loop {
                let Some(event) = watcher_rx.recv().await else {
                    continue;
                };

                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        dbg!(&event.paths);
                    }
                    _ => {}
                }
            }
        });

        loop {
            let (stream, _) = listener.accept().await?;

            let io = TokioIo::new(stream);

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(handle_request))
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
