use std::collections::HashMap;
use std::path::PathBuf;

use crate::content::{Page, Section};

pub struct RenderSectionContext<'a> {
    pub section: SectionToRender<'a>,
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
