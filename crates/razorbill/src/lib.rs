#![doc = include_str!("../README.md")]

pub mod content;
pub mod markdown;
pub mod render;
mod site;
mod storage;
mod style;

pub use site::*;
pub use style::*;

#[cfg(test)]
mod tests {
    use auk::renderer::HtmlElementRenderer;
    use auk::*;
    use indoc::indoc;

    use super::markdown::*;

    #[test]
    fn test_kitchen_sink() {
        let text = indoc! {"
            # Homepage { #home .class1 .class2 }

            This is some Markdown content.
        "};

        let root_element = html().child(
            body().child(
                div().class("container").child(
                    div()
                        .class("content")
                        .children(markdown(text, MarkdownComponents::default())),
                ),
            ),
        );

        let rendered = HtmlElementRenderer::new()
            .render_to_string(&root_element)
            .unwrap();

        dbg!(rendered);
    }
}
