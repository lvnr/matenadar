//! Figure numbers: which figures the design numbers, and their numbers, in
//! the book's order. Print and ebook number them alike from this.

use crate::block::{Block, Figure};
use crate::walk::{Node, walk};
use crate::{Document, Item};

/// A numbered figure: its number (`3.2`) and label (`Figure 3.2`).
pub struct Numbered<'a> {
    pub figure: &'a Figure,
    pub number: String,
    pub label: String,
}

/// Whether a figure gets a number, when the design numbers figures: one
/// with a caption does, unless it says `numbered: false`.
pub fn is_numbered(f: &Figure) -> bool {
    let has_caption = f.caption.as_deref().is_some_and(|c| !c.trim().is_empty()) || !f.caption_content.is_empty();
    f.numbered.unwrap_or(has_caption)
}

/// Every numbered figure of the document, in order. Counting by chapter,
/// the count restarts in each section, after its `chapter` var when it has
/// one.
pub fn numbered(doc: &Document) -> Vec<Numbered<'_>> {
    let Some(figs) = &doc.design.figures else { return Vec::new() };
    let by_chapter = figs.per.as_deref() == Some("chapter");
    let word = figs.label.as_deref().unwrap_or("Figure");
    let mut out = Vec::new();
    let mut n = 0u32;
    let mut chapter: Option<String> = None;
    walk(doc, &mut |_, node| match node {
        Node::Item(Item::Section(s)) if by_chapter => {
            n = 0;
            chapter = s.vars.get("chapter").cloned();
        }
        Node::Block(Block::Figure(f)) if is_numbered(f) => {
            n += 1;
            let number = match (&chapter, by_chapter) {
                (Some(ch), true) => format!("{ch}.{n}"),
                _ => n.to_string(),
            };
            // A no-break space: "Fig. 2" never splits across lines.
            let label = format!("{word}\u{a0}{number}");
            out.push(Numbered { figure: f, number, label });
        }
        _ => {}
    });
    out
}

/// A figure's caption as a list of figures gives it: its short form or its
/// words, without a closing full stop (a leader follows it).
pub fn list_caption(f: &Figure) -> String {
    let words = f.short.clone().unwrap_or_else(|| crate::block::plain_text(&crate::block::content_of(&f.caption, &f.caption_content)));
    words.trim().trim_end_matches('.').to_string()
}

/// A figure's identity while its document is borrowed: its address.
pub fn key(f: &Figure) -> usize {
    std::ptr::from_ref(f) as usize
}
