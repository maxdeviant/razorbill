mod shortcodes;

use auk_markdown::MarkdownComponents;

pub use shortcodes::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct DefaultMarkdownComponents;

impl DefaultMarkdownComponents {
    #[cfg(test)]
    pub fn boxed(self) -> Box<dyn MarkdownComponents> {
        Box::new(self)
    }
}

impl MarkdownComponents for DefaultMarkdownComponents {}
