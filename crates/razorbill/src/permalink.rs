use crate::SiteConfig;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Permalink(String);

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

        Self(format!("{base_url}/{path}{suffix}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn make_config(base_url: &str) -> SiteConfig {
        SiteConfig {
            base_url: base_url.to_string(),
        }
    }

    #[test]
    fn test_permalink() {
        assert_eq!(
            Permalink::from_path(&make_config("https://example.com/"), "/"),
            Permalink("https://example.com/".into())
        );
        assert_eq!(
            Permalink::from_path(&make_config("https://example.com"), "/"),
            Permalink("https://example.com/".into())
        );
        assert_eq!(
            Permalink::from_path(&make_config("https://example.com"), ""),
            Permalink("https://example.com/".into())
        );
    }
}
