mod shortcodes;

use std::collections::{HashMap, VecDeque};

use auk::visitor::{noop_visit_element, MutVisitor};
use auk::{Element, HtmlElement, WithChildren};
use derive_more::{Deref, DerefMut};
use pulldown_cmark::{
    self as md, Alignment, CodeBlockKind, CowStr, Event, HeadingLevel, LinkType, Tag,
};

pub use shortcodes::*;
use slug::slugify;

pub struct MarkdownComponents {
    pub div: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub p: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub h1: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub h2: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub h3: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub h4: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub h5: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub h6: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub table: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub thead: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub tbody: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub tr: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub th: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub td: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub blockquote: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub pre: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub code: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub ol: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub ul: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub li: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub em: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub strong: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub del: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub a: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub img: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub br: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub hr: Box<dyn Fn() -> HtmlElement + Send + Sync>,
    pub sup: Box<dyn Fn() -> HtmlElement + Send + Sync>,
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
    pub fn tbody(&self) -> HtmlElement {
        (self.tbody)()
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
            tbody: Box::new(tbody),
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

pub fn markdown(text: &str, components: &MarkdownComponents) -> (Vec<Element>, TableOfContents) {
    let mut options = md::Options::empty();
    options.insert(md::Options::ENABLE_TABLES);
    options.insert(md::Options::ENABLE_FOOTNOTES);
    options.insert(md::Options::ENABLE_STRIKETHROUGH);
    options.insert(md::Options::ENABLE_TASKLISTS);
    options.insert(md::Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = md::Parser::new_ext(text, options);

    let mut elements = HtmlElementWriter::new(parser, components).run();

    let mut heading_identifier = HeadingIdentifier::new();
    heading_identifier.visit_children(&mut elements).unwrap();

    (
        elements,
        TableOfContents::from_headings(heading_identifier.headings),
    )
}

#[derive(Debug, Default, Deref, DerefMut)]
pub struct TableOfContents(Vec<Heading>);

impl TableOfContents {
    pub fn from_headings(headings: Vec<Heading>) -> Self {
        let mut table_of_contents = vec![];
        for heading in headings {
            if table_of_contents.is_empty()
                || !Self::insert_into_parent(table_of_contents.iter_mut().last(), &heading)
            {
                table_of_contents.push(heading);
            }
        }

        Self(table_of_contents)
    }

    fn insert_into_parent(parent: Option<&mut Heading>, heading: &Heading) -> bool {
        let Some(parent) = parent else {
            return false;
        };

        if heading.level <= parent.level {
            return false;
        }

        if heading.level + 1 == parent.level {
            parent.children.push(heading.clone());
            return true;
        }

        if !Self::insert_into_parent(parent.children.iter_mut().last(), heading) {
            parent.children.push(heading.clone());
        }

        true
    }
}

#[derive(Debug, Clone)]
pub struct Heading {
    pub level: u32,
    pub id: String,
    pub title: String,
    pub children: Vec<Heading>,
}

struct HeadingIdentifier {
    headings: Vec<Heading>,
    heading_id_counts: HashMap<String, usize>,
    inside_header: bool,
    title: Option<String>,
}

impl HeadingIdentifier {
    fn new() -> Self {
        Self {
            headings: Vec::new(),
            heading_id_counts: HashMap::new(),
            inside_header: false,
            title: None,
        }
    }
}

impl MutVisitor for HeadingIdentifier {
    type Error = ();

    fn visit(&mut self, element: &mut HtmlElement) -> Result<(), Self::Error> {
        match element.tag_name.as_str() {
            "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.inside_header = true;

                noop_visit_element(self, element)?;

                if let Some(title) = self.title.take() {
                    let mut id = slugify(
                        title
                            // HACK: Remove undesired remnants from escaping.
                            // We should figure out how to avoid escaping in the first place.
                            .replace("&quot;", ""),
                    );

                    let id_count = self.heading_id_counts.entry(id.clone()).or_insert(0);
                    if *id_count > 0 {
                        id.push_str("-");
                        id.push_str(&id_count.to_string());
                    }

                    *id_count += 1;

                    if element.attrs.get("id").is_none() {
                        element.attrs.insert("id".to_string(), id.clone());
                    }

                    self.headings.push(Heading {
                        level: match element.tag_name.as_str() {
                            "h2" => 2,
                            "h3" => 3,
                            "h4" => 4,
                            "h5" => 5,
                            "h6" => 6,
                            _ => unreachable!(),
                        },
                        id,
                        title,
                        children: Vec::new(),
                    });
                }

                self.inside_header = false;
            }
            _ => {}
        }

        noop_visit_element(self, element)
    }

    fn visit_text(&mut self, text: &mut String) -> Result<(), Self::Error> {
        if self.inside_header {
            let mut title = self.title.take().unwrap_or_default();
            title.push_str(&text);
            self.title = Some(title);
        }

        Ok(())
    }
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

fn escape_html_extended(text: &str) -> String {
    escape_html(text)
        .replace('\'', "&#x27;")
        .replace('/', "&#x2F;")
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
    components: &'a MarkdownComponents,
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
    pub fn new(input: I, components: &'a MarkdownComponents) -> Self {
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
                    let inside_pre = self
                        .current_element_stack
                        .iter()
                        .rfind(|element| element.tag_name == "pre")
                        .is_some();

                    if let Some(element) = self.current_element_stack.iter_mut().last() {
                        let text = if inside_pre {
                            escape_html_extended(&text)
                        } else {
                            escape_html(&text)
                        };

                        element.extend([text.into()]);
                    }

                    self.write_img_alt_text(&text);
                }
                Event::Code(text) => {
                    if !self.write_img_alt_text(&text) {
                        self.write(self.components.code().child(escape_html(&text)));
                    }
                }
                Event::Html(html) => self.write_raw_html(&html),
                Event::SoftBreak => {
                    self.write_raw_html("\n");
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
                Event::TaskListMarker(_checked) => todo!(),
            }
        }

        self.elements
    }

    fn write(&mut self, element: HtmlElement) {
        if let Some(parent) = self.current_element_stack.back_mut() {
            parent.extend([element.into()]);
        } else {
            self.elements.push(element.into());
        }
    }

    fn write_raw_html(&mut self, html: &str) {
        if let Some(parent) = self.current_element_stack.back_mut() {
            parent.extend([html.into()]);
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
                    let language = info.split(' ').next().unwrap();
                    if language.is_empty() {
                        self.push(self.components.pre());
                        self.push(self.components.code());
                    } else {
                        let language = escape_html(language);
                        let language_class = format!("language-{language}");

                        self.push(
                            self.components
                                .pre()
                                .attr("data-lang", &language)
                                .class(&language_class),
                        );
                        self.push(
                            self.components
                                .code()
                                .class(language_class)
                                .attr("data-lang", language),
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
                self.push(self.components.tbody());

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

    /// Writes the given text to the `alt` attribute of the deepest `img` tag.
    ///
    /// If there isn't an `img` tag on the element stack, will return `false`.
    fn write_img_alt_text(&mut self, text: &str) -> bool {
        let Some(image) = self
            .current_element_stack
            .iter_mut()
            .rfind(|element| element.tag_name == "img")
        else {
            return false;
        };

        image
            .attrs
            .entry("alt".to_string())
            .or_default()
            .push_str(&escape_html(&text));
        true
    }
}

#[cfg(test)]
mod tests {
    use auk::renderer::HtmlElementRenderer;
    use indoc::indoc;

    use super::*;

    fn parse_and_render_markdown(text: &str) -> String {
        let (elements, _table_of_contents) = markdown(text, &MarkdownComponents::default());

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
    fn test_markdown_image() {
        let text = indoc! {"
            Check out this cool image:

            ![very cool image](https://example.com/cool-image.png)

            This painting is beautiful:

            ![A photo of _Sunflowers_ by Van Gogh](https://example.com/sunflowers.png)

            Here's a picture of a less-than sign:

            ![A picture of a < sign](https://example.com/less-than.png)

            ![A screenshot of `ls` output](https://example.com/ls-output.png)
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
