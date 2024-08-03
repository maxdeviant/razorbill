use std::fmt::Write;
use std::path::PathBuf;

use auk::renderer::HtmlElementRenderer;
use auk::visitor::Visitor;
use auk::*;

use crate::content::Page;
use crate::storage::Store;
use crate::{Site, SiteConfig};

pub fn render_feed(site: &Site, pages: Vec<&Page>, storage: &impl Store) {
    let mut pages = pages
        .into_iter()
        .filter(|page| page.meta.date.is_some())
        .collect::<Vec<_>>();

    pages.sort_unstable_by(|a, b| {
        b.meta
            .date
            .cmp(&a.meta.date)
            .then_with(|| a.permalink.cmp(&b.permalink))
    });

    let rendered = XmlRenderer::new()
        .render_to_string(&atom_feed_template(&site.config, pages))
        .unwrap();

    const XML_PROLOG: &str = r#"<?xml version="1.0" encoding="UTF-8"?>"#;

    let sitemap_xml = format!("{XML_PROLOG}\n{rendered}");

    storage
        .store_static_file(&PathBuf::from("atom.xml"), sitemap_xml)
        .unwrap();
}

pub fn atom_feed_template(config: &SiteConfig, pages: Vec<&Page>) -> HtmlElement {
    let feed_url = format!("{}/atom.xml", config.base_url);

    feed()
        .attr("xmlns", "http://www.w3.org/2005/Atom")
        .attr("xml:lang", "en")
        .child(title().child(config.title.clone().unwrap_or_default()))
        .child(
            link()
                .rel("self")
                .attr("type", "application/atom+xml")
                .href(&feed_url),
        )
        .child(
            generator()
                .attr("uri", "https://github.com/maxdeviant/razorbill")
                .child("Razorbill"),
        )
        .child(updated().child("Never"))
        .child(id().child(feed_url))
        .children(pages.into_iter().map(|page| {
            let mut html_renderer = HtmlElementRenderer::new();
            html_renderer.visit_children(&page.content).unwrap();
            let content_html = html_renderer.html();

            entry()
                .attr("xml:lang", "en")
                .child(title().child(page.meta.title.clone().unwrap_or_default()))
                .child(published().child(page.meta.date.clone().unwrap_or_default()))
                .child(updated().child(page.meta.updated.clone().unwrap_or_default()))
                .child(author().child(name().child("Unknown")))
                .child(
                    link()
                        .rel("alternate")
                        .attr("type", "text/html")
                        .href(page.permalink.as_str()),
                )
                .child(id().child(page.permalink.as_str()))
                .child(
                    content()
                        .attr("type", "html")
                        .attr("xml:base", page.permalink.as_str())
                        .child(escape_xml(content_html)),
                )
        }))
}

fn escape_xml(content: &str) -> String {
    content
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('/', "&#x2F;")
}

fn feed() -> HtmlElement {
    HtmlElement::new("feed")
}

fn generator() -> HtmlElement {
    HtmlElement::new("generator")
}

fn updated() -> HtmlElement {
    HtmlElement::new("updated")
}

fn id() -> HtmlElement {
    HtmlElement::new("id")
}

fn entry() -> HtmlElement {
    HtmlElement::new("entry")
}

fn published() -> HtmlElement {
    HtmlElement::new("published")
}

fn author() -> HtmlElement {
    HtmlElement::new("author")
}

fn name() -> HtmlElement {
    HtmlElement::new("name")
}

fn content() -> HtmlElement {
    HtmlElement::new("content")
}

fn is_void(element: &HtmlElement) -> bool {
    match element.tag_name.as_str() {
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta"
        | "param" | "source" | "track" | "wbr" => true,
        _ => false,
    }
}

/// A renderer for [`HtmlElement`]s to a string of XML.
pub struct XmlRenderer {
    html: String,
}

impl XmlRenderer {
    /// Returns a new [`XmlRenderer`].
    pub fn new() -> Self {
        Self {
            html: String::new(),
        }
    }

    /// Renders the given [`HtmlElement`] to a string of XML.
    pub fn render_to_string(mut self, element: &HtmlElement) -> Result<String, std::fmt::Error> {
        self.visit(element)?;

        Ok(self.html)
    }
}

impl Visitor for XmlRenderer {
    type Error = std::fmt::Error;

    fn visit(&mut self, element: &HtmlElement) -> Result<(), Self::Error> {
        if element.tag_name == "html" {
            write!(&mut self.html, "<!DOCTYPE html>")?;
        }

        write!(&mut self.html, "<{}", element.tag_name)?;

        for (name, value) in &element.attrs {
            self.visit_attr(name, value)?;
        }

        if is_void(&element) {
            write!(&mut self.html, "/>")?;
            return Ok(());
        } else {
            write!(&mut self.html, ">")?;
        }

        self.visit_children(&element.children)?;

        write!(&mut self.html, "</{}>", element.tag_name)?;

        Ok(())
    }

    fn visit_text(&mut self, text: &str) -> Result<(), Self::Error> {
        write!(&mut self.html, "{}", text)?;

        Ok(())
    }

    fn visit_attr(&mut self, name: &str, value: &str) -> Result<(), Self::Error> {
        write!(&mut self.html, " ")?;
        write!(&mut self.html, "{name}")?;

        if !value.is_empty() {
            write!(&mut self.html, r#"="{value}""#)?;
        }

        Ok(())
    }
}
