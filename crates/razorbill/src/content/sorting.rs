use std::cmp::Ordering;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::content::Page;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    /// Sort by date, in descending order (newest to oldest).
    Date,
}

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default, Serialize, Deserialize,
)]
#[serde(untagged, rename_all = "snake_case")]
pub enum MaybeSortBy {
    SortBy(SortBy),

    /// Don't sort.
    #[default]
    None,
}

impl From<MaybeSortBy> for Option<SortBy> {
    fn from(value: MaybeSortBy) -> Self {
        match value {
            MaybeSortBy::SortBy(sort_by) => Some(sort_by),
            MaybeSortBy::None => None,
        }
    }
}

pub fn sort_pages_by(sort_by: SortBy, pages: Vec<&Page>) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let (mut sortable, not_sortable): (Vec<&Page>, Vec<_>) =
        pages.iter().partition(|page| match sort_by {
            SortBy::Date => page.meta.date.is_some(),
        });

    sortable.sort_unstable_by(|a, b| {
        let ord = match sort_by {
            SortBy::Date => {
                let a_date = a.meta.date.as_ref().unwrap();
                let b_date = b.meta.date.as_ref().unwrap();

                b_date.cmp(&a_date)
            }
        };

        match ord {
            Ordering::Equal => a.path.0.cmp(&b.path.0),
            ord => ord,
        }
    });

    (
        sortable.iter().map(|page| page.file.path.clone()).collect(),
        not_sortable
            .iter()
            .map(|page| page.file.path.clone())
            .collect(),
    )
}
