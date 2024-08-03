use std::collections::HashMap;
use std::path::PathBuf;

use crate::content::{sort_pages_by, Page, Pages, Section, Sections, SortBy, Taxonomy};

pub struct ContentAggregator {
    content_path: PathBuf,
    sections: Sections,
    pages: Pages,
    taxonomies: HashMap<String, HashMap<String, Vec<PathBuf>>>,
}

impl ContentAggregator {
    /// Returns a new [`ContentAggregator`].
    pub fn new(content_path: PathBuf, taxonomy_definitions: Vec<Taxonomy>) -> Self {
        let mut taxonomies = HashMap::new();

        for taxonomy in taxonomy_definitions {
            taxonomies.insert(taxonomy.name.clone(), HashMap::new());
        }

        Self {
            content_path,
            sections: Sections::default(),
            pages: Pages::default(),
            taxonomies,
        }
    }

    /// Adds the given [`Section`] to the aggregate.
    pub fn add_section(&mut self, section: Section) {
        self.sections.insert(section.file.path.clone(), section);
    }

    /// Adds the given [`Page`] to the aggregate.
    pub fn add_page(&mut self, page: Page) {
        for (taxonomy_name, terms) in &page.meta.taxonomies {
            let Some(pages_by_term) = self.taxonomies.get_mut(taxonomy_name) else {
                continue;
            };

            for term in terms {
                let pages = pages_by_term.entry(term.clone()).or_default();
                pages.push(page.file.path.clone());
            }
        }

        self.pages.insert(page.file.path.clone(), page);
    }

    /// Aggregates and returns all of the sections, pages, and taxonomies in the aggregate.
    pub fn aggregate(
        mut self,
    ) -> (
        Sections,
        Pages,
        HashMap<String, HashMap<String, Vec<PathBuf>>>,
    ) {
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

        for (_path, section) in self.sections.iter_mut() {
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

        for (_taxonomy, pages_by_term) in self.taxonomies.iter_mut() {
            for (_term, page_paths) in pages_by_term {
                let pages = page_paths
                    .iter()
                    .map(|page| self.pages.get(page).unwrap())
                    .collect::<Vec<_>>();

                let (sorted_pages, unsorted_pages) = sort_pages_by(SortBy::Date, pages);

                let mut reordered_pages = sorted_pages;
                reordered_pages.extend(unsorted_pages);

                *page_paths = reordered_pages;
            }
        }

        (self.sections, self.pages, self.taxonomies)
    }

    fn build_ancestors(&self) -> HashMap<PathBuf, Vec<PathBuf>> {
        let mut ancestors = HashMap::new();

        for (_path, section) in self.sections.iter() {
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::content::{
        FileInfo, MaybeSortBy, PageFrontMatter, PagePath, ReadTime, SectionFrontMatter,
        SectionPath, SortBy, WordCount,
    };
    use crate::permalink::Permalink;
    use crate::SiteConfig;

    use super::*;

    fn make_section(filepath: &str, sort_by: MaybeSortBy) -> Section {
        let config = SiteConfig {
            base_url: "https://example.com".to_string(),
            title: None,
            taxonomies: Vec::new(),
        };

        let root_path = PathBuf::new();
        let file = FileInfo::new(&root_path, filepath);
        let path = SectionPath::from_file_path(root_path, &file.path).unwrap();

        Section {
            meta: SectionFrontMatter {
                sort_by,
                ..Default::default()
            },
            permalink: Permalink::from_path(&config, path.0.as_str()),
            path,
            file,
            raw_content: String::new(),
            content: Vec::new(),
            word_count: WordCount(0),
            read_time: ReadTime(0),
            pages: Vec::new(),
        }
    }

    fn make_page(filepath: &str, date: &str) -> Page {
        let config = SiteConfig {
            base_url: "https://example.com".to_string(),
            title: None,
            taxonomies: Vec::new(),
        };

        let root_path = PathBuf::new();
        let file = FileInfo::new(&root_path, filepath);
        let path = PagePath::from_file_path(root_path, &file.path).unwrap();

        Page {
            meta: PageFrontMatter {
                date: Some(date.to_string()),
                ..Default::default()
            },
            permalink: Permalink::from_path(&config, path.0.as_str()),
            path,
            file,
            ancestors: Vec::new(),
            slug: String::new(),
            raw_content: String::new(),
            content: Vec::new(),
            word_count: WordCount(0),
            read_time: ReadTime(0),
        }
    }

    #[test]
    fn test_aggregate() {
        let mut aggregator = ContentAggregator::new(PathBuf::from("content"), Vec::new());

        let sections = vec![
            ("content/_index.md", MaybeSortBy::None),
            ("content/blog/_index.md", MaybeSortBy::SortBy(SortBy::Date)),
        ];
        let pages = vec![
            ("content/blog/2023-07-01-hello-world.md", "2023-07-01"),
            ("content/blog/2023-12-31-year-in-review.md", "2023-12-31"),
            ("content/blog/2024-01-01-happy-new-year.md", "2024-01-01"),
        ];

        for (filepath, sort_by) in sections {
            aggregator.add_section(make_section(filepath, sort_by))
        }

        for (filepath, date) in pages {
            aggregator.add_page(make_page(filepath, date));
        }

        let (sections, pages, _taxonomies) = aggregator.aggregate();

        let blog_section = sections
            .get(&PathBuf::from("content/blog/_index.md"))
            .unwrap();
        assert_eq!(
            blog_section.pages,
            vec![
                PathBuf::from("content/blog/2024-01-01-happy-new-year.md"),
                PathBuf::from("content/blog/2023-12-31-year-in-review.md"),
                PathBuf::from("content/blog/2023-07-01-hello-world.md"),
            ]
        );

        let hello_world_page = pages
            .get(&PathBuf::from("content/blog/2023-07-01-hello-world.md"))
            .unwrap();
        assert_eq!(
            hello_world_page.ancestors,
            vec![
                PathBuf::from("content/_index.md"),
                PathBuf::from("content/blog/_index.md")
            ]
        );
    }
}
