use anyhow::Result;
use razorbill::content::Page;
use razorbill::markdown::{markdown, MarkdownComponents};
use razorbill::{html::*, Site};

fn main() -> Result<()> {
    let mut site = Site::builder()
        .root("examples/blog")
        .templates(
            || div(),
            |_section| div(),
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

        .text-center {
            text-align: center;
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

    html()
        .lang("en")
        .child(head().child(style().content(styles)))
        .child(
            body()
                .child(h1().class("heading text-center").content("Razorbill Blog"))
                .child(
                    h3().class("text-center")
                        .content(format!("path = {}", page.path)),
                )
                .child(div().class("content").children(children)),
        )
}

struct ProseProps {
    pub children: Vec<HtmlElement>,
}

fn prose(ProseProps { children }: ProseProps) -> HtmlElement {
    let styles = r#"
        body {
            background-color: papayawhip;
            color: palevioletred;
        }

        .heading {
            font-size: 5rem;
        }

        .text-center {
            text-align: center;
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

    html()
        .lang("en")
        .child(head().child(style().content(styles)))
        .child(
            body()
                .child(h1().class("heading text-center").content("Razorbill Blog"))
                .child(div().class("content").children(children)),
        )
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
