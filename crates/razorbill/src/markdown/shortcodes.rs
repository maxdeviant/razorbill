mod parser;

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use auk::{Element, HtmlElement};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::markdown::shortcodes::parser::parse_document;
use crate::markdown::{markdown, MarkdownComponents, TableOfContents};

const SHORTCODE_PLACEHOLDER: &str = "@@RAZORBILL_SHORTCODE@@";

pub type RenderShortcode = Arc<dyn Fn(Map<String, Value>) -> Element + Send + Sync>;

pub struct Shortcode {
    pub render: RenderShortcode,
}

impl Shortcode {
    pub fn new<Args: DeserializeOwned>(
        render: impl Fn(Args) -> Element + Send + Sync + 'static,
    ) -> Self {
        Self {
            render: Arc::new(move |args| {
                let args = serde_json::from_value(Value::Object(args)).unwrap();
                render(args)
            }),
        }
    }

    pub fn new_thunk(render: impl Fn() -> Element + Send + Sync + 'static) -> Self {
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
    components: &Box<dyn MarkdownComponents>,
    shortcodes: &HashMap<String, Shortcode>,
) -> (Vec<Element>, TableOfContents) {
    let (output, shortcode_calls) = parse_document(input).unwrap();
    let (elements, table_of_contents) = markdown(&output, components);
    let elements = replace_shortcodes(elements, shortcodes, &mut shortcode_calls.into_iter());

    (elements, table_of_contents)
}

fn replace_shortcodes(
    elements: Vec<Element>,
    shortcodes: &HashMap<String, Shortcode>,
    calls: &mut std::vec::IntoIter<ShortcodeCall>,
) -> Vec<Element> {
    let mut new_elements = Vec::with_capacity(elements.len());

    for child in elements {
        match child {
            Element::Text(element) => {
                if element.text.contains(SHORTCODE_PLACEHOLDER) {
                    let mut text = element.text.as_str();

                    while let Some((before, after)) = text.split_once(SHORTCODE_PLACEHOLDER) {
                        new_elements.push(before.into());

                        let call = calls.next().unwrap();
                        let shortcode = shortcodes.get(&call.name).unwrap();

                        new_elements.push((shortcode.render)(call.args));

                        text = after;
                    }

                    if !text.is_empty() {
                        new_elements.push(text.into());
                    }
                } else {
                    new_elements.push(element.into());
                }
            }
            Element::Html(element) => {
                new_elements.push(
                    HtmlElement {
                        tag_name: element.tag_name,
                        attrs: element.attrs,
                        children: replace_shortcodes(element.children, shortcodes, calls),
                    }
                    .into(),
                );
            }
        }
    }

    new_elements
}

#[cfg(test)]
mod tests {
    use auk::renderer::HtmlElementRenderer;
    use auk::*;
    use indoc::indoc;
    use serde::Deserialize;

    use crate::markdown::DefaultMarkdownComponents;

    use super::*;

    fn parse_and_render_markdown_with_shortcodes(
        text: &str,
        shortcodes: HashMap<String, Shortcode>,
    ) -> String {
        let (elements, _table_of_contents) =
            markdown_with_shortcodes(text, &DefaultMarkdownComponents.boxed(), &shortcodes);

        elements
            .into_iter()
            .map(|element| match element {
                Element::Text(element) => element.text,
                Element::Html(element) => HtmlElementRenderer::new()
                    .render_to_string(&element)
                    .unwrap(),
            })
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn test_shortcodes_with_no_args() {
        let text = indoc! {"
            # Chinese Numbers

            1. {{ yi() }}
            1. {{ er() }}
            1. {{ san() }}
        "};

        let shortcodes = HashMap::from_iter([
            ("yi".into(), Shortcode::new_thunk(|| "一".into())),
            ("er".into(), Shortcode::new_thunk(|| "二".into())),
            ("san".into(), Shortcode::new_thunk(|| "三".into())),
        ]);

        insta::assert_yaml_snapshot!(parse_and_render_markdown_with_shortcodes(text, shortcodes));
    }

    #[test]
    fn test_shortcodes_with_args() {
        let text = indoc! {r#"
            {{ repeat(message="hey", times=1) }}
            {{ repeat(message="ho", times=3) }}
            {{ repeat(message="yo", times=5) }}

            Check out this video:

            {{ youtube(id="8o3i10OuMFQ")}}

            {{ youtube(id="8o3i10OuMFQ", autoplay=true) }}

            Did you watch {{ youtube(id="8o3i10OuMFQ", autoplay=true, class="youtube") }} yet?
        "#};

        #[derive(Deserialize)]
        struct RepeatArgs {
            message: String,
            times: usize,
        }

        #[derive(Deserialize)]
        struct YoutubeArgs {
            class: Option<String>,
            id: String,
            #[serde(default)]
            autoplay: bool,
        }

        let shortcodes = HashMap::from_iter([
            (
                "repeat".into(),
                Shortcode::new(|args: RepeatArgs| args.message.repeat(args.times).into()),
            ),
            (
                "youtube".into(),
                Shortcode::new(|args: YoutubeArgs| {
                    div()
                        .class::<String>(args.class)
                        .child(iframe().src(format!(
                            "https://youtube.com/embed/{id}{autoplay}",
                            id = args.id,
                            autoplay = if args.autoplay { "?autoplay=1" } else { "" }
                        )))
                        .into()
                }),
            ),
        ]);

        insta::assert_yaml_snapshot!(parse_and_render_markdown_with_shortcodes(text, shortcodes));
    }
}
