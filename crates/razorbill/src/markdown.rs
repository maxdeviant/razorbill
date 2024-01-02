use std::collections::VecDeque;

use pulldown_cmark::{self as md, Alignment, CodeBlockKind, Event, HeadingLevel, LinkType, Tag};

use crate::html::{br, code, hr, HtmlElement};

pub fn markdown(text: &str) -> Vec<HtmlElement> {
    let mut options = md::Options::empty();
    options.insert(md::Options::ENABLE_TABLES);
    options.insert(md::Options::ENABLE_FOOTNOTES);
    options.insert(md::Options::ENABLE_STRIKETHROUGH);
    options.insert(md::Options::ENABLE_TASKLISTS);
    options.insert(md::Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = md::Parser::new_ext(text, options);

    HtmlElementWriter::new(parser).run()
}

enum TableState {
    Head,
    Body,
}

struct HtmlElementWriter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    input: I,
    elements: Vec<HtmlElement>,
    current_element_stack: VecDeque<HtmlElement>,
    table_state: TableState,
    table_alignments: Vec<Alignment>,
    table_cell_index: usize,
}

impl<'a, I> HtmlElementWriter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    pub fn new(input: I) -> Self {
        Self {
            input,
            elements: Vec::new(),
            current_element_stack: VecDeque::new(),
            table_state: TableState::Head,
            table_alignments: Vec::new(),
            table_cell_index: 0,
        }
    }

    fn run(mut self) -> Vec<HtmlElement> {
        while let Some(event) = self.input.next() {
            match event {
                Event::Start(tag) => {
                    self.start_tag(tag);
                }
                Event::End(tag) => {
                    self.end_tag(tag);
                }
                Event::Text(text) => {
                    if let Some(element) = self.current_element_stack.iter_mut().last() {
                        element.content = Some(text.to_string());
                    }
                }
                Event::Code(text) => self.push(code().content(text.to_string())),
                Event::Html(html) => todo!(),
                Event::SoftBreak => todo!(),
                Event::HardBreak => self.push(br()),
                Event::Rule => self.push(hr()),
                Event::FootnoteReference(_) => todo!(),
                Event::TaskListMarker(checked) => todo!(),
            }
        }

        self.elements
    }

    fn push(&mut self, element: HtmlElement) {
        self.current_element_stack.push_back(element);
    }

    fn pop(&mut self) {
        if let Some(element) = self.current_element_stack.pop_back() {
            if let Some(parent) = self.current_element_stack.back_mut() {
                parent.children.push(element);
            } else {
                self.elements.push(element);
            }
        }
    }

    fn start_tag(&mut self, tag: Tag) {
        use crate::html::*;

        match tag {
            Tag::Paragraph => self.push(p()),
            Tag::Heading(level, id, classes) => {
                let heading = match level {
                    HeadingLevel::H1 => h1(),
                    HeadingLevel::H2 => h2(),
                    HeadingLevel::H3 => h3(),
                    HeadingLevel::H4 => h4(),
                    HeadingLevel::H5 => h5(),
                    HeadingLevel::H6 => h6(),
                };

                self.push(
                    heading.id::<String>(id.map(Into::into)).class::<String>(
                        Some(classes)
                            .filter(|classes| !classes.is_empty())
                            .map(|classes| classes.join(" ")),
                    ),
                )
            }
            Tag::Table(alignments) => {
                self.table_alignments = alignments;

                self.push(table())
            }
            Tag::TableHead => {
                self.table_state = TableState::Head;
                self.table_cell_index = 0;

                self.push(thead());
                self.push(tr());
            }
            Tag::TableRow => {
                self.table_cell_index = 0;

                self.push(tr());
            }
            Tag::TableCell => match self.table_state {
                TableState::Head => self.push(th()),
                TableState::Body => self.push(td()),
            },
            Tag::BlockQuote => self.push(blockquote()),
            Tag::CodeBlock(kind) => match kind {
                CodeBlockKind::Indented => {
                    self.push(pre());
                    self.push(code());
                }
                CodeBlockKind::Fenced(info) => {
                    self.push(pre());
                    self.push(code());
                }
            },
            Tag::List(Some(1)) => self.push(ol()),
            Tag::List(Some(start)) => self.push(ol().start(start.to_string())),
            Tag::List(None) => self.push(ul()),
            Tag::Item => self.push(li()),
            Tag::Emphasis => self.push(em()),
            Tag::Strong => self.push(strong()),
            Tag::Strikethrough => self.push(del()),
            Tag::Link(LinkType::Email, dest, title) => self.push(
                a().href(format!("mailto:{}", dest)).title::<String>(
                    Some(title)
                        .filter(|title| !title.is_empty())
                        .map(|title| title.to_string()),
                ),
            ),
            Tag::Link(_link_type, dest, title) => self.push(
                a().href(dest.to_string()).title::<String>(
                    Some(title)
                        .filter(|title| !title.is_empty())
                        .map(|title| title.to_string()),
                ),
            ),
            Tag::Image(_link_type, dest, title) => self.push(
                img().src(dest.to_string()).title::<String>(
                    Some(title)
                        .filter(|title| !title.is_empty())
                        .map(|title| title.to_string()),
                ),
            ),
            Tag::FootnoteDefinition(_) => todo!(),
        }
    }

    fn end_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => self.pop(),
            Tag::Heading(_, _, _) => self.pop(),
            Tag::Table(_) => {
                self.pop();
                self.pop();
            }
            Tag::TableHead => {
                self.pop();
                self.pop();
            }
            Tag::TableRow => self.pop(),
            Tag::TableCell => {
                self.pop();

                self.table_cell_index += 1;
            }
            Tag::BlockQuote => self.pop(),
            Tag::CodeBlock(_) => {
                self.pop();
                self.pop();
            }
            Tag::List(_) => self.pop(),
            Tag::Item => self.pop(),
            Tag::Emphasis => self.pop(),
            Tag::Strong => self.pop(),
            Tag::Strikethrough => self.pop(),
            Tag::Link(_, _, _) => self.pop(),
            Tag::Image(_, _, _) => unreachable!(),
            Tag::FootnoteDefinition(_) => todo!(),
        }
    }
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

    #[test]
    fn test_markdown_table() {
        let text = indoc! {"
            # Table

            | Name | Value | Value 2 |
            | ---- | ----- | ------- |
            | A    | 1     | 17      |
            | B    | 2     | 25      |
            | C    | 3     | 32      |
        "};

        dbg!(markdown(text));
    }
}
