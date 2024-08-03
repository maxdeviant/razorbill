use std::path::PathBuf;

use crate::permalink::Permalink;

#[derive(Debug, Clone)]
pub struct Taxonomy {
    pub name: String,
}

/// A taxonomy term.
#[derive(Debug)]
pub struct Term {
    pub name: String,
    pub slug: String,
    pub permalink: Permalink,
    pub pages: Vec<PathBuf>,
}
