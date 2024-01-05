use anyhow::Result;
use razorbill::content::{Page, Section};
use razorbill::markdown::{markdown, MarkdownComponents};
use razorbill::{html::*, Site};

fn main() -> Result<()> {
    let mut site = Site::builder()
        .root("examples/blog")
        .templates(
            || div(),
            |section| {
                crate::section(SectionProps {
                    section,
                    children: vec![],
                })
            },
            |page| {
                crate::page(PageProps {
                    page,
                    children: vec![post(PostProps {
                        text: &page.raw_content,
                    })],
                })
            },
        )
        .add_page_template("prose", |page| {
            prose(ProseProps {
                page,
                children: vec![post(PostProps {
                    text: &page.raw_content,
                })],
            })
        })
        .build();

    site.load()?;
    site.render()?;

    Ok(())
}

struct BasePageProps<'a> {
    pub title: &'a str,
    pub styles: Vec<&'a str>,
    pub children: Vec<HtmlElement>,
}

fn base_page(props: BasePageProps) -> HtmlElement {
    html()
        .lang("en")
        .child(
            head()
                .child(meta().charset("utf-8"))
                .child(meta().http_equiv("x-ua-compatible").content_("ie=edge"))
                .child(
                    meta()
                        .name("viewport")
                        .content_("width=device-width, initial-scale=1.0, maximum-scale=1"),
                )
                .child(title().content(props.title))
                .child(
                    link()
                        .rel("stylesheet")
                        .href("https://unpkg.com/tachyons@4.12.0/css/tachyons.min.css"),
                )
                .children(
                    props
                        .styles
                        .into_iter()
                        .map(|styles| style().content(styles)),
                ),
        )
        .children(props.children)
}

struct SectionProps<'a> {
    pub section: &'a Section,
    pub children: Vec<HtmlElement>,
}

fn section(SectionProps { section, children }: SectionProps) -> HtmlElement {
    let title = section
        .meta
        .title
        .clone()
        .unwrap_or(section.path.to_string());

    base_page(BasePageProps {
        title: &title,
        styles: vec![],
        children: vec![body()
            .child(h1().class("heading tc").content(&title))
            .child(div().class("content").children(children))],
    })
}

struct PageProps<'a> {
    pub page: &'a Page,
    pub children: Vec<HtmlElement>,
}

fn page(PageProps { page, children }: PageProps) -> HtmlElement {
    let styles = r#"
        body {
            background-color: darkslategray;
            color: #f4f4f4;
        }

        .heading {
            font-size: 5rem;
        }

        .content {
            max-width: 720px;
            margin: auto;
        }

        .post-paragraph {
            font-size: 1.2rem;
            line-height: 1.5rem;
        }
    "#;

    base_page(BasePageProps {
        title: page.meta.title.as_ref().unwrap_or(&page.slug),
        styles: vec![styles],
        children: vec![body()
            .child(h1().class("heading tc").content("Razorbill Blog"))
            .child(h3().class("tc").content(format!("path = {}", page.path)))
            .child(div().class("content").children(children))],
    })
}

struct ProseProps<'a> {
    pub page: &'a Page,
    pub children: Vec<HtmlElement>,
}

fn prose(ProseProps { page, children }: ProseProps) -> HtmlElement {
    let styles = r#"
        body {
            background-color: papayawhip;
            color: palevioletred;
        }

        .heading {
            font-size: 5rem;
        }

        .content {
            max-width: 720px;
            margin: auto;
        }

        .post-paragraph {
            font-size: 1.2rem;
            line-height: 1.5rem;
        }
    "#;

    base_page(BasePageProps {
        title: page.meta.title.as_ref().unwrap_or(&page.slug),
        styles: vec![styles],
        children: vec![body()
            .child(h1().class("heading tc").content("Razorbill Blog"))
            .child(div().class("content").children(children))],
    })
}

struct PostProps<'a> {
    pub text: &'a str,
}

fn post(PostProps { text }: PostProps) -> HtmlElement {
    div().children(markdown(
        &text,
        MarkdownComponents {
            p: Box::new(post_paragraph),
            ..Default::default()
        },
    ))
}

fn post_paragraph() -> HtmlElement {
    p().class("post-paragraph")
}
