use std::fmt::Write;

use auk::visitor::Visitor;
use auk::*;
use chrono_tz::Tz;

use crate::content::Page;
use crate::date::format_date;
use crate::permalink::Permalink;
use crate::storage::Store;
use crate::{Site, SiteConfig};

pub fn render_feed(site: &Site, permalink: Permalink, pages: Vec<&Page>, storage: &impl Store) {
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
        .render_to_string(&atom_feed_template(&site.config, &permalink, pages))
        .unwrap();

    const XML_PROLOG: &str = r#"<?xml version="1.0" encoding="UTF-8"?>"#;

    let sitemap_xml = format!("{XML_PROLOG}\n{rendered}");

    storage.store_content(permalink, sitemap_xml).unwrap();
}

pub fn atom_feed_template(
    config: &SiteConfig,
    feed_url: &Permalink,
    pages: Vec<&Page>,
) -> HtmlElement {
    let last_updated_at = pages
        .iter()
        .filter_map(|page| page.meta.updated.as_ref())
        .chain(pages[0].meta.date.as_ref())
        .max()
        .unwrap();

    feed()
        .attr("xmlns", "http://www.w3.org/2005/Atom")
        .attr("xml:lang", "en")
        .child(title().child(config.title.clone().unwrap_or_default()))
        .child(
            link()
                .rel("self")
                .attr("type", "application/atom+xml")
                .href(feed_url.as_str()),
        )
        .child(
            link()
                .rel("alternate")
                .attr("type", "text/html")
                .href(&config.base_url),
        )
        .child(
            generator()
                .attr("uri", "https://github.com/maxdeviant/razorbill")
                .child("Razorbill"),
        )
        .child(updated().child(format_date(last_updated_at, "%+", Tz::UTC)))
        .child(id().child(feed_url.as_str()))
        .children(pages.into_iter().map(|page| {
            let date = page.meta.date.clone().unwrap();
            let updated_at = page.meta.updated.clone().unwrap_or(date.clone());

            // We're rendering the HTML with the `XmlRenderer` primarily so that
            // void elements (e.g., `img`, `hr`) get self-closing tags.
            let mut html_renderer = XmlRenderer::new();
            html_renderer.visit_children(&page.content).unwrap();
            let content_html = html_renderer.xml;

            entry()
                .attr("xml:lang", "en")
                .child(title().child(page.meta.title.clone().unwrap_or_default()))
                .child(published().child(format_date(&date, "%+", Tz::UTC)))
                .child(updated().child(format_date(&updated_at, "%+", Tz::UTC)))
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
                        .child(escape_xml(&content_html)),
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
    xml: String,
}

impl XmlRenderer {
    /// Returns a new [`XmlRenderer`].
    pub fn new() -> Self {
        Self { xml: String::new() }
    }

    /// Renders the given [`HtmlElement`] to a string of XML.
    pub fn render_to_string(mut self, element: &HtmlElement) -> Result<String, std::fmt::Error> {
        self.visit(element)?;

        Ok(self.xml)
    }
}

impl Visitor for XmlRenderer {
    type Error = std::fmt::Error;

    fn visit(&mut self, element: &HtmlElement) -> Result<(), Self::Error> {
        if element.tag_name == "html" {
            write!(&mut self.xml, "<!DOCTYPE html>")?;
        }

        write!(&mut self.xml, "<{}", element.tag_name)?;

        for (name, value) in &element.attrs {
            self.visit_attr(name, value)?;
        }

        if is_void(&element) {
            write!(&mut self.xml, "/>")?;
            return Ok(());
        } else {
            write!(&mut self.xml, ">")?;
        }

        self.visit_children(&element.children)?;

        write!(&mut self.xml, "</{}>", element.tag_name)?;

        Ok(())
    }

    fn visit_text(&mut self, text: &str) -> Result<(), Self::Error> {
        write!(&mut self.xml, "{}", text)?;

        Ok(())
    }

    fn visit_attr(&mut self, name: &str, value: &str) -> Result<(), Self::Error> {
        write!(&mut self.xml, " ")?;
        write!(&mut self.xml, "{name}")?;

        if !value.is_empty() {
            write!(&mut self.xml, r#"="{value}""#)?;
        }

        Ok(())
    }
}
