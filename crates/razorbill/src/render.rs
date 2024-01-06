use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::content::{Page, Section};

pub struct RenderSectionContext<'a> {
    pub(crate) content_path: &'a Path,
    pub(crate) pages: &'a HashMap<PathBuf, Page>,
    pub section: SectionToRender<'a>,
}

impl<'a> RenderSectionContext<'a> {
    pub fn get_page(&self, path: impl AsRef<Path>) -> Option<&'a Page> {
        let path = path.as_ref();
        let path = if path.starts_with("@/") {
            let mut new_path = self.content_path.to_owned();

            let mut components = path.components();
            components.next();

            for component in components {
                new_path.push(component);
            }

            new_path
        } else {
            path.to_owned()
        };

        self.pages.get(&path)
    }
}

pub struct SectionToRender<'a> {
    pub title: &'a Option<String>,
    pub path: &'a str,
    pub raw_content: &'a str,
    pub pages: Vec<PageToRender<'a>>,
}

impl<'a> SectionToRender<'a> {
    pub fn from_section(section: &'a Section, pages: &'a HashMap<PathBuf, Page>) -> Self {
        let pages = section
            .pages
            .iter()
            .map(|page| pages.get(page).unwrap())
            .map(PageToRender::from_page)
            .collect::<Vec<_>>();

        Self {
            title: &section.meta.title,
            path: &section.path.0,
            raw_content: &section.raw_content,
            pages,
        }
    }
}

pub struct RenderPageContext<'a> {
    pub page: PageToRender<'a>,
}

pub struct PageToRender<'a> {
    pub title: &'a Option<String>,
    pub slug: &'a str,
    pub path: &'a str,
    pub raw_content: &'a str,
}

impl<'a> PageToRender<'a> {
    pub fn from_page(page: &'a Page) -> Self {
        Self {
            title: &page.meta.title,
            slug: &page.slug,
            path: &page.path.0,
            raw_content: &page.raw_content,
        }
    }
}
