use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Deserializer};

static TOML_REGEX: OnceLock<Regex> = OnceLock::new();

fn toml_regex() -> &'static Regex {
    &TOML_REGEX.get_or_init(|| {
        let pattern = r"^[[:space:]]*\+\+\+(\r?\n(?s).*?(?-s))\+\+\+[[:space:]]*(?:$|(?:\r?\n((?s).*(?-s))$))";
        Regex::new(pattern).expect("failed to compile regex for TOML front matter")
    })
}

#[derive(Debug)]
pub struct RawTomlFrontMatter<'a>(&'a str);

impl RawTomlFrontMatter<'_> {
    fn deserialize<T>(&self) -> Result<T, toml::de::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        toml::from_str(self.0)
    }
}

pub fn parse_front_matter<'a, T>(content: &'a str) -> Option<(T, &'a str)>
where
    T: serde::de::DeserializeOwned,
{
    if let Some(captures) = toml_regex().captures(content) {
        let front_matter = RawTomlFrontMatter(captures.get(1).unwrap().as_str());
        let content = captures.get(2).map_or("", |m| m.as_str());

        let front_matter: T = front_matter.deserialize().unwrap();

        Some((front_matter, content))
    } else {
        None
    }
}

pub fn from_toml_datetime<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use std::str::FromStr;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DatetimeOrString {
        Datetime(toml::value::Datetime),
        String(String),
    }

    match DatetimeOrString::deserialize(deserializer)? {
        DatetimeOrString::Datetime(datetime) => Ok(Some(datetime.to_string())),
        DatetimeOrString::String(string) => match toml::value::Datetime::from_str(&string) {
            Ok(datetime) => Ok(Some(datetime.to_string())),
            Err(err) => Err(D::Error::custom(err)),
        },
    }
}
