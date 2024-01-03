use std::path::{Path, PathBuf};

use thiserror::Error;
use walkdir::WalkDir;

use crate::content::{Page, ParsePageError};

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

pub struct Site {
    pub root_path: PathBuf,
    pub content_path: PathBuf,
    pub pages: Vec<Page>,
}

impl Site {
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        let root_path = root_path.as_ref();

        Self {
            root_path: root_path.to_owned(),
            content_path: root_path.join("content"),
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
                pages.push(Page::from_path(path)?);

                continue;
            }
        }

        self.pages = pages;

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
