use std::collections::HashSet;
use std::path::PathBuf;

use auk::renderer::HtmlElementRenderer;
use auk::*;

use crate::permalink::Permalink;
use crate::storage::Store;
use crate::Site;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SitemapEntry {
    pub permalink: Permalink,
    pub updated_at: Option<String>,
}

pub fn render_sitemap(site: &Site, storage: &impl Store) {
    let mut entries = HashSet::new();

    for section in site.sections.values() {
        entries.insert(SitemapEntry {
            permalink: section.permalink.clone(),
            updated_at: None,
        });
    }

    for page in site.pages.values() {
        entries.insert(SitemapEntry {
            permalink: page.permalink.clone(),
            updated_at: page
                .meta
                .updated
                .as_ref()
                .or(page.meta.date.as_ref())
                .cloned(),
        });
    }

    let mut entries = entries.into_iter().collect::<Vec<_>>();
    entries.sort();

    let rendered = HtmlElementRenderer::new()
        .render_to_string(&sitemap_template(entries))
        .unwrap();

    const XML_PROLOG: &str = r#"<?xml version="1.0" encoding="UTF-8"?>"#;

    let sitemap_xml = format!("{XML_PROLOG}\n{rendered}");

    storage
        .store_static_file(&PathBuf::from("sitemap.xml"), sitemap_xml)
        .unwrap();
}

pub fn sitemap_template(entries: Vec<SitemapEntry>) -> HtmlElement {
    urlset()
        .attr("xmlns", "http://www.sitemaps.org/schemas/sitemap/0.9")
        .children(entries.into_iter().map(|entry| {
            url().child(loc().child(entry.permalink.as_str())).children(
                entry
                    .updated_at
                    .as_ref()
                    .map(|updated_at| lastmod().child(updated_at)),
            )
        }))
}

fn urlset() -> HtmlElement {
    HtmlElement::new("urlset")
}

fn url() -> HtmlElement {
    HtmlElement::new("url")
}

fn loc() -> HtmlElement {
    HtmlElement::new("loc")
}

fn lastmod() -> HtmlElement {
    HtmlElement::new("lastmod")
}

// <?xml version="1.0" encoding="UTF-8"?>
// <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
//     {%- for sitemap_entry in entries %}
//     <url>
//         <loc>{{ sitemap_entry.permalink | escape_xml | safe }}</loc>
//         {%- if sitemap_entry.updated %}
//         <lastmod>{{ sitemap_entry.updated }}</lastmod>
//         {%- endif %}
//     </url>
//     {%- endfor %}
// </urlset>
