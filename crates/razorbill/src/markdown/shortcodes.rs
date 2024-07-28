mod parser;

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use auk::visitor::MutVisitor;
use auk::{Element, HtmlElement};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::markdown::shortcodes::parser::parse_document;
use crate::markdown::{markdown, MarkdownComponents};

const SHORTCODE_PLACEHOLDER: &str = "@@RAZORBILL_SHORTCODE@@";

pub type RenderShortcode = Arc<dyn Fn(Map<String, Value>) -> HtmlElement + Send + Sync>;

pub struct Shortcode {
    pub render: RenderShortcode,
}

impl Shortcode {
    pub fn new<Args: DeserializeOwned>(
        render: impl Fn(Args) -> HtmlElement + Send + Sync + 'static,
    ) -> Self {
        Self {
            render: Arc::new(move |args| {
                let args = serde_json::from_value(Value::Object(args)).unwrap();
                render(args)
            }),
        }
    }

    pub fn new_thunk(render: impl Fn() -> HtmlElement + Send + Sync + 'static) -> Self {
        Self {
            render: Arc::new(move |_args| render()),
        }
    }
}

#[derive(Debug)]
pub struct ShortcodeCall {
    pub name: String,
    pub args: Map<String, Value>,
    pub span: Range<usize>,
}

pub fn markdown_with_shortcodes(
    input: &str,
    components: MarkdownComponents,
    shortcodes: HashMap<String, Shortcode>,
) -> Vec<Element> {
    let (output, shortcode_calls) = parse_document(input).unwrap();

    let mut elements = markdown(&output, components);
    let mut shortcode_replacer = ShortcodeReplacer {
        shortcodes,
        calls: shortcode_calls.into_iter(),
    };

    shortcode_replacer.visit_children(&mut elements).unwrap();

    elements
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

                        *child = (shortcode.render)(call.args).into();
                    }
                }
                Element::Html(element) => self.visit(element)?,
            }
        }

        Ok(())
    }
}
