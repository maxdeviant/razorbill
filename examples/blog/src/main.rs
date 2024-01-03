use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use razorbill::markdown::{markdown, MarkdownComponents};
use razorbill::{html::*, Site};

fn main() -> Result<()> {
    let mut site = Site::new("examples/blog");

    site.load()?;

    fs::create_dir_all("examples/blog/public")?;

    for p in site.pages {
        let rendered = page(PageProps {
            children: vec![post(PostProps {
                text: p.raw_content,
            })],
        })
        .render_to_string()?;

        let filepath =
            PathBuf::from_iter(["examples", "blog", "public", &format!("{}.html", p.slug)]);

        let mut out_file = File::create(&filepath)?;
        out_file.write_all(rendered.as_bytes())?;

        println!("Wrote {:?}", filepath);
    }

    Ok(())
}

struct PageProps {
    pub children: Vec<HtmlElement>,
}

fn page(PageProps { children }: PageProps) -> HtmlElement {
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
                .child(div().class("content").children(children)),
        )
}

struct PostProps {
    pub text: String,
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
