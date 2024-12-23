use std::collections::HashMap;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::{fs, io, thread};

use anyhow::Result;
use auk::renderer::HtmlElementRenderer;
use auk::visitor::{noop_visit_element, MutVisitor, Visitor};
use auk::HtmlElement;
use auk_markdown::MarkdownComponents;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use mime_guess::MimeGuess;
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
    Sections, Taxonomy, TaxonomyTerm, AVERAGE_ADULT_WPM,
};
use crate::feed::render_feed;
use crate::markdown::{markdown_with_shortcodes, DefaultMarkdownComponents, Shortcode};
use crate::permalink::Permalink;
use crate::render::{
    BaseRenderContext, PageToRender, RenderPageContext, RenderSectionContext,
    RenderTaxonomyContext, RenderTaxonomyTermContext, SectionToRender, TaxonomyTermToRender,
    TaxonomyToRender,
};
use crate::sitemap::render_sitemap;
use crate::storage::{DiskStorage, InMemoryStorage, Store};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum TemplateKey {
    Default,
    Custom(String),
}

pub type RenderIndex = Arc<dyn Fn(&RenderSectionContext) -> HtmlElement + Send + Sync>;

pub type RenderSection = Arc<dyn Fn(&RenderSectionContext) -> HtmlElement + Send + Sync>;

pub type RenderPage = Arc<dyn Fn(&RenderPageContext) -> HtmlElement + Send + Sync>;

pub type RenderTaxonomy = Arc<dyn Fn(&RenderTaxonomyContext) -> HtmlElement + Send + Sync>;

pub type RenderTaxonomyTerm = Arc<dyn Fn(&RenderTaxonomyTermContext) -> HtmlElement + Send + Sync>;

struct Templates {
    pub index: RenderIndex,
    pub section: HashMap<TemplateKey, RenderSection>,
    pub page: HashMap<TemplateKey, RenderPage>,
    pub taxonomy: HashMap<String, RenderTaxonomy>,
    pub taxonomy_term: HashMap<String, RenderTaxonomyTerm>,
    pub not_found: Option<Arc<dyn Fn(&BaseRenderContext) -> HtmlElement + Send + Sync>>,
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

struct LinkReplacer<'a> {
    site: &'a Site,
    current_url: &'a Permalink,
    inside_footnote_reference: bool,
}

impl<'a> LinkReplacer<'a> {
    pub fn new(site: &'a Site, current_url: &'a Permalink) -> Self {
        Self {
            site,
            current_url,
            inside_footnote_reference: false,
        }
    }
}

impl<'a> MutVisitor for LinkReplacer<'a> {
    type Error = ();

    fn visit(&mut self, element: &mut HtmlElement) -> Result<(), Self::Error> {
        if let Some(class) = element.attrs.get("class") {
            if class.contains("footnote-reference") {
                self.inside_footnote_reference = true;
            }
        }

        noop_visit_element(self, element)?;

        self.inside_footnote_reference = false;

        Ok(())
    }

    fn visit_attr(&mut self, name: &str, value: &mut String) -> Result<(), Self::Error> {
        if name == "href" {
            if value.starts_with("@/") {
                let path = self.site.content_path.join(value.replacen("@/", "", 1));

                let permalink = None
                    .or_else(|| {
                        self.site
                            .pages
                            .get(&path)
                            .map(|page| page.permalink.clone())
                    })
                    .or_else(|| {
                        self.site
                            .sections
                            .get(&path)
                            .map(|section| section.permalink.clone())
                    });

                if let Some(permalink) = permalink {
                    *value = permalink.as_str().to_owned();
                } else {
                    eprintln!("Invalid link: {value}");
                }

                return Ok(());
            }

            if value.starts_with('#') && !self.inside_footnote_reference {
                *value = format!(
                    "{current_url}{value}",
                    current_url = self.current_url.as_str()
                )
            }
        }

        Ok(())
    }
}

struct LiveReloadInjector {
    port: u16,
}

impl LiveReloadInjector {
    pub fn inject(live_reload_port: Option<u16>, element: &mut HtmlElement) {
        let Some(port) = live_reload_port else {
            return;
        };

        let mut injector = Self::new(port);
        injector.visit(element).unwrap();
    }

    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

impl MutVisitor for LiveReloadInjector {
    type Error = ();

    fn visit(&mut self, element: &mut HtmlElement) -> Result<(), Self::Error> {
        use auk::*;

        noop_visit_element(self, element)?;

        if element.tag_name == "body" {
            element.children.extend([script()
                .src(format!("/livereload.js?port={}&amp;mindelay=10", self.port))
                .into()])
        }

        Ok(())
    }
}

struct BuildSiteParams {
    base_url: String,
    title: Option<String>,
    include_drafts: bool,
    reading_speed: usize,
    root_path: PathBuf,
    sass_path: Option<PathBuf>,
    sass_load_paths: Vec<PathBuf>,
    templates: Templates,
    markdown_components: Box<dyn MarkdownComponents>,
    shortcodes: HashMap<String, Shortcode>,
    taxonomies: Vec<Taxonomy>,
}

pub struct SiteConfig {
    pub base_url: String,
    pub title: Option<String>,
    pub taxonomies: Vec<Taxonomy>,
    /// The reading speed (in WPM) to use when determining reading time.
    pub reading_speed: usize,
}

pub struct Site {
    pub(crate) config: SiteConfig,
    #[allow(unused)]
    root_path: PathBuf,
    content_path: PathBuf,
    /// The path to the `static` directory that houses static assets.
    static_path: PathBuf,
    sass_path: Option<PathBuf>,
    sass_load_paths: Vec<PathBuf>,
    output_path: PathBuf,
    templates: Templates,
    markdown_components: Box<dyn MarkdownComponents>,
    shortcodes: HashMap<String, Shortcode>,
    pub(crate) sections: Sections,
    pub(crate) pages: Pages,
    pub(crate) taxonomies: HashMap<String, HashMap<String, Vec<PathBuf>>>,
    include_drafts: bool,
    is_serving: bool,
    live_reload_port: Option<u16>,
}

impl Site {
    pub fn builder() -> SiteBuilder<()> {
        SiteBuilder::new()
    }

    fn from_params(params: BuildSiteParams) -> Self {
        let root_path = params.root_path;

        Site {
            config: SiteConfig {
                base_url: params.base_url,
                title: params.title,
                taxonomies: params.taxonomies,
                reading_speed: params.reading_speed,
            },
            root_path: root_path.to_owned(),
            content_path: root_path.join("content"),
            static_path: root_path.join("static"),
            sass_path: params.sass_path.map(|sass_path| root_path.join(sass_path)),
            sass_load_paths: params
                .sass_load_paths
                .into_iter()
                .map(|load_path| root_path.join(load_path))
                .collect(),
            output_path: root_path.join("public"),
            templates: params.templates,
            markdown_components: params.markdown_components,
            shortcodes: params.shortcodes,
            sections: Sections::default(),
            pages: Pages::default(),
            taxonomies: HashMap::new(),
            include_drafts: params.include_drafts,
            is_serving: false,
            live_reload_port: None,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    pub fn set_output_path(&mut self, output_path: impl AsRef<Path>) {
        self.output_path = output_path.as_ref().to_owned();
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

                pages.push(Page::from_path(&self.config, &self.content_path, path)?);
            } else {
                if let Some(section) = Section::from_path(&self.config, &self.content_path, path)? {
                    sections.push(section);
                }
            }
        }

        let mut aggregator =
            ContentAggregator::new(self.content_path.clone(), self.config.taxonomies.clone());

        for section in sections {
            if section.meta.draft && !self.include_drafts {
                continue;
            }

            aggregator.add_section(section);
        }

        for page in pages {
            if page.meta.draft && !self.include_drafts {
                continue;
            }

            aggregator.add_page(page);
        }

        let (sections, pages, taxonomies) = aggregator.aggregate();
        self.sections = sections;
        self.pages = pages;
        self.taxonomies = taxonomies;

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
        self.render_aliases(&storage);

        let mut sections_to_update = HashMap::new();

        for (section_path, section) in self.sections.iter() {
            let (mut content, table_of_contents) = markdown_with_shortcodes(
                &section.raw_content,
                &self.markdown_components,
                &self.shortcodes,
            );

            let mut link_replacer = LinkReplacer::new(&self, &section.permalink);
            link_replacer.visit_children(&mut content).unwrap();

            sections_to_update.insert(section_path.clone(), (content, table_of_contents));
        }

        for (section_path, (content, table_of_contents)) in sections_to_update {
            let section = self.sections.get_mut(&section_path).unwrap();
            section.content = content;
            section.table_of_contents = table_of_contents;
        }

        let mut pages_to_update = HashMap::new();

        for (page_path, page) in self.pages.iter() {
            let (mut content, table_of_contents) = markdown_with_shortcodes(
                &page.raw_content,
                &self.markdown_components,
                &self.shortcodes,
            );

            let mut link_replacer = LinkReplacer::new(&self, &page.permalink);
            link_replacer.visit_children(&mut content).unwrap();

            pages_to_update.insert(page_path.clone(), (content, table_of_contents));
        }

        for (page_path, (content, table_of_contents)) in pages_to_update {
            let page = self.pages.get_mut(&page_path).unwrap();
            page.content = content;
            page.table_of_contents = table_of_contents;
        }

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
                    base_url: self.base_url(),
                    content_path: &self.content_path,
                    markdown_components: &self.markdown_components,
                    shortcodes: &self.shortcodes,
                    sections: &self.sections,
                    pages: &self.pages,
                },
                section: SectionToRender::from_section(section, &self.pages),
            };

            let mut rendered_section = section_template(&ctx);

            let mut link_replacer = LinkReplacer::new(&self, &section.permalink);
            link_replacer.visit(&mut rendered_section).unwrap();

            LiveReloadInjector::inject(self.live_reload_port, &mut rendered_section);

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
                    base_url: self.base_url(),
                    content_path: &self.content_path,
                    markdown_components: &self.markdown_components,
                    shortcodes: &self.shortcodes,
                    sections: &self.sections,
                    pages: &self.pages,
                },
                page: PageToRender::from_page(page),
            };

            let mut rendered_page = page_template(&ctx);

            let mut link_replacer = LinkReplacer::new(&self, &page.permalink);
            link_replacer.visit(&mut rendered_page).unwrap();

            LiveReloadInjector::inject(self.live_reload_port, &mut rendered_page);

            let rendered = HtmlElementRenderer::new().render_to_string(&rendered_page)?;

            storage
                .store_rendered_page(&page, rendered)
                .map_err(|err| RenderSiteError::Storage(err.to_string()))?;
        }

        render_sitemap(&self, &storage);
        render_feed(
            &self,
            Permalink::from_path(&self.config, "atom.xml"),
            None,
            self.pages.values().collect(),
            &storage,
        );
        self.render_404_page(&storage)?;
        self.render_robots_txt(&storage)?;
        self.render_taxonomies(&storage)?;

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

            let options = grass::Options::default()
                .style(grass::OutputStyle::Compressed)
                .load_paths(&self.sass_load_paths);

            for file in sass_files {
                let css = grass::from_path(&file, &options).unwrap();
                let path = file.strip_prefix(&sass_path).unwrap();

                storage
                    .store_static_file(&path.with_extension("css"), css)
                    .map_err(|err| RenderSiteError::Storage(err.to_string()))?;
            }
        }

        self.copy_static_directory().unwrap();

        Ok(())
    }

    fn render_aliases(&self, storage: &impl Store) {
        for section in self.sections.values() {
            for alias in &section.meta.aliases {
                self.render_alias(alias, &section.permalink, storage);
            }
        }

        for page in self.pages.values() {
            for alias in &page.meta.aliases {
                self.render_alias(alias, &page.permalink, storage);
            }
        }
    }

    fn render_alias(&self, alias: &str, permalink: &Permalink, storage: &impl Store) {
        use auk::*;

        let url = permalink.as_str();
        let alias_template = vec![
            meta().charset("utf-8").into(),
            link().rel("canonical").href(url).into(),
            meta()
                .http_equiv("refresh")
                .content(format!("0; url={url}"))
                .into(),
            title().child("Redirect").into(),
            p().child(a().href(url).child("Click here"))
                .child(" to be redirected.")
                .into(),
        ];

        let mut html_renderer = HtmlElementRenderer::new();
        html_renderer.visit_children(&alias_template).unwrap();
        let mut alias_html = "<!DOCTYPE html>".to_string();
        alias_html.push_str(html_renderer.html());

        storage
            .store_content(Permalink::from_path(&self.config, alias), alias_html)
            .unwrap();
    }

    fn render_404_page(&self, storage: &impl Store) -> Result<(), RenderSiteError> {
        let page_template = self.templates.not_found.clone().unwrap_or_else(|| {
            Arc::new(|_ctx| {
                use auk::*;

                html()
                    .child(head().child(title().child("404: Page Not Found")))
                    .child(body().child(h1().child("404: Page Not Found")))
            })
        });

        let ctx = BaseRenderContext {
            base_url: self.base_url(),
            content_path: &self.content_path,
            markdown_components: &self.markdown_components,
            shortcodes: &self.shortcodes,
            sections: &self.sections,
            pages: &self.pages,
        };

        let mut rendered_page = page_template(&ctx);
        LiveReloadInjector::inject(self.live_reload_port, &mut rendered_page);
        let rendered = HtmlElementRenderer::new().render_to_string(&rendered_page)?;

        storage
            .store_content(Permalink::from_path(&self.config, "404.html"), rendered)
            .map_err(|err| RenderSiteError::Storage(err.to_string()))?;

        Ok(())
    }

    fn render_robots_txt(&self, storage: &impl Store) -> Result<(), RenderSiteError> {
        let sitemap_url = Permalink::from_path(&self.config, "sitemap.xml");

        let lines = vec![
            "User-agent: *".to_string(),
            "Disallow:".to_string(),
            "Allow: /".to_string(),
            format!("Sitemap: {}", sitemap_url.as_str()),
            String::new(),
        ];

        storage
            .store_content(
                Permalink::from_path(&self.config, "robots.txt"),
                lines.join("\n"),
            )
            .map_err(|err| RenderSiteError::Storage(err.to_string()))?;

        Ok(())
    }

    fn render_taxonomies(&self, storage: &impl Store) -> Result<(), RenderSiteError> {
        for (taxonomy, pages_by_term) in &self.taxonomies {
            let taxonomy_template = self
                .templates
                .taxonomy
                .get(taxonomy)
                .expect("taxonomy template not found for {taxonomy:?}");

            let mut terms = pages_by_term
                .iter()
                .map(|(term, pages)| TaxonomyTerm {
                    name: term.clone(),
                    permalink: Permalink::from_path(&self.config, &format!("/{taxonomy}/{term}")),
                    pages: pages.clone(),
                })
                .collect::<Vec<_>>();

            terms.sort_by(|a, b| a.name.cmp(&b.name));

            let ctx = RenderTaxonomyContext {
                base: BaseRenderContext {
                    base_url: self.base_url(),
                    content_path: &self.content_path,
                    markdown_components: &self.markdown_components,
                    shortcodes: &self.shortcodes,
                    sections: &self.sections,
                    pages: &self.pages,
                },
                taxonomy: TaxonomyToRender {
                    name: taxonomy.as_str(),
                    terms: terms
                        .iter()
                        .map(|term| {
                            let pages = term
                                .pages
                                .iter()
                                .map(|page| self.pages.get(page).unwrap())
                                .map(PageToRender::from_page)
                                .collect::<Vec<_>>();

                            TaxonomyTermToRender {
                                name: term.name.as_str(),
                                permalink: term.permalink.as_str(),
                                pages,
                            }
                        })
                        .collect(),
                },
            };

            let rendered_taxonomy_page = taxonomy_template(&ctx);

            storage
                .store_content(
                    Permalink::from_path(&self.config, &format!("/{taxonomy}")),
                    HtmlElementRenderer::new().render_to_string(&rendered_taxonomy_page)?,
                )
                .map_err(|err| RenderSiteError::Storage(err.to_string()))?;

            for (term, pages) in pages_by_term {
                let term_template = self
                    .templates
                    .taxonomy_term
                    .get(taxonomy)
                    .expect("taxonomy term template not found for {taxonomy:?}");

                let permalink = Permalink::from_path(&self.config, &format!("/{taxonomy}/{term}"));
                let pages = pages
                    .iter()
                    .map(|page| self.pages.get(page).unwrap())
                    .collect::<Vec<_>>();
                let pages_to_render = pages
                    .iter()
                    .copied()
                    .map(PageToRender::from_page)
                    .collect::<Vec<_>>();

                let ctx = RenderTaxonomyTermContext {
                    base: BaseRenderContext {
                        base_url: self.base_url(),
                        content_path: &self.content_path,
                        markdown_components: &self.markdown_components,
                        shortcodes: &self.shortcodes,
                        sections: &self.sections,
                        pages: &self.pages,
                    },
                    term: TaxonomyTermToRender {
                        name: term.as_str(),
                        permalink: permalink.as_str(),
                        pages: pages_to_render,
                    },
                };

                let rendered_term_page = term_template(&ctx);

                storage
                    .store_content(
                        Permalink::from_path(&self.config, &format!("/{taxonomy}/{term}")),
                        HtmlElementRenderer::new().render_to_string(&rendered_term_page)?,
                    )
                    .map_err(|err| RenderSiteError::Storage(err.to_string()))?;

                render_feed(
                    &self,
                    Permalink::from_path(&self.config, &format!("{taxonomy}/{term}/atom.xml")),
                    Some(term),
                    pages,
                    storage,
                );
            }
        }

        Ok(())
    }

    fn copy_static_directory(&self) -> Result<()> {
        let source = self.static_path.clone();
        if !source.exists() {
            return Ok(());
        }

        let walker = WalkDir::new(&source).follow_links(true).into_iter();

        for entry in walker {
            let entry = entry?;

            let relative_path = entry.path().strip_prefix(&source).unwrap();
            let destination_path = self.output_path.join(relative_path);

            if entry.path().is_dir() {
                if !destination_path.exists() {
                    fs::create_dir_all(&destination_path)?;
                }
            } else {
                fs::copy(entry.path(), &destination_path)?;
            }
        }

        Ok(())
    }

    pub fn build(mut self) -> Result<()> {
        self.load()?;
        self.render()?;

        Ok(())
    }

    pub async fn serve(mut self) -> Result<(), ServeSiteError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

        self.config.base_url = format!("http://{}", addr.to_string());

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
        let live_reload_port = 35729;
        let live_reload_address = SocketAddr::from(([127, 0, 0, 1], live_reload_port));
        self.live_reload_port = Some(live_reload_port);

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
            static_path: Arc<Path>,
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

                    let extension = path.rsplit_once('.').map(|(_, extension)| extension);

                    if let Some(content) = SITE_CONTENT.read().unwrap().get(path) {
                        let content_type = match extension {
                            Some("css") => "text/css",
                            Some("xml") => "application/xml",
                            _ => "text/html",
                        };

                        return Ok(Response::builder()
                            .header(header::CONTENT_TYPE, content_type)
                            .status(StatusCode::OK)
                            .body(full(content.to_owned()))
                            .unwrap());
                    }

                    // Check if the user forgot to add a trailing `/`.
                    if !path.ends_with('/') && extension.is_none() {
                        let path = format!("{path}/");
                        if SITE_CONTENT.read().unwrap().get(&path).is_some() {
                            return Ok(Response::builder()
                                .header(header::LOCATION, path)
                                .status(StatusCode::PERMANENT_REDIRECT)
                                .body(empty())
                                .unwrap());
                        }
                    }

                    let static_file_path = static_path.join(&path[1..]);
                    if let Ok(contents) = tokio::fs::read(&static_file_path).await {
                        return Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(
                                header::CONTENT_TYPE,
                                MimeGuess::from_path(static_file_path)
                                    .first_or_octet_stream()
                                    .essence_str(),
                            )
                            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                            .body(full(contents))
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

        let static_path: Arc<Path> = self.static_path.clone().into();
        let site = Arc::new(RwLock::new(self));

        {
            let mut site = site.write().unwrap();
            site.is_serving = true;
            site.load().unwrap();
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

            tokio::task::spawn({
                let static_path = static_path.clone();
                async move {
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |req| handle_request(req, static_path.clone())),
                        )
                        .await
                    {
                        eprintln!("Error serving connection: {err:?}");
                    }
                }
            });
        }
    }
}

pub struct SiteBuilder<State> {
    state: PhantomData<State>,
    root_path: PathBuf,
    base_url: String,
    title: Option<String>,
    include_drafts: bool,
    reading_speed: usize,
    templates: Templates,
    markdown_components: Box<dyn MarkdownComponents>,
    shortcodes: HashMap<String, Shortcode>,
    taxonomies: Vec<Taxonomy>,
    sass_path: Option<PathBuf>,
    sass_load_paths: Vec<PathBuf>,
}

impl<State> SiteBuilder<State> {
    fn coerce<NewState>(self) -> SiteBuilder<NewState> {
        SiteBuilder {
            state: PhantomData,
            root_path: self.root_path,
            base_url: self.base_url,
            title: self.title,
            include_drafts: self.include_drafts,
            reading_speed: self.reading_speed,
            templates: self.templates,
            markdown_components: self.markdown_components,
            shortcodes: self.shortcodes,
            taxonomies: self.taxonomies,
            sass_path: self.sass_path,
            sass_load_paths: self.sass_load_paths,
        }
    }

    fn build_site(self) -> Site {
        Site::from_params(BuildSiteParams {
            base_url: self.base_url,
            title: self.title,
            include_drafts: self.include_drafts,
            reading_speed: self.reading_speed,
            root_path: self.root_path,
            sass_path: self.sass_path,
            sass_load_paths: self.sass_load_paths,
            templates: self.templates,
            markdown_components: self.markdown_components,
            shortcodes: self.shortcodes,
            taxonomies: self.taxonomies,
        })
    }

    pub fn include_drafts(mut self, include_drafts: bool) -> Self {
        self.include_drafts = include_drafts;
        self
    }

    pub fn reading_speed(mut self, wpm: usize) -> Self {
        self.reading_speed = wpm;
        self
    }
}

impl SiteBuilder<()> {
    pub fn new() -> Self {
        Self {
            state: PhantomData,
            root_path: PathBuf::new(),
            base_url: String::new(),
            title: None,
            include_drafts: false,
            reading_speed: AVERAGE_ADULT_WPM,
            templates: Templates {
                index: Arc::new(|_| auk::div()),
                section: HashMap::new(),
                page: HashMap::new(),
                taxonomy: HashMap::new(),
                taxonomy_term: HashMap::new(),
                not_found: None,
            },
            markdown_components: Box::new(DefaultMarkdownComponents),
            shortcodes: HashMap::new(),
            taxonomies: Vec::new(),
            sass_path: None,
            sass_load_paths: Vec::new(),
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
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

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
                taxonomy: HashMap::new(),
                taxonomy_term: HashMap::new(),
                not_found: None,
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

    pub fn add_404_template(
        mut self,
        template: impl Fn(&BaseRenderContext) -> HtmlElement + Send + Sync + 'static,
    ) -> Self {
        self.templates.not_found = Some(Arc::new(template));
        self
    }

    pub fn with_markdown_components(
        mut self,
        markdown_components: impl MarkdownComponents + Send + Sync + 'static,
    ) -> Self {
        self.markdown_components = Box::new(markdown_components);
        self
    }

    pub fn add_shortcode(mut self, name: impl Into<String>, shortcode: Shortcode) -> Self {
        self.shortcodes.insert(name.into(), shortcode);
        self
    }

    pub fn add_taxonomy(
        mut self,
        taxonomy: Taxonomy,
        template: impl Fn(&RenderTaxonomyContext) -> HtmlElement + Send + Sync + 'static,
        term_template: impl Fn(&RenderTaxonomyTermContext) -> HtmlElement + Send + Sync + 'static,
    ) -> Self {
        self.templates
            .taxonomy
            .insert(taxonomy.name.clone(), Arc::new(template));
        self.templates
            .taxonomy_term
            .insert(taxonomy.name.clone(), Arc::new(term_template));
        self.taxonomies.push(taxonomy);
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
    pub fn add_sass_load_path(mut self, load_path: impl AsRef<Path>) -> Self {
        self.sass_load_paths.push(load_path.as_ref().to_owned());
        self
    }

    pub fn build(self) -> Site {
        self.build_site()
    }
}
