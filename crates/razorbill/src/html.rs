use std::fmt::Write;

use indexmap::IndexMap;

#[derive(Debug)]
pub struct HtmlElement {
    pub tag_name: String,
    pub children: Vec<HtmlElement>,
    pub attrs: IndexMap<String, String>,
}

impl HtmlElement {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag_name: tag.into(),
            children: Vec::new(),
            attrs: IndexMap::new(),
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

        write!(&mut html, "<{}", self.tag_name)?;

        for (name, value) in &self.attrs {
            write!(&mut html, " ")?;
            write!(&mut html, r#"{name}="{value}""#)?;
        }

        write!(&mut html, ">")?;

        for child in &self.children {
            write!(&mut html, "{}", child.render_to_string()?)?;
        }

        write!(&mut html, "</{}>", self.tag_name)?;

        Ok(html)
    }
}

impl HtmlElement {
    pub fn id<V>(self, id: impl Into<Option<V>>) -> Self
    where
        V: Into<String>,
    {
        self.attr("id", id)
    }

    pub fn class<V>(self, class: impl Into<Option<V>>) -> Self
    where
        V: Into<String>,
    {
        self.attr("class", class)
    }

    pub fn title<V>(self, title: impl Into<Option<V>>) -> Self
    where
        V: Into<String>,
    {
        self.attr("title", title)
    }
}

pub fn div() -> HtmlElement {
    HtmlElement::new("div")
}

pub fn h1() -> HtmlElement {
    HtmlElement::new("h1")
}

pub fn h2() -> HtmlElement {
    HtmlElement::new("h2")
}

pub fn h3() -> HtmlElement {
    HtmlElement::new("h3")
}

pub fn h4() -> HtmlElement {
    HtmlElement::new("h4")
}

pub fn h5() -> HtmlElement {
    HtmlElement::new("h5")
}

pub fn h6() -> HtmlElement {
    HtmlElement::new("h6")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let element = div().class("container").child(h1().class("heading"));

        dbg!(element);
    }

    #[test]
    fn test_render() {
        let element = div()
            .class("outer")
            .child(div().class("inner").child(h1().class("heading")));

        dbg!(element.render_to_string().unwrap());
    }
}
