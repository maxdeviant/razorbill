#![doc = include_str!("../README.md")]

pub mod content;
pub mod markdown;
pub mod render;
mod site;
mod storage;

pub use site::*;

#[cfg(test)]
mod tests {
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

        let rendered = root_element.render_to_string().unwrap();

        dbg!(rendered);
    }
}
