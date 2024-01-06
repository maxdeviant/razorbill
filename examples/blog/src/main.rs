use anyhow::Result;
use auk::*;
use clap::{Parser, Subcommand};
use razorbill::markdown::{markdown, MarkdownComponents};
use razorbill::render::{RenderPageContext, RenderSectionContext};
use razorbill::Site;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Build,
    Serve,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut site = Site::builder()
        .root("examples/blog")
        .templates(
            |ctx| index(IndexProps { ctx }),
            |ctx| section(SectionProps { ctx }),
            |ctx| {
                page(PageProps {
                    ctx,
                    children: vec![post(PostProps {
                        text: &ctx.page.raw_content,
                    })],
                })
            },
        )
        .add_page_template("prose", |ctx| {
            prose(ProseProps {
                ctx,
                children: vec![post(PostProps {
                    text: &ctx.page.raw_content,
                })],
            })
        })
        .build();

    site.load()?;

    match cli.command {
        Command::Build => site.render()?,
        Command::Serve => site.serve().await?,
    }

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
                .child(meta().http_equiv("x-ua-compatible").content("ie=edge"))
                .child(
                    meta()
                        .name("viewport")
                        .content("width=device-width, initial-scale=1.0, maximum-scale=1"),
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
                )
                .child(script().src("/livereload.js?port=35729")),
        )
        .children(props.children)
}

struct IndexProps<'a> {
    pub ctx: &'a RenderSectionContext<'a>,
}

fn index(IndexProps { ctx }: IndexProps) -> HtmlElement {
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

    base_page(BasePageProps {
        title: "Razorbill",
        styles: vec![styles],
        children: vec![body()
            .child(h1().class("heading tc").text_content("Razorbill"))
            .child(
                div()
                    .class("content")
                    .child(
                        div()
                            .child(h2().text_content("Highlights"))
                            .child(ul().child({
                                let year_in_review =
                                    ctx.get_page("@/posts/year-in-review.md").unwrap();

                                li().child(
                                    a().href(year_in_review.path.to_string())
                                        .text_content(year_in_review.title.as_ref().unwrap()),
                                )
                            })),
                    )
                    .child(
                        div()
                            .child(h2().text_content("Posts"))
                            .child(ul().children({
                                let posts = ctx.get_section("@/posts/_index.md").unwrap();

                                posts.pages.into_iter().map(|page| {
                                    li().child(
                                        a().href(page.path.to_string())
                                            .text_content(page.title.as_ref().unwrap()),
                                    )
                                })
                            })),
                    ),
            )],
    })
}

struct SectionProps<'a> {
    pub ctx: &'a RenderSectionContext<'a>,
}

fn section(SectionProps { ctx }: SectionProps) -> HtmlElement {
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

    let section = &ctx.section;

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
                            a().href(page.path)
                                .text_content(page.title.clone().unwrap_or_default()),
                        )
                    })),
            )],
    })
}

struct PageProps<'a> {
    pub ctx: &'a RenderPageContext<'a>,
    pub children: Vec<HtmlElement>,
}

fn page(PageProps { ctx, children }: PageProps) -> HtmlElement {
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

    let page = &ctx.page;

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
    pub ctx: &'a RenderPageContext<'a>,
    pub children: Vec<HtmlElement>,
}

fn prose(ProseProps { ctx, children }: ProseProps) -> HtmlElement {
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

    let page = &ctx.page;

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
