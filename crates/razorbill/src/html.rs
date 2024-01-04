use std::fmt::Write;

use indexmap::IndexMap;

#[derive(Debug)]
pub struct HtmlElement {
    pub tag_name: String,
    pub content: Option<String>,
    pub children: Vec<HtmlElement>,
    pub attrs: IndexMap<String, String>,
}

impl HtmlElement {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag_name: tag.into(),
            content: None,
            children: Vec::new(),
            attrs: IndexMap::new(),
        }
    }

    /// Returns whether this element is a [void element](https://developer.mozilla.org/en-US/docs/Glossary/Void_element).
    fn is_void(&self) -> bool {
        match self.tag_name.as_str() {
            "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input" | "link" | "meta"
            | "param" | "source" | "track" | "wbr" => true,
            _ => false,
        }
    }

    fn attr<V>(mut self, name: impl Into<String>, value: impl Into<Option<V>>) -> Self
    where
        V: Into<String>,
    {
        let name = name.into();
        match value.into() {
            Some(id) => {
                *self.attrs.entry(name).or_default() = id.into();
            }
            None => {
                self.attrs.remove(&name);
            }
        }

        self
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn child(mut self, child: HtmlElement) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = HtmlElement>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn render_to_string(&self) -> Result<String, std::fmt::Error> {
        let mut html = String::new();

        if self.tag_name == "html" {
            write!(&mut html, "<!DOCTYPE html>")?;
        }

        write!(&mut html, "<{}", self.tag_name)?;

        for (name, value) in &self.attrs {
            write!(&mut html, " ")?;
            write!(&mut html, r#"{name}="{value}""#)?;
        }

        write!(&mut html, ">")?;

        if self.is_void() {
            return Ok(html);
        }

        if let Some(content) = &self.content {
            write!(&mut html, "{}", content)?;
        }

        for child in &self.children {
            write!(&mut html, "{}", child.render_to_string()?)?;
        }

        write!(&mut html, "</{}>", self.tag_name)?;

        Ok(html)
    }
}

macro_rules! create_attribute_methods {
    ($($name:ident),*) => {
        $(
            pub fn $name<V>(self, value: impl Into<Option<V>>) -> Self
            where
                V: Into<String>,
            {
                self.attr(stringify!($name), value)
            }
        )*
    }
}

impl HtmlElement {
    create_attribute_methods!(
        charset, class, defer, href, id, lang, name, rel, role, src, start, style, tabindex, title,
        translate
    );

    pub fn async_<V>(self, value: impl Into<Option<V>>) -> Self
    where
        V: Into<String>,
    {
        self.attr("async", value)
    }

    pub fn content_<V>(self, value: impl Into<Option<V>>) -> Self
    where
        V: Into<String>,
    {
        self.attr("content", value)
    }

    pub fn http_equiv<V>(self, value: impl Into<Option<V>>) -> Self
    where
        V: Into<String>,
    {
        self.attr("http-equiv", value)
    }
}

macro_rules! html_elements {
    ($($name:ident),*) => {
        $(
            pub fn $name() -> HtmlElement {
                HtmlElement::new(stringify!($name))
            }
        )*
    }
}

html_elements!(
    a, blockquote, body, br, code, del, div, em, h1, h2, h3, h4, h5, h6, head, hr, html, img, li,
    link, meta, ol, p, pre, strong, style, table, td, th, thead, title, tr, ul
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let element = div()
            .class("container")
            .child(h1().class("heading").content("Hello, world!"));

        dbg!(element);
    }

    #[test]
    fn test_render() {
        let element = div().class("outer").child(
            div()
                .class("inner")
                .child(h1().class("heading").content("Hello, world!")),
        );

        dbg!(element.render_to_string().unwrap());
    }
}
