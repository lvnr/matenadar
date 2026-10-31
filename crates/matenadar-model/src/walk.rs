//! Visiting every block, element and inline of a document, with its path.

use crate::block::{Block, Cell, Inline, ListItem};
use crate::element::Element;
use crate::{Document, Item};

pub enum Node<'a> {
    Item(&'a Item),
    Block(&'a Block),
    Element(&'a Element),
    Inline(&'a Inline),
}

/// Calls `f` for every node, with a path such as `content[2].flow[5]`.
pub fn walk<'a>(doc: &'a Document, f: &mut dyn FnMut(&str, Node<'a>)) {
    for (i, m) in doc.design.masters.iter() {
        for (j, e) in m.background.iter().enumerate() {
            element(&format!("design.masters.{i}.background[{j}]"), e, f);
        }
        for (j, e) in m.foreground.iter().enumerate() {
            element(&format!("design.masters.{i}.foreground[{j}]"), e, f);
        }
    }
    for (i, item) in doc.content.iter().enumerate() {
        let path = format!("content[{i}]");
        f(&path, Node::Item(item));
        match item {
            Item::Section(s) => blocks(&format!("{path}.flow"), &s.flow, f),
            Item::Page(p) => {
                for (j, e) in p.elements.iter().enumerate() {
                    element(&format!("{path}.elements[{j}]"), e, f);
                }
            }
            Item::Spread(s) => {
                for (side, sp) in [("left", &s.left), ("right", &s.right)] {
                    for (j, e) in sp.elements.iter().enumerate() {
                        element(&format!("{path}.{side}.elements[{j}]"), e, f);
                    }
                }
            }
        }
    }
}

pub fn blocks<'a>(path: &str, list: &'a [Block], f: &mut dyn FnMut(&str, Node<'a>)) {
    for (i, b) in list.iter().enumerate() {
        block(&format!("{path}[{i}]"), b, f);
    }
}

pub fn block<'a>(path: &str, b: &'a Block, f: &mut dyn FnMut(&str, Node<'a>)) {
    f(path, Node::Block(b));
    match b {
        Block::P(p) => inlines(&format!("{path}.content"), &p.content, f),
        Block::H(h) => inlines(&format!("{path}.content"), &h.content, f),
        Block::Figure(fig) => inlines(&format!("{path}.caption_content"), &fig.caption_content, f),
        Block::Box(x) => blocks(&format!("{path}.blocks"), &x.blocks, f),
        Block::Keep(x) => blocks(&format!("{path}.blocks"), &x.blocks, f),
        Block::Columns(x) => blocks(&format!("{path}.blocks"), &x.blocks, f),
        Block::Margin(m) => {
            inlines(&format!("{path}.content"), &m.content, f);
            blocks(&format!("{path}.blocks"), &m.blocks, f);
        }
        Block::List(l) => {
            for (i, item) in l.items.iter().enumerate() {
                if let ListItem::Blocks(bs) = item {
                    blocks(&format!("{path}.items[{i}]"), bs, f);
                }
            }
        }
        Block::Table(t) => {
            for (kind, rows) in [("header", &t.header), ("rows", &t.rows)] {
                for (r, row) in rows.iter().enumerate() {
                    for (c, cell) in row.iter().enumerate() {
                        if let Cell::Full(spec) = cell {
                            inlines(&format!("{path}.{kind}[{r}][{c}].content"), &spec.content, f);
                            blocks(&format!("{path}.{kind}[{r}][{c}].blocks"), &spec.blocks, f);
                        }
                    }
                }
            }
        }
        Block::Canvas(c) => {
            for (i, e) in c.elements.iter().enumerate() {
                element(&format!("{path}.elements[{i}]"), e, f);
            }
        }
        _ => {}
    }
}

pub fn element<'a>(path: &str, e: &'a Element, f: &mut dyn FnMut(&str, Node<'a>)) {
    f(path, Node::Element(e));
    match e {
        Element::Text(t) => {
            inlines(&format!("{path}.content"), &t.content, f);
            blocks(&format!("{path}.blocks"), &t.blocks, f);
        }
        Element::Group(g) => {
            for (i, c) in g.children.iter().enumerate() {
                element(&format!("{path}.children[{i}]"), c, f);
            }
        }
        Element::Panels(p) => {
            for (i, row) in p.rows.iter().enumerate() {
                for (j, panel) in row.panels.iter().enumerate() {
                    for (k, c) in panel.elements.iter().enumerate() {
                        element(&format!("{path}.rows[{i}].panels[{j}].elements[{k}]"), c, f);
                    }
                }
            }
        }
        Element::Balloon(b) => inlines(&format!("{path}.content"), &b.content, f),
        _ => {}
    }
}

pub fn inlines<'a>(path: &str, list: &'a [Inline], f: &mut dyn FnMut(&str, Node<'a>)) {
    for (i, x) in list.iter().enumerate() {
        let p = format!("{path}[{i}]");
        f(&p, Node::Inline(x));
        if let Inline::Note(n) = x {
            inlines(&format!("{p}.note"), &n.note, f);
        }
    }
}
