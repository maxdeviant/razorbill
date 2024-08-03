use std::str::FromStr;

use url::Url;

use crate::SiteConfig;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Permalink(Url);

impl Permalink {
    pub fn from_path(config: &SiteConfig, path: &str) -> Self {
        // HACK: We probably need to deal with this elsewhere.
        let path = path.trim_end_matches("_index");

        let suffix = if path.ends_with('/') || path.is_empty() {
            ""
        } else {
            "/"
        };
        let base_url = config.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');

        Self(Url::from_str(&format!("{base_url}/{path}{suffix}")).unwrap())
    }

    pub fn as_str(&self) -> &str {
        &self.0.as_str()
    }

    pub fn path(&self) -> &str {
        &self.0.path()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn make_config(base_url: &str) -> SiteConfig {
        SiteConfig {
            base_url: base_url.to_string(),
            taxonomies: Vec::new(),
        }
    }

    #[test]
    fn test_permalink() {
        assert_eq!(
            Permalink::from_path(&make_config("https://example.com/"), "/"),
            Permalink("https://example.com/".parse().unwrap())
        );
        assert_eq!(
            Permalink::from_path(&make_config("https://example.com"), "/"),
            Permalink("https://example.com/".parse().unwrap())
        );
        assert_eq!(
            Permalink::from_path(&make_config("https://example.com"), ""),
            Permalink("https://example.com/".parse().unwrap())
        );
    }

    #[test]
    fn test_permalink_path() {
        let permalink = Permalink("https://example.com/this/is/a/cool/site/".parse().unwrap());
        assert_eq!(permalink.path(), "/this/is/a/cool/site/");
    }
}
