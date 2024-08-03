use std::path::PathBuf;

use crate::permalink::Permalink;

#[derive(Debug, Clone)]
pub struct Taxonomy {
    pub name: String,
}

/// A taxonomy term.
#[derive(Debug)]
pub struct TaxonomyTerm {
    pub name: String,
    pub permalink: Permalink,
    pub pages: Vec<PathBuf>,
}
