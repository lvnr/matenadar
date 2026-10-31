//! Matenadar's document model.
//!
//! A document is a sequence of sections and pages under one design:
//!
//! - a **section** is a flow of blocks (paragraphs, headings, figures,
//!   tables, boxes, canvases…) poured through as many pages of its master
//!   as it needs;
//! - a **page** is laid out by hand: elements (text frames, images, shapes,
//!   drawings, component instances…) at fixed positions.
//!
//! The design holds colours, font stacks, paragraph and character styles,
//! page masters and components. The model is the only source of truth: the
//! engine generates its layout program from it and never stores that.

pub mod block;
pub mod component;
pub mod element;
pub mod figures;
pub mod ops;
pub mod resolve;
pub mod runs;
pub mod style;
pub mod template;
pub mod units;
pub mod validate;
pub mod walk;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub use block::{Block, Inline, Run};
pub use component::Component;
pub use element::{Element, Place};
pub use ops::{Op, OpError};
pub use style::{Design, Master, Style, TextStyle};
pub use units::{Color, Length};
pub use validate::{Problem, validate};

/// The model version this crate reads and writes.
pub const VERSION: &str = "1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// The model version, `"1"`.
    pub matenadar: String,
    pub meta: Meta,
    /// Font folders under the asset root, searched before the system's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fonts: Vec<String>,
    #[serde(default)]
    pub design: Design,
    pub content: Vec<Item>,
    /// Settings for the page a node lands on, keyed by the node's id. They
    /// follow the node when the text reflows; a key whose node is gone does
    /// nothing.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub on_page: IndexMap<String, OnPage>,
    /// How the book is printed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub print: Option<Print>,
}

/// How the book is printed: what the checks hold it to.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Print {
    /// `"black"` (a black-ink interior: any colour on a page is reported)
    /// or `"color"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ink: Option<String>,
    /// A filter for every raster picture that sets none of its own (such as
    /// `{"gray": true, "contrast": 3}` for black ink).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<element::Filter>,
}

impl Document {
    /// Whether the interior is printed in black ink only.
    pub fn black_ink(&self) -> bool {
        self.print.as_ref().and_then(|p| p.ink.as_deref()) == Some("black")
    }
}

/// What changes on the page where a node lands.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OnPage {
    /// Master elements not drawn there: by id, by the style of a text
    /// element (`"folio"`, `"running"`), or `"all"`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hide: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Meta {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// BCP 47 language, such as `"en-US"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Item {
    Section(Section),
    Page(Page),
    Spread(Box<Spread>),
}

impl Item {
    pub fn id(&self) -> &str {
        match self {
            Item::Section(s) => &s.id,
            Item::Page(p) => &p.id,
            Item::Spread(s) => &s.id,
        }
    }

    pub fn master(&self) -> Option<&str> {
        match self {
            Item::Section(s) => Some(&s.master),
            Item::Page(p) => p.master.as_deref(),
            Item::Spread(s) => s.master.as_deref(),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Item::Section(_) => "section",
            Item::Page(_) => "page",
            Item::Spread(_) => "spread",
        }
    }
}

/// Two facing pages, a verso and the recto after it, under one picture:
/// the picture covers both trims and their outer bleeds (top, bottom and
/// outside; there is no bleed at the spine), as one or as a half per side.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Spread {
    pub id: String,
    /// The master that gives both pages their size and bleed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master: Option<String>,
    /// The picture across both pages (or give each side its own `src`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    /// Only this part of the picture (see the image element's `crop`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop: Option<element::Crop>,
    /// Which part stays in view when the picture is cropped to the spread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<(f64, f64)>,
    /// How much of each page the binding hides at the spine. That strip is
    /// repeated on both sides of the fold, so the picture reads across it
    /// without losing what is there (default 0: a lay-flat binding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gutter: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<element::Filter>,
    /// What the picture shows, for readers who can't see it (ebooks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    /// The verso (left) page.
    pub left: SpreadSide,
    /// The recto (right) page.
    pub right: SpreadSide,
    /// Also draw the master's background and foreground (default false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<style::Fill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub vars: IndexMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_from: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numbering: Option<String>,
}

/// One page of a spread: its own picture (or its part of the spread's),
/// how that part is framed, and what is set over it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpreadSide {
    /// A picture for this page only (half of a spread drawn in two).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop: Option<element::Crop>,
    /// This page's own focus. With the spread's one picture, it moves only
    /// this page's half (which breaks the picture at the fold, on purpose).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<(f64, f64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<element::Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    /// Placed over the picture, from this page's trim corner.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<Element>,
}

/// A flow of blocks through the pages of a master.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub id: String,
    pub master: String,
    /// Where the section's first page falls: `"next"` (default), `"recto"`,
    /// `"verso"`, or `"continue"` (on the same page, when the master is the
    /// same as the section before).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// Page numbers from here on: `"1"`, `"i"`, `"I"`, `"a"`, `"A"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbering: Option<String>,
    /// Restart page numbers at this number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_from: Option<u32>,
    /// The section's title, for running heads (`{"field": "section.title"}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Values for the master's fields (`{"field": "section.chapter"}`).
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub vars: IndexMap<String, String>,
    /// Paragraph style of the flow (default the master's, then `body`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    pub flow: Vec<Block>,
}

/// A page laid out by hand.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    /// The master that gives the page its size and bleed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master: Option<String>,
    /// Also draw the master's background and foreground (default false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub vars: IndexMap<String, String>,
    /// Restart page numbers here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_from: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbering: Option<String>,
    /// `"next"` (default), `"recto"` or `"verso"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<style::Fill>,
    pub elements: Vec<Element>,
}

impl Document {
    /// Reads a document from JSON, with the path of the first error.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let de = &mut serde_json::Deserializer::from_str(text);
        let doc: Document = serde_path_to_error::deserialize(de).map_err(|e| {
            let path = e.path().to_string();
            format!("{path}: {}", e.into_inner())
        })?;
        if doc.matenadar != VERSION {
            return Err(format!("matenadar: this engine reads model version \"{VERSION}\", not \"{}\"", doc.matenadar));
        }
        Ok(doc)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("documents serialize")
    }

    /// The files the document draws (pictures and drawings), as it names
    /// them under the asset root, each once.
    pub fn assets(&self) -> Vec<String> {
        fn scan(v: &serde_json::Value, out: &mut indexmap::IndexSet<String>) {
            match v {
                serde_json::Value::Object(map) => {
                    for (k, child) in map {
                        match (k.as_str(), child) {
                            ("src" | "image", serde_json::Value::String(path)) if !path.is_empty() => {
                                out.insert(path.trim_start_matches('/').to_string());
                            }
                            _ => scan(child, out),
                        }
                    }
                }
                serde_json::Value::Array(items) => items.iter().for_each(|i| scan(i, out)),
                _ => {}
            }
        }
        let mut out = indexmap::IndexSet::new();
        scan(&serde_json::to_value(self).unwrap_or_default(), &mut out);
        out.into_iter().collect()
    }
}
