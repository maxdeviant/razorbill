use anyhow::Result;
use auk::*;
use clap::{Parser, Subcommand};
use razorbill::markdown::{markdown, MarkdownComponents};
use razorbill::render::{PageToRender, RenderPageContext, RenderSectionContext};
use razorbill::{plumage, Site};

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
        .base_url("https://blog.example.com")
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
        .with_sass("sass")
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
    pub stylesheets: Vec<&'a str>,
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
                .child(title().child(text(props.title)))
                .child(
                    link()
                        .rel("stylesheet")
                        .href("https://unpkg.com/tachyons@4.12.0/css/tachyons.min.css"),
                )
                .children(
                    props
                        .stylesheets
                        .into_iter()
                        .map(|stylesheet| link().rel("stylesheet").href(stylesheet)),
                )
                .child(script().src("/livereload.js?port=35729")),
        )
        .children(props.children)
}

struct IndexProps<'a> {
    pub ctx: &'a RenderSectionContext<'a>,
}

fn index(IndexProps { ctx }: IndexProps) -> HtmlElement {
    base_page(BasePageProps {
        title: "Razorbill",
        stylesheets: vec!["/style.css"],
        children: vec![body()
            .child(
                h1().class(plumage().class("heading").text_center())
                    .child("Razorbill"),
            )
            .child(
                div()
                    .class("content")
                    .child(page_list(PageListProps {
                        heading: "Highlights",
                        pages: vec![ctx.get_page("@/posts/year-in-review.md").unwrap()],
                    }))
                    .child(page_list(PageListProps {
                        heading: "Posts",
                        pages: ctx.get_section("@/posts/_index.md").unwrap().pages,
                    })),
            )],
    })
}

struct PageListProps<'a> {
    pub heading: &'a str,
    pub pages: Vec<PageToRender<'a>>,
}

fn page_list(PageListProps { heading, pages }: PageListProps) -> HtmlElement {
    div()
        .child(h2().child(heading))
        .child(ul().children(pages.into_iter().map(|page| {
            li().child(
                a().href(page.path.to_string())
                    .child(page.title.as_ref().unwrap()),
            )
        })))
}

struct SectionProps<'a> {
    pub ctx: &'a RenderSectionContext<'a>,
}

fn section(SectionProps { ctx }: SectionProps) -> HtmlElement {
    let section = &ctx.section;

    let title = section.title.clone().unwrap_or(section.path.to_string());

    base_page(BasePageProps {
        title: &title,
        stylesheets: vec!["/style.css"],
        children: vec![body()
            .child(
                h1().class(plumage().class("heading").text_center())
                    .child(&title),
            )
            .child(
                div()
                    .class("content")
                    .children(section.pages.iter().map(|page| {
                        li().child(
                            a().href(page.path)
                                .child(page.title.clone().unwrap_or_default()),
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
    let page = &ctx.page;

    base_page(BasePageProps {
        title: page
            .title
            .as_ref()
            .map(|title| title.as_str())
            .unwrap_or(page.slug),
        stylesheets: vec!["/style.css"],
        children: vec![body()
            .child(
                h1().class(plumage().class("heading").text_center())
                    .child("Razorbill Blog"),
            )
            .children(page.date.as_ref().map(|date| {
                h3().class(plumage().text_center())
                    .child(format!("{}", date))
            }))
            .child(
                h3().class(plumage().text_center())
                    .child(format!("path = {}", page.path)),
            )
            .child(h4().class(plumage().text_center()).child(format!(
                "{} words | {} minutes",
                page.word_count.0, page.read_time.0
            )))
            .child(div().class("content").children(children))],
    })
}

struct ProseProps<'a> {
    pub ctx: &'a RenderPageContext<'a>,
    pub children: Vec<HtmlElement>,
}

fn prose(ProseProps { ctx, children }: ProseProps) -> HtmlElement {
    let page = &ctx.page;

    base_page(BasePageProps {
        title: page
            .title
            .as_ref()
            .map(|title| title.as_str())
            .unwrap_or(page.slug),
        stylesheets: vec!["/prose.css"],
        children: vec![body()
            .child(
                h1().class(plumage().class("heading").text_center())
                    .child("Razorbill Blog"),
            )
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
