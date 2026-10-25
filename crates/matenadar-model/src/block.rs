//! Blocks and inline content: what flows through a section's pages and fills
//! text frames.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::element::Element;
use crate::style::{Align, Border, Fill, Insets, Radius, Style, TextStyle};
use crate::units::Length;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Block {
    /// A paragraph.
    P(Para),
    /// A heading; level 1 headings feed running heads and the contents.
    H(Heading),
    /// A scene break.
    Break(SceneBreak),
    Figure(Figure),
    /// A box around blocks: callouts, sidebars, slips.
    Box(BoxBlock),
    Table(Table),
    List(List),
    /// Preformatted lines (code, listings), spaces kept.
    Code(Code),
    /// A drawing of placed elements, inline in the flow.
    Canvas(Canvas),
    /// An instance of a component made of blocks (or of elements, as a canvas).
    Use(UseBlock),
    PageBreak(PageBreak),
    /// Vertical space.
    Space(Space),
    Columns(Columns),
    /// A margin note, in the master's note column, level with the block
    /// before it (or with where it stands, with `at: "here"`).
    Margin(MarginNote),
    /// Table of contents from the level 1 (and deeper) headings.
    Toc(Toc),
    /// Blocks that must stay on one page.
    Keep(Keep),
}

impl Block {
    pub fn id(&self) -> Option<&str> {
        match self {
            Block::P(b) => b.id.as_deref(),
            Block::H(b) => b.id.as_deref(),
            Block::Break(b) => b.id.as_deref(),
            Block::Figure(b) => b.id.as_deref(),
            Block::Box(b) => b.id.as_deref(),
            Block::Table(b) => b.id.as_deref(),
            Block::List(b) => b.id.as_deref(),
            Block::Code(b) => b.id.as_deref(),
            Block::Canvas(b) => b.id.as_deref(),
            Block::Use(b) => b.id.as_deref(),
            Block::Margin(b) => b.id.as_deref(),
            Block::Toc(b) => b.id.as_deref(),
            Block::Keep(b) => b.id.as_deref(),
            Block::Columns(b) => b.id.as_deref(),
            Block::PageBreak(_) | Block::Space(_) => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Block::P(_) => "p",
            Block::H(_) => "h",
            Block::Break(_) => "break",
            Block::Figure(_) => "figure",
            Block::Box(_) => "box",
            Block::Table(_) => "table",
            Block::List(_) => "list",
            Block::Code(_) => "code",
            Block::Canvas(_) => "canvas",
            Block::Use(_) => "use",
            Block::PageBreak(_) => "page-break",
            Block::Space(_) => "space",
            Block::Columns(_) => "columns",
            Block::Margin(_) => "margin",
            Block::Toc(_) => "toc",
            Block::Keep(_) => "keep",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Para {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Plain text (a newline breaks the line).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Formatted text, used when `text` is absent.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Inline>,
    /// Local overrides of the style.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<Style>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Heading {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub level: u8,
    /// Defaults to `h1`, `h2`… when the design has such styles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Inline>,
    /// Listed in the contents (default true for every level).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline: Option<bool>,
    /// The heading's title for running heads and the contents, when it
    /// differs from the text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<Style>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SceneBreak {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// The ornament (default `"*  *  *"`; `""` for white space only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Figure {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub src: String,
    /// Width (default the column's width).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub caption_content: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption_style: Option<String>,
    /// `"here"` (default), `"top"` or `"bottom"` of the page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub float: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Border>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_before: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_after: Option<Length>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BoxBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Paragraph style of the box's text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    pub blocks: Vec<Block>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Border>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inset: Option<Insets>,
    /// Width (default the column's width).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    /// May split across pages (default false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_before: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_after: Option<Length>,
    /// Turn the box (it keeps its place in the flow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotate: Option<f64>,
}

/// A column width: a length, `"auto"`, or a share of what is left (`"1fr"`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Track {
    Length(Length),
    Word(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Cell {
    Text(String),
    Full(CellSpec),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CellSpec {
    pub text: Option<String>,
    pub content: Vec<Inline>,
    pub blocks: Vec<Block>,
    pub colspan: Option<u32>,
    pub rowspan: Option<u32>,
    pub fill: Option<Fill>,
    pub align: Option<Align>,
    pub style: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Table {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub columns: Vec<Track>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<Vec<Cell>>,
    pub rows: Vec<Vec<Cell>>,
    /// Paragraph style of the cells.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_fill: Option<Fill>,
    /// Alternate row fill.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zebra: Option<Fill>,
    /// Lines between cells (default a thin grey rule).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inset: Option<Insets>,
    /// Alignment per column.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub align: Vec<Align>,
    /// Repeat the header on every page the table reaches (default true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_header: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_before: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_after: Option<Length>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ListItem {
    Text(String),
    Blocks(Vec<Block>),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct List {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub items: Vec<ListItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordered: Option<bool>,
    /// The marker (`"•"`, `"–"`, `"☐"`), or for ordered lists a pattern (`"1."`, `"a)"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indent: Option<Length>,
    /// Between the marker and the text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Code {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Border>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inset: Option<Insets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_before: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_after: Option<Length>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Canvas {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Width (default the column's width).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<Length>,
    pub h: Option<Length>,
    pub elements: Vec<Element>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_before: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_after: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UseBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub component: String,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub props: IndexMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PageBreak {
    /// `"recto"` or `"verso"`: the next page on that side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Space {
    pub h: Length,
    /// Weak space disappears at the top of a page (default true).
    #[serde(default)]
    pub weak: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Columns {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub count: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gutter: Option<Length>,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MarginNote {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Inline>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>,
    /// `"previous"` (default: level with the top of the block before) or `"here"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// Shift down (or up, negative) from that level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dy: Option<Length>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Toc {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Deepest heading level listed (default 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u8>,
    /// Paragraph style per level: `"toc-1"`, `"toc-2"`… by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Leader between title and page number (`"."`, `" "`; default none).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keep {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub blocks: Vec<Block>,
}

/// Inline content: text runs, notes, fields, references and line breaks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Inline {
    Plain(String),
    Run(Run),
    Note(Footnote),
    Field(Field),
    Ref(PageRef),
    Break(LineBreak),
    Image(InlineImage),
}

/// Text with the same marks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub text: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub em: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub strong: bool,
    /// A named character style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub sup: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub sub: bool,
    /// Never break this run across lines.
    #[serde(default, skip_serializing_if = "is_false")]
    pub nobreak: bool,
    #[serde(flatten)]
    pub set: TextStyle,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Footnote {
    pub note: Vec<Inline>,
}

/// A value filled in at layout: `"page"` (the page number), `"pages"`
/// (the total), `"section.title"`, `"section.<var>"`, `"heading"` (the last
/// level 1 heading on or before the page), `"doc.title"`, `"doc.author"`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<String>,
    #[serde(default, flatten)]
    pub set: TextStyle,
}

/// The page number (or title) of the node with id `ref`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageRef {
    #[serde(rename = "ref")]
    pub target: String,
    /// `"page"` (default) or `"title"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineBreak {
    pub br: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InlineImage {
    pub image: String,
    #[serde(default)]
    pub h: Option<Length>,
    #[serde(default)]
    pub w: Option<Length>,
    /// Lowers the image below the baseline.
    #[serde(default)]
    pub baseline: Option<Length>,
}

impl Inline {
    /// The text this inline contributes to a paragraph's plain text.
    pub fn plain(&self) -> &str {
        match self {
            Inline::Plain(s) => s,
            Inline::Run(r) => &r.text,
            _ => "",
        }
    }
}

/// The plain text of inline content.
pub fn plain_text(content: &[Inline]) -> String {
    content.iter().map(Inline::plain).collect()
}

/// A paragraph's content: its `text` as one run, or its `content`.
pub fn content_of(text: &Option<String>, content: &[Inline]) -> Vec<Inline> {
    match text {
        Some(t) => vec![Inline::Plain(t.clone())],
        None => content.to_vec(),
    }
}

/// Default insets helper for callers that need a zero inset.
pub fn no_inset() -> Insets {
    Insets::All(Length::ZERO)
}

/// Unused import guard for types only referenced in docs.
#[allow(dead_code)]
fn _types(_: TextStyle) {}
