use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use thiserror::Error;
use walkdir::WalkDir;

use crate::content::{Page, ParsePageError};
use crate::html::HtmlElement;

pub mod content;
pub mod html;
pub mod markdown;

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

pub struct Site {
    pub root_path: PathBuf,
    pub content_path: PathBuf,
    pub output_path: PathBuf,
    pub templates: HashMap<String, Box<dyn Fn(&Page) -> HtmlElement>>,
    pub pages: Vec<Page>,
}

impl Site {
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        let root_path = root_path.as_ref();

        Self {
            root_path: root_path.to_owned(),
            content_path: root_path.join("content"),
            output_path: root_path.join("public"),
            templates: HashMap::new(),
            pages: Vec::new(),
        }
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
        self.templates.insert(name.into(), Box::new(template));
    }

    pub fn render(&mut self) -> Result<(), RenderSiteError> {
        for page in &self.pages {
            let output_dir = self
                .output_path
                .join(PathBuf::from_str(&page.path.0.trim_start_matches("/")).unwrap());

            fs::create_dir_all(&output_dir)?;

            let output_path = output_dir.join("index.html");
            let mut output_file = File::create(&output_path)?;

            let page_template = self.templates.get("page").expect("no page template set");

            let rendered = page_template(page).render_to_string()?;

            output_file.write_all(rendered.as_bytes())?;

            println!("Wrote {:?}", output_path);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::html::*;
    use super::markdown::*;

    #[test]
    fn test_kitchen_sink() {
        let text = indoc! {"
            # Homepage { #home .class1 .class2 }

            This is some Markdown content.
        "};

        let root_element = html().child(
            body().child(
                div().class("container").child(
                    div()
                        .class("content")
                        .children(markdown(text, MarkdownComponents::default())),
                ),
            ),
        );

        let rendered = root_element.render_to_string().unwrap();

        dbg!(rendered);
    }
}
