use std::collections::HashMap;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::{io, thread};

use auk::renderer::HtmlElementRenderer;
use auk::visitor::MutVisitor;
use auk::HtmlElement;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use serde_json::json;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::mpsc::unbounded_channel;
use walkdir::WalkDir;
use ws::{Message, Sender, WebSocket};

use crate::content::{
    ContentAggregator, Page, Pages, ParsePageError, ParseSectionError, Section, SectionPath,
    Sections,
};
use crate::markdown::Shortcode;
use crate::render::{
    BaseRenderContext, PageToRender, RenderPageContext, RenderSectionContext, SectionToRender,
};
use crate::storage::{DiskStorage, InMemoryStorage, Store};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum TemplateKey {
    Default,
    Custom(String),
}

pub type RenderIndex = Arc<dyn Fn(&RenderSectionContext) -> HtmlElement + Send + Sync>;

pub type RenderSection = Arc<dyn Fn(&RenderSectionContext) -> HtmlElement + Send + Sync>;

pub type RenderPage = Arc<dyn Fn(&RenderPageContext) -> HtmlElement + Send + Sync>;

struct Templates {
    pub index: RenderIndex,
    pub section: HashMap<TemplateKey, RenderSection>,
    pub page: HashMap<TemplateKey, RenderPage>,
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

struct LinkReplacer<'site> {
    site: &'site Site,
}

impl<'site> MutVisitor for LinkReplacer<'site> {
    type Error = ();

    fn visit_attr(&mut self, name: &str, value: &mut String) -> Result<(), Self::Error> {
        if name == "href" && value.starts_with("@/") {
            *value = format!(
                "{}/{}",
                self.site.base_url,
                value
                    .replacen("@/", "", 1)
                    .replace("_index.md", "")
                    .replace(".md", "")
            );
        }

        Ok(())
    }
}

struct BuildSiteParams {
    base_url: String,
    root_path: PathBuf,
    sass_path: Option<PathBuf>,
    templates: Templates,
    shortcodes: HashMap<String, Shortcode>,
}

pub struct Site {
    base_url: String,
    root_path: PathBuf,
    content_path: PathBuf,
    sass_path: Option<PathBuf>,
    output_path: PathBuf,
    templates: Templates,
    shortcodes: HashMap<String, Shortcode>,
    sections: Sections,
    pages: Pages,
    is_serving: bool,
}

impl Site {
    pub fn builder() -> SiteBuilder<()> {
        SiteBuilder::new()
    }

    fn from_params(params: BuildSiteParams) -> Self {
        let root_path = params.root_path;

        Site {
            base_url: params.base_url,
            root_path: root_path.to_owned(),
            content_path: root_path.join("content"),
            sass_path: params.sass_path.map(|sass_path| root_path.join(sass_path)),
            output_path: root_path.join("public"),
            templates: params.templates,
            shortcodes: params.shortcodes,
            sections: Sections::default(),
            pages: Pages::default(),
            is_serving: false,
        }
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
                if let Some(section) = Section::from_path(&self.content_path, path)? {
                    sections.push(section);
                }
            }
        }

        let mut aggregator = ContentAggregator::new(self.content_path.clone());

        for section in sections {
            aggregator.add_section(section);
        }

        for page in pages {
            aggregator.add_page(page);
        }

        let (sections, pages) = aggregator.aggregate();
        self.sections = sections;
        self.pages = pages;

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

            let ctx = RenderSectionContext {
                base: BaseRenderContext {
                    base_url: &self.base_url,
                    content_path: &self.content_path,
                    sections: &self.sections,
                    pages: &self.pages,
                },
                section: SectionToRender::from_section(section, &self.pages),
            };

            let mut link_replacer = LinkReplacer { site: self };

            let mut rendered_section = section_template(&ctx);
            link_replacer.visit(&mut rendered_section).unwrap();

            let rendered = HtmlElementRenderer::new().render_to_string(&rendered_section)?;

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

            let ctx = RenderPageContext {
                base: BaseRenderContext {
                    base_url: &self.base_url,
                    content_path: &self.content_path,
                    sections: &self.sections,
                    pages: &self.pages,
                },
                page: PageToRender::from_page(page),
            };

            let mut link_replacer = LinkReplacer { site: self };

            let mut rendered_page = page_template(&ctx);
            link_replacer.visit(&mut rendered_page).unwrap();

            let rendered = HtmlElementRenderer::new().render_to_string(&rendered_page)?;

            storage
                .store_rendered_page(&page, rendered)
                .map_err(|err| RenderSiteError::Storage(err.to_string()))?;
        }

        if let Some(sass_path) = self.sass_path.as_ref() {
            fn is_sass(entry: &walkdir::DirEntry) -> bool {
                entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| extension == "sass" || extension == "scss")
                    .unwrap_or(false)
            }

            fn is_partial(entry: &walkdir::DirEntry) -> bool {
                entry
                    .file_name()
                    .to_str()
                    .map(|filename| filename.starts_with('_'))
                    .unwrap_or(false)
            }

            let sass_files = WalkDir::new(sass_path)
                .into_iter()
                .filter_entry(|entry| !is_partial(entry))
                .filter_map(|entry| entry.ok())
                .filter(is_sass)
                .map(|entry| entry.into_path())
                .collect::<Vec<_>>();

            let options = grass::Options::default().style(grass::OutputStyle::Compressed);

            for file in sass_files {
                let css = grass::from_path(&file, &options).unwrap();
                let path = file.strip_prefix(&sass_path).unwrap();

                storage
                    .store_css(&path.with_extension("css"), css)
                    .map_err(|err| RenderSiteError::Storage(err.to_string()))?;
            }
        }

        Ok(())
    }

    pub async fn serve(mut self) -> Result<(), ServeSiteError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

        self.base_url = format!("http://{}", addr.to_string());

        let listener = TcpListener::bind(addr).await?;

        /// [v4.0.2](https://github.com/livereload/livereload-js/blob/v4.0.2/dist/livereload.min.js)
        const LIVE_RELOAD_JS: &'static str = include_str!("../assets/livereload.min.js");

        let live_reload_server = WebSocket::new(|output: Sender| {
            move |message: Message| {
                if message.into_text().unwrap().contains("\"hello\"") {
                    let handshake_response = json!({
                        "command": "hello",
                        "protocols": ["http://livereload.com/protocols/official-7"],
                        "serverName": "Razorbill"
                    });

                    return output.send(Message::text(
                        serde_json::to_string(&handshake_response).unwrap(),
                    ));
                }

                Ok(())
            }
        })
        .unwrap();

        let live_reload_broadcaster = live_reload_server.broadcaster();
        let live_reload_address = SocketAddr::from(([127, 0, 0, 1], 35729));

        let live_reload_server = live_reload_server
            .bind(&live_reload_address)
            .expect("failed to bind live reload server");

        thread::spawn(move || {
            live_reload_server.run().unwrap();
        });

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
                    if path == "/livereload.js" {
                        return Ok(Response::builder()
                            .header(header::CONTENT_TYPE, "text/javascript")
                            .status(StatusCode::OK)
                            .body(full(LIVE_RELOAD_JS.to_owned()))
                            .unwrap());
                    }

                    if let Some(content) = SITE_CONTENT.read().unwrap().get(path) {
                        let content_type = if path.ends_with(".css") {
                            "text/css"
                        } else {
                            "text/html"
                        };

                        return Ok(Response::builder()
                            .header(header::CONTENT_TYPE, content_type)
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

        let site = Arc::new(RwLock::new(self));

        {
            let mut site = site.write().unwrap();
            site.is_serving = true;
            site.render().unwrap();
        }

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
            .watch(&site.read().unwrap().content_path, RecursiveMode::Recursive)
            .unwrap();

        if let Some(sass_path) = site.read().unwrap().sass_path.as_ref() {
            watcher.watch(sass_path, RecursiveMode::Recursive).unwrap();
        }

        tokio::task::spawn(async move {
            use notify::EventKind;

            loop {
                let Some(event) = watcher_rx.recv().await else {
                    continue;
                };

                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        dbg!(&event.paths);

                        let mut site = site.write().unwrap();
                        site.load().unwrap();
                        site.render().unwrap();

                        let reload_message = json!({
                            "command": "reload",
                            "path": "/",
                            "originalPath": "",
                            "liveCSS": true,
                            "liveImg": true,
                            "protocol": ["http://livereload.com/protocols/official-7"]
                        });

                        live_reload_broadcaster
                            .send(serde_json::to_string(&reload_message).unwrap())
                            .unwrap();
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

pub struct SiteBuilder<State> {
    state: PhantomData<State>,
    root_path: PathBuf,
    base_url: String,
    templates: Templates,
    shortcodes: HashMap<String, Shortcode>,
    sass_path: Option<PathBuf>,
}

impl<State> SiteBuilder<State> {
    fn coerce<NewState>(self) -> SiteBuilder<NewState> {
        SiteBuilder {
            state: PhantomData,
            root_path: self.root_path,
            base_url: self.base_url,
            templates: self.templates,
            shortcodes: self.shortcodes,
            sass_path: self.sass_path,
        }
    }

    fn build_site(self) -> Site {
        Site::from_params(BuildSiteParams {
            base_url: self.base_url,
            root_path: self.root_path,
            sass_path: self.sass_path,
            templates: self.templates,
            shortcodes: self.shortcodes,
        })
    }
}

impl SiteBuilder<()> {
    pub fn new() -> Self {
        Self {
            state: PhantomData,
            root_path: PathBuf::new(),
            base_url: String::new(),
            templates: Templates {
                index: Arc::new(|_| auk::div()),
                section: HashMap::new(),
                page: HashMap::new(),
            },
            shortcodes: HashMap::new(),
            sass_path: None,
        }
    }

    pub fn root(self, root_path: impl AsRef<Path>) -> SiteBuilder<WithRootPath> {
        SiteBuilder {
            root_path: root_path.as_ref().to_owned(),
            ..self.coerce()
        }
    }
}

pub struct WithRootPath;

impl SiteBuilder<WithRootPath> {
    pub fn base_url(self, base_url: impl Into<String>) -> SiteBuilder<WithBaseUrl> {
        SiteBuilder {
            base_url: base_url.into(),
            ..self.coerce()
        }
    }
}

pub struct WithBaseUrl;

impl SiteBuilder<WithBaseUrl> {
    pub fn templates(
        self,
        index: impl Fn(&RenderSectionContext) -> HtmlElement + Send + Sync + 'static,
        section: impl Fn(&RenderSectionContext) -> HtmlElement + Send + Sync + 'static,
        page: impl Fn(&RenderPageContext) -> HtmlElement + Send + Sync + 'static,
    ) -> SiteBuilder<WithTemplates> {
        SiteBuilder {
            templates: Templates {
                index: Arc::new(index),
                section: HashMap::from_iter([(
                    TemplateKey::Default,
                    Arc::new(section) as RenderSection,
                )]),
                page: HashMap::from_iter([(TemplateKey::Default, Arc::new(page) as RenderPage)]),
            },
            ..self.coerce()
        }
    }
}

pub struct WithTemplates;

impl SiteBuilder<WithTemplates> {
    pub fn add_section_template(
        mut self,
        name: impl Into<String>,
        template: impl Fn(&RenderSectionContext) -> HtmlElement + Send + Sync + 'static,
    ) -> Self {
        self.templates
            .section
            .insert(TemplateKey::Custom(name.into()), Arc::new(template));
        self
    }

    pub fn add_page_template(
        mut self,
        name: impl Into<String>,
        template: impl Fn(&RenderPageContext) -> HtmlElement + Send + Sync + 'static,
    ) -> Self {
        self.templates
            .page
            .insert(TemplateKey::Custom(name.into()), Arc::new(template));
        self
    }

    pub fn add_shortcode(
        mut self,
        name: impl Into<String>,
        render: impl Fn() -> HtmlElement + Send + Sync + 'static,
    ) -> Self {
        self.shortcodes.insert(
            name.into(),
            Shortcode {
                render: Arc::new(render),
            },
        );
        self
    }

    pub fn with_sass(self, sass_path: impl AsRef<Path>) -> SiteBuilder<WithSass> {
        SiteBuilder {
            sass_path: Some(sass_path.as_ref().to_owned()),
            ..self.coerce()
        }
    }

    pub fn build(self) -> Site {
        self.build_site()
    }
}

pub struct WithSass;

impl SiteBuilder<WithSass> {
    pub fn build(self) -> Site {
        self.build_site()
    }
}
