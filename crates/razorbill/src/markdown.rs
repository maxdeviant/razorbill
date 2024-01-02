use pulldown_cmark as md;

pub fn markdown(text: &str) -> String {
    let mut options = md::Options::empty();
    options.insert(md::Options::ENABLE_TABLES);
    options.insert(md::Options::ENABLE_FOOTNOTES);
    options.insert(md::Options::ENABLE_STRIKETHROUGH);
    options.insert(md::Options::ENABLE_TASKLISTS);
    options.insert(md::Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = md::Parser::new_ext(text, options);

    let mut html_output = String::new();
    md::html::push_html(&mut html_output, parser);

    html_output
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn test_markdown() {
        let text = indoc! {"
            # Hello, world!

            ## This is

            ### A markdown document

            Here are some items:
            - Apple
            - Banana
            - Fruit

            And some ordered items:
            1. One
            1. Two
            1. Three
        "};

        dbg!(markdown(text));
    }
}
