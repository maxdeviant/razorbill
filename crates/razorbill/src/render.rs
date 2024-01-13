use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::content::{Page, ReadTime, Section, WordCount};

pub struct BaseRenderContext<'a> {
    pub(crate) content_path: &'a Path,
    pub(crate) sections: &'a HashMap<PathBuf, Section>,
    pub(crate) pages: &'a HashMap<PathBuf, Page>,
}

impl<'a> BaseRenderContext<'a> {
    pub fn get_section(&self, path: impl AsRef<Path>) -> Option<SectionToRender<'a>> {
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

        let section = self.sections.get(&path)?;

        Some(SectionToRender::from_section(section, &self.pages))
    }

    pub fn get_page(&self, path: impl AsRef<Path>) -> Option<PageToRender<'a>> {
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

        let page = self.pages.get(&path)?;

        Some(PageToRender::from_page(page))
    }
}

pub struct RenderSectionContext<'a> {
    pub(crate) base: BaseRenderContext<'a>,
    pub section: SectionToRender<'a>,
}

impl<'a> Deref for RenderSectionContext<'a> {
    type Target = BaseRenderContext<'a>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<'a> RenderSectionContext<'a> {}

pub struct SectionToRender<'a> {
    pub title: &'a Option<String>,
    pub path: &'a str,
    pub raw_content: &'a str,
    pub word_count: WordCount,
    pub read_time: ReadTime,
    pub extra: &'a toml::Table,
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
            word_count: section.word_count,
            read_time: section.read_time,
            extra: &section.meta.extra,
            pages,
        }
    }

    pub fn extra<'de, T>(&self) -> Result<T, toml::de::Error>
    where
        T: Deserialize<'de>,
    {
        T::deserialize(self.extra.clone())
    }
}

pub struct RenderPageContext<'a> {
    pub(crate) base: BaseRenderContext<'a>,
    pub page: PageToRender<'a>,
}

impl<'a> Deref for RenderPageContext<'a> {
    type Target = BaseRenderContext<'a>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

pub struct PageToRender<'a> {
    pub title: &'a Option<String>,
    pub slug: &'a str,
    pub path: &'a str,
    pub raw_content: &'a str,
    pub word_count: WordCount,
    pub read_time: ReadTime,
    pub extra: &'a toml::Table,
}

impl<'a> PageToRender<'a> {
    pub fn from_page(page: &'a Page) -> Self {
        Self {
            title: &page.meta.title,
            slug: &page.slug,
            path: &page.path.0,
            raw_content: &page.raw_content,
            word_count: page.word_count,
            read_time: page.read_time,
            extra: &page.meta.extra,
        }
    }

    pub fn extra<'de, T>(&self) -> Result<T, toml::de::Error>
    where
        T: Deserialize<'de>,
    {
        T::deserialize(self.extra.clone())
    }
}
