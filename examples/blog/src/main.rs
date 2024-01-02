use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use razorbill::html::*;
use razorbill::markdown::{markdown, MarkdownComponents};

struct Post {
    pub slug: String,
    pub text: String,
}

fn main() -> Result<()> {
    let mut posts = Vec::new();

    for entry in fs::read_dir("examples/blog/content/posts")? {
        let entry = entry?;

        let filename = entry
            .path()
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let text = fs::read_to_string(entry.path())?;

        posts.push(Post {
            slug: filename,
            text,
        });
    }

    fs::create_dir_all("examples/blog/public")?;

    for p in posts {
        let rendered = page(PageProps {
            children: vec![post(PostProps { text: p.text })],
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
