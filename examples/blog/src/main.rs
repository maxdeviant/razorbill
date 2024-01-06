use anyhow::Result;
use razorbill::markdown::{markdown, MarkdownComponents};
use razorbill::render::{PageToRender as Page, SectionToRender as Section};
use razorbill::{html::*, Site};

fn main() -> Result<()> {
    let mut site = Site::builder()
        .root("examples/blog")
        .templates(
            || div(),
            |section| crate::section(SectionProps { section }),
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
                .child(title().text_content(props.title))
                .child(
                    link()
                        .rel("stylesheet")
                        .href("https://unpkg.com/tachyons@4.12.0/css/tachyons.min.css"),
                )
                .children(
                    props
                        .styles
                        .into_iter()
                        .map(|styles| style().text_content(styles)),
                ),
        )
        .children(props.children)
}

struct SectionProps<'a> {
    pub section: &'a Section<'a>,
}

fn section(SectionProps { section }: SectionProps) -> HtmlElement {
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

        a {
            color: #fff;
        }
    "#;

    let title = section.title.clone().unwrap_or(section.path.to_string());

    base_page(BasePageProps {
        title: &title,
        styles: vec![styles],
        children: vec![body()
            .child(h1().class("heading tc").text_content(&title))
            .child(
                div()
                    .class("content")
                    .children(section.pages.iter().map(|page| {
                        li().child(
                            a().href(format!("..{}/index.html", page.path))
                                .text_content(page.title.clone().unwrap_or_default()),
                        )
                    })),
            )],
    })
}

struct PageProps<'a> {
    pub page: &'a Page<'a>,
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
        title: page
            .title
            .as_ref()
            .map(|title| title.as_str())
            .unwrap_or(page.slug),
        styles: vec![styles],
        children: vec![body()
            .child(h1().class("heading tc").text_content("Razorbill Blog"))
            .child(
                h3().class("tc")
                    .text_content(format!("path = {}", page.path)),
            )
            .child(div().class("content").children(children))],
    })
}

struct ProseProps<'a> {
    pub page: &'a Page<'a>,
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
        title: page
            .title
            .as_ref()
            .map(|title| title.as_str())
            .unwrap_or(page.slug),
        styles: vec![styles],
        children: vec![body()
            .child(h1().class("heading tc").text_content("Razorbill Blog"))
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
