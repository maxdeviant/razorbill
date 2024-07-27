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
    input: &str,
    components: MarkdownComponents,
    shortcodes: HashMap<String, Shortcode>,
) -> Vec<Element> {
    let mut shortcode_calls = extract_shortcodes(&input);

    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;

    for call in &mut shortcode_calls {
        output.push_str(&input[cursor..call.span.start]);
        output.push_str(SHORTCODE_PLACEHOLDER);
        cursor = call.span.end;
        call.span = output.len() - SHORTCODE_PLACEHOLDER.len()..output.len();
    }

    output.push_str(&input[cursor..]);

    let mut elements = markdown(&output, components);
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
