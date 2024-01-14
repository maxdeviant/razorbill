use std::collections::{HashMap, VecDeque};

use auk::{Element, HtmlElement, WithChildren};
use pulldown_cmark::{
    self as md, Alignment, CodeBlockKind, CowStr, Event, HeadingLevel, LinkType, Tag,
};

pub struct MarkdownComponents {
    pub div: Box<dyn Fn() -> HtmlElement>,
    pub p: Box<dyn Fn() -> HtmlElement>,
    pub h1: Box<dyn Fn() -> HtmlElement>,
    pub h2: Box<dyn Fn() -> HtmlElement>,
    pub h3: Box<dyn Fn() -> HtmlElement>,
    pub h4: Box<dyn Fn() -> HtmlElement>,
    pub h5: Box<dyn Fn() -> HtmlElement>,
    pub h6: Box<dyn Fn() -> HtmlElement>,
    pub table: Box<dyn Fn() -> HtmlElement>,
    pub thead: Box<dyn Fn() -> HtmlElement>,
    pub tr: Box<dyn Fn() -> HtmlElement>,
    pub th: Box<dyn Fn() -> HtmlElement>,
    pub td: Box<dyn Fn() -> HtmlElement>,
    pub blockquote: Box<dyn Fn() -> HtmlElement>,
    pub pre: Box<dyn Fn() -> HtmlElement>,
    pub code: Box<dyn Fn() -> HtmlElement>,
    pub ol: Box<dyn Fn() -> HtmlElement>,
    pub ul: Box<dyn Fn() -> HtmlElement>,
    pub li: Box<dyn Fn() -> HtmlElement>,
    pub em: Box<dyn Fn() -> HtmlElement>,
    pub strong: Box<dyn Fn() -> HtmlElement>,
    pub del: Box<dyn Fn() -> HtmlElement>,
    pub a: Box<dyn Fn() -> HtmlElement>,
    pub img: Box<dyn Fn() -> HtmlElement>,
    pub br: Box<dyn Fn() -> HtmlElement>,
    pub hr: Box<dyn Fn() -> HtmlElement>,
    pub sup: Box<dyn Fn() -> HtmlElement>,
}

impl MarkdownComponents {
    #[inline(always)]
    pub fn div(&self) -> HtmlElement {
        (self.div)()
    }

    #[inline(always)]
    pub fn p(&self) -> HtmlElement {
        (self.p)()
    }

    #[inline(always)]
    pub fn h1(&self) -> HtmlElement {
        (self.h1)()
    }

    #[inline(always)]
    pub fn h2(&self) -> HtmlElement {
        (self.h2)()
    }

    #[inline(always)]
    pub fn h3(&self) -> HtmlElement {
        (self.h3)()
    }

    #[inline(always)]
    pub fn h4(&self) -> HtmlElement {
        (self.h4)()
    }

    #[inline(always)]
    pub fn h5(&self) -> HtmlElement {
        (self.h5)()
    }

    #[inline(always)]
    pub fn h6(&self) -> HtmlElement {
        (self.h6)()
    }

    #[inline(always)]
    pub fn table(&self) -> HtmlElement {
        (self.table)()
    }

    #[inline(always)]
    pub fn thead(&self) -> HtmlElement {
        (self.thead)()
    }

    #[inline(always)]
    pub fn tr(&self) -> HtmlElement {
        (self.tr)()
    }

    #[inline(always)]
    pub fn th(&self) -> HtmlElement {
        (self.th)()
    }

    #[inline(always)]
    pub fn td(&self) -> HtmlElement {
        (self.td)()
    }

    #[inline(always)]
    pub fn blockquote(&self) -> HtmlElement {
        (self.blockquote)()
    }

    #[inline(always)]
    pub fn pre(&self) -> HtmlElement {
        (self.pre)()
    }

    #[inline(always)]
    pub fn code(&self) -> HtmlElement {
        (self.code)()
    }

    #[inline(always)]
    pub fn ol(&self) -> HtmlElement {
        (self.ol)()
    }

    #[inline(always)]
    pub fn ul(&self) -> HtmlElement {
        (self.ul)()
    }

    #[inline(always)]
    pub fn li(&self) -> HtmlElement {
        (self.li)()
    }

    #[inline(always)]
    pub fn em(&self) -> HtmlElement {
        (self.em)()
    }

    #[inline(always)]
    pub fn strong(&self) -> HtmlElement {
        (self.strong)()
    }

    #[inline(always)]
    pub fn del(&self) -> HtmlElement {
        (self.del)()
    }

    #[inline(always)]
    pub fn a(&self) -> HtmlElement {
        (self.a)()
    }

    #[inline(always)]
    pub fn img(&self) -> HtmlElement {
        (self.img)()
    }

    #[inline(always)]
    pub fn br(&self) -> HtmlElement {
        (self.br)()
    }

    #[inline(always)]
    pub fn hr(&self) -> HtmlElement {
        (self.hr)()
    }

    #[inline(always)]
    pub fn sup(&self) -> HtmlElement {
        (self.sup)()
    }
}

impl Default for MarkdownComponents {
    fn default() -> Self {
        use auk::*;

        Self {
            div: Box::new(div),
            p: Box::new(p),
            h1: Box::new(h1),
            h2: Box::new(h2),
            h3: Box::new(h3),
            h4: Box::new(h4),
            h5: Box::new(h5),
            h6: Box::new(h6),
            table: Box::new(table),
            thead: Box::new(thead),
            tr: Box::new(tr),
            th: Box::new(th),
            td: Box::new(td),
            blockquote: Box::new(blockquote),
            pre: Box::new(pre),
            code: Box::new(code),
            ol: Box::new(ol),
            ul: Box::new(ul),
            li: Box::new(li),
            em: Box::new(em),
            strong: Box::new(strong),
            del: Box::new(del),
            a: Box::new(a),
            img: Box::new(img),
            br: Box::new(br),
            hr: Box::new(hr),
            sup: Box::new(sup),
        }
    }
}

pub fn markdown(text: &str, components: MarkdownComponents) -> Vec<Element> {
    let mut options = md::Options::empty();
    options.insert(md::Options::ENABLE_TABLES);
    options.insert(md::Options::ENABLE_FOOTNOTES);
    options.insert(md::Options::ENABLE_STRIKETHROUGH);
    options.insert(md::Options::ENABLE_TASKLISTS);
    options.insert(md::Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = md::Parser::new_ext(text, options);

    HtmlElementWriter::new(parser, components).run()
}

enum TableState {
    Head,
    Body,
}

fn escape_html(text: &str) -> String {
    // TODO: Should we be doing the escaping inside of `auk`?
    let mut escaped_text = String::with_capacity(text.len());
    md::escape::escape_html(&mut escaped_text, &text).unwrap();
    escaped_text
}

fn escape_href(href: &str) -> String {
    let mut escaped_href = String::with_capacity(href.len());
    md::escape::escape_href(&mut escaped_href, &href).unwrap();
    escaped_href
}

struct HtmlElementWriter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    input: I,
    components: MarkdownComponents,
    elements: Vec<Element>,
    current_element_stack: VecDeque<HtmlElement>,
    table_state: TableState,
    table_alignments: Vec<Alignment>,
    table_cell_index: usize,
    footnotes: HashMap<CowStr<'a>, usize>,
}

impl<'a, I> HtmlElementWriter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    pub fn new(input: I, components: MarkdownComponents) -> Self {
        Self {
            input,
            components,
            elements: Vec::new(),
            current_element_stack: VecDeque::new(),
            table_state: TableState::Head,
            table_alignments: Vec::new(),
            table_cell_index: 0,
            footnotes: HashMap::new(),
        }
    }

    fn run(mut self) -> Vec<Element> {
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
                        element.children_mut().push(escape_html(&text).into());
                    }
                }
                Event::Code(text) => {
                    self.write(self.components.code().child(escape_html(&text)));
                }
                Event::Html(html) => self.write_raw_html(&html),
                Event::SoftBreak => {
                    // TODO: Do we need to do anything here?
                }
                Event::HardBreak => self.write(self.components.br()),
                Event::Rule => self.write(self.components.hr()),
                Event::FootnoteReference(name) => {
                    let next_footnote_number = self.footnotes.len() + 1;
                    let number = *self
                        .footnotes
                        .entry(name.clone())
                        .or_insert(next_footnote_number);

                    self.write(
                        self.components.sup().class("footnote-reference").child(
                            self.components
                                .a()
                                .href(format!("#{}", escape_html(&name)))
                                .child(number.to_string()),
                        ),
                    );
                }
                Event::TaskListMarker(checked) => todo!(),
            }
        }

        self.elements
    }

    fn write(&mut self, element: HtmlElement) {
        if let Some(parent) = self.current_element_stack.back_mut() {
            parent.children_mut().push(element.into());
        } else {
            self.elements.push(element.into());
        }
    }

    fn write_raw_html(&mut self, html: &str) {
        if let Some(parent) = self.current_element_stack.back_mut() {
            parent.children_mut().push(html.into());
        } else {
            self.elements.push(html.into());
        }
    }

    fn push(&mut self, element: HtmlElement) {
        self.current_element_stack.push_back(element);
    }

    fn pop(&mut self) {
        if let Some(element) = self.current_element_stack.pop_back() {
            self.write(element);
        }
    }

    fn start_tag(&mut self, tag: Tag<'a>) {
        match tag {
            Tag::Paragraph => self.push(self.components.p()),
            Tag::Heading(level, id, classes) => {
                let heading = match level {
                    HeadingLevel::H1 => self.components.h1(),
                    HeadingLevel::H2 => self.components.h2(),
                    HeadingLevel::H3 => self.components.h3(),
                    HeadingLevel::H4 => self.components.h4(),
                    HeadingLevel::H5 => self.components.h5(),
                    HeadingLevel::H6 => self.components.h6(),
                };

                self.push(
                    heading.id::<String>(id.map(escape_html)).class::<String>(
                        Some(classes)
                            .filter(|classes| !classes.is_empty())
                            .map(|classes| {
                                classes
                                    .into_iter()
                                    .map(escape_html)
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            }),
                    ),
                )
            }
            Tag::Table(alignments) => {
                self.table_alignments = alignments;

                self.push(self.components.table())
            }
            Tag::TableHead => {
                self.table_state = TableState::Head;
                self.table_cell_index = 0;

                self.push(self.components.thead());
                self.push(self.components.tr());
            }
            Tag::TableRow => {
                self.table_cell_index = 0;

                self.push(self.components.tr());
            }
            Tag::TableCell => match self.table_state {
                TableState::Head => self.push(self.components.th()),
                TableState::Body => self.push(self.components.td()),
            },
            Tag::BlockQuote => self.push(self.components.blockquote()),
            Tag::CodeBlock(kind) => match kind {
                CodeBlockKind::Fenced(info) => {
                    self.push(self.components.pre());

                    let language = info.split(' ').next().unwrap();
                    if language.is_empty() {
                        self.push(self.components.code());
                    } else {
                        self.push(
                            self.components
                                .code()
                                .class(format!("language-{}", escape_html(language))),
                        );
                    }
                }
                CodeBlockKind::Indented => {
                    self.push(self.components.pre());
                    self.push(self.components.code());
                }
            },
            Tag::List(Some(1)) => self.push(self.components.ol()),
            Tag::List(Some(start)) => self.push(self.components.ol().start(start.to_string())),
            Tag::List(None) => self.push(self.components.ul()),
            Tag::Item => self.push(self.components.li()),
            Tag::Emphasis => self.push(self.components.em()),
            Tag::Strong => self.push(self.components.strong()),
            Tag::Strikethrough => self.push(self.components.del()),
            Tag::Link(LinkType::Email, dest, title) => self.push(
                self.components
                    .a()
                    .href(format!("mailto:{}", escape_href(&dest)))
                    .title::<String>(
                        Some(title)
                            .filter(|title| !title.is_empty())
                            .map(|title| escape_html(&title)),
                    ),
            ),
            Tag::Link(_link_type, dest, title) => self.push(
                self.components
                    .a()
                    .href(escape_href(&dest))
                    .title::<String>(
                        Some(title)
                            .filter(|title| !title.is_empty())
                            .map(|title| escape_html(&title)),
                    ),
            ),
            Tag::Image(_link_type, dest, title) => self.push(
                self.components
                    .img()
                    .src(escape_href(&dest))
                    // TODO: Add `alt` text.
                    .title::<String>(
                        Some(title)
                            .filter(|title| !title.is_empty())
                            .map(|title| escape_html(&title)),
                    ),
            ),
            Tag::FootnoteDefinition(name) => {
                let next_footnote_number = self.footnotes.len() + 1;
                let number = *self
                    .footnotes
                    .entry(name.clone())
                    .or_insert(next_footnote_number);

                self.push(
                    self.components
                        .div()
                        .class("footnote-definition")
                        .id(escape_html(&name)),
                );
                self.write(
                    self.components
                        .sup()
                        .class("footnote-definition-label")
                        .child(number.to_string()),
                );
            }
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

                self.table_state = TableState::Body;
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
            Tag::Image(_, _, _) => self.pop(),
            Tag::FootnoteDefinition(_) => {
                self.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use auk::renderer::HtmlElementRenderer;
    use indoc::indoc;

    use super::*;

    fn parse_and_render_markdown(text: &str) -> String {
        let elements = markdown(text, MarkdownComponents::default());

        elements
            .into_iter()
            .map(|element| match element {
                // TODO: Need to expose a way to retrieve the text from a `TextElement` in `auk`.
                Element::Text(element) => "".to_string(),
                Element::Html(element) => HtmlElementRenderer::new()
                    .render_to_string(&element)
                    .unwrap(),
            })
            .collect::<Vec<_>>()
            .join("")
    }

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

        insta::assert_yaml_snapshot!(parse_and_render_markdown(text));
    }

    #[test]
    fn test_markdown_link() {
        let text = indoc! {"
            Here is a [link](https://example.com) that you should click!
        "};

        insta::assert_yaml_snapshot!(parse_and_render_markdown(text));
    }

    #[test]
    fn test_markdown_list_with_inline_code() {
        let text = indoc! {"
            - `One`
            - `Two`
            - `Three`
        "};

        insta::assert_yaml_snapshot!(parse_and_render_markdown(text));
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

        insta::assert_yaml_snapshot!(parse_and_render_markdown(text));
    }

    #[test]
    fn test_markdown_footnotes() {
        let text = indoc! {"
            The quick[^1] brown fox jumped over the lazy[^2] dog.

            ---

            [^1]: The fox wasn't all that quick.

            [^2]: The dog wasn't all that lazy.
        "};

        insta::assert_yaml_snapshot!(parse_and_render_markdown(text));
    }
}
