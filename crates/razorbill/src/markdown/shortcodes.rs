use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use auk::visitor::MutVisitor;
use auk::{Element, HtmlElement};

use crate::markdown::{markdown, MarkdownComponents};

const SHORTCODE_PLACEHOLDER: &str = "@@RAZORBILL_SHORTCODE@@";

pub type RenderShortcode = Arc<dyn Fn() -> HtmlElement + Send + Sync>;

pub struct Shortcode {
    pub render: RenderShortcode,
}

#[derive(Debug)]
pub struct ShortcodeCall {
    pub name: String,
    pub span: Range<usize>,
}

pub fn markdown_with_shortcodes(
    text: &str,
    components: MarkdownComponents,
    shortcodes: HashMap<String, Shortcode>,
) -> Vec<Element> {
    let mut text = text.to_string();

    let shortcode_calls = extract_shortcodes(&text);

    for call in &shortcode_calls {
        if let Some(_shortcode) = shortcodes.get(&call.name) {
            text.replace_range(call.span.clone(), SHORTCODE_PLACEHOLDER);
        } else {
            eprintln!("Unknown shortcode: '{}'", call.name);
        }
    }

    let mut elements = markdown(&text, components);
    let mut shortcode_replacer = ShortcodeReplacer {
        shortcodes,
        calls: shortcode_calls.into_iter(),
    };

    shortcode_replacer.visit_children(&mut elements).unwrap();

    elements
}

fn extract_shortcodes(text: &str) -> Vec<ShortcodeCall> {
    let regex = regex::Regex::new(r"\{\{\s*(\w+)\(\)\s*\}\}").unwrap();
    regex
        .captures_iter(text)
        .map(|captures| ShortcodeCall {
            name: captures[1].to_string(),
            span: captures.get(0).unwrap().start()..captures.get(0).unwrap().end(),
        })
        .collect()
}

struct ShortcodeReplacer {
    shortcodes: HashMap<String, Shortcode>,
    calls: std::vec::IntoIter<ShortcodeCall>,
}

impl MutVisitor for ShortcodeReplacer {
    type Error = ();

    fn visit_children(&mut self, children: &mut [Element]) -> Result<(), Self::Error> {
        for child in children {
            match child {
                Element::Text(element) => {
                    if element.text() == SHORTCODE_PLACEHOLDER {
                        let call = self.calls.next().unwrap();
                        let shortcode = self.shortcodes.get(&call.name).unwrap();

                        *child = (shortcode.render)().into();
                    }
                }
                Element::Html(element) => self.visit(element)?,
            }
        }

        Ok(())
    }
}
