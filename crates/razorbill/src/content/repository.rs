use std::collections::HashMap;
use std::path::PathBuf;

use crate::content::{sort_pages_by, Page, Section};

/// A repository for the content of a site.
pub struct Repository {
    content_path: PathBuf,
    pub(crate) sections: HashMap<PathBuf, Section>,
    pub(crate) pages: HashMap<PathBuf, Page>,
}

impl Repository {
    /// Returns a new [`Repository`].
    pub fn new(content_path: PathBuf) -> Self {
        Self {
            content_path,
            sections: HashMap::new(),
            pages: HashMap::new(),
        }
    }

    /// Adds the given [`Section`] to the repository.
    pub fn add_section(&mut self, section: Section) {
        self.sections.insert(section.file.path.clone(), section);
    }

    /// Adds the given [`Page`] to the repository.
    pub fn add_page(&mut self, page: Page) {
        self.pages.insert(page.file.path.clone(), page);
    }

    /// Populates the contents of the repository.
    pub fn populate(&mut self) {
        let ancestors = self.build_ancestors();

        for (path, page) in self.pages.iter_mut() {
            let mut parent_section_path = page.file.parent.join("_index.md");

            while let Some(parent_section) = self.sections.get_mut(&parent_section_path) {
                let is_transparent = parent_section.meta.transparent;

                parent_section.pages.push(path.clone());

                page.ancestors = ancestors
                    .get(&parent_section_path)
                    .cloned()
                    .unwrap_or_default();
                page.ancestors.push(parent_section.file.path.clone());

                if page.meta.template.is_none() {
                    for ancestor in page.ancestors.iter().rev() {
                        let section = self.sections.get(ancestor).unwrap();
                        if let Some(template) = section.meta.page_template.as_ref() {
                            page.meta.template = Some(template.clone());
                            break;
                        }
                    }
                }

                if !is_transparent {
                    break;
                }

                match parent_section_path.clone().parent().unwrap().parent() {
                    Some(parent) => parent_section_path = parent.join("_index.md"),
                    None => break,
                }
            }
        }

        for (_path, section) in &mut self.sections {
            let pages = section
                .pages
                .iter()
                .map(|path| &self.pages[path])
                .collect::<Vec<_>>();

            let (sorted_pages, unsorted_pages) = match section.meta.sort_by.into() {
                Some(sort_by) => sort_pages_by(sort_by, pages),
                None => continue,
            };

            let mut reordered_pages = sorted_pages;
            reordered_pages.extend(unsorted_pages);

            section.pages = reordered_pages;
        }
    }

    fn build_ancestors(&self) -> HashMap<PathBuf, Vec<PathBuf>> {
        let mut ancestors = HashMap::new();

        for (_path, section) in &self.sections {
            if section.file.components.is_empty() {
                ancestors.insert(section.file.path.clone(), Vec::new());
                continue;
            }

            let mut current_path = self.content_path.clone();
            let mut section_ancestors = vec![current_path.join("_index.md")];
            for component in &section.file.components {
                current_path = current_path.join(component);
                if current_path == section.file.parent {
                    continue;
                }

                if let Some(ancestor) = self.sections.get(&current_path.join("_index.md")) {
                    section_ancestors.push(ancestor.file.path.clone());
                }
            }

            ancestors.insert(section.file.path.clone(), section_ancestors);
        }

        ancestors
    }
}
