//! Elements: things placed at a position, on fixed pages, in masters, in
//! canvases and in components. Positions are from the top-left corner of the
//! page's trim (or of the containing canvas or group); the bleed lies at
//! negative coordinates and beyond the trim's width and height.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::block::{Block, Inline};
use crate::style::{Border, Fill, Insets, Radius, Stroke, TextStyle};
use crate::units::{Color, Degrees, Length};

/// Where an element sits. `x`/`y` place the corner named by `anchor`
/// (top-left by default). Rotation turns the element about its centre.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Place {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<Length>,
    /// `"top-left"` (default), `"top"`, `"top-right"`, `"left"`, `"center"`,
    /// `"right"`, `"bottom-left"`, `"bottom"`, `"bottom-right"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotate: Option<Degrees>,
    /// Slant the element horizontally by this angle (positive leans right).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skew: Option<Degrees>,
    /// Conditions, all of which must hold for the element to be drawn:
    /// `"recto"`, `"verso"`, `"first"` (a section's first page),
    /// `"not-first"`, `"blank"` (a page left empty so a section opens on the
    /// right side), `"not-blank"`, `"flow"` (a page of a section's flow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// Marks the element as art that may cross the trim (no margin checks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decorative: Option<bool>,
    /// A hard shadow: a copy of the element's box in a colour, offset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Shadow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub dx: Length,
    pub dy: Length,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Element {
    Text(TextFrame),
    Image(ImageEl),
    Rect(ShapeEl),
    Ellipse(ShapeEl),
    Line(LineEl),
    Path(PathEl),
    Svg(SvgEl),
    Group(GroupEl),
    Use(UseEl),
    Qr(QrEl),
    Lettering(Lettering),
    LetterGrid(LetterGrid),
}

impl Element {
    pub fn place(&self) -> Option<&Place> {
        Some(match self {
            Element::Text(e) => &e.place,
            Element::Image(e) => &e.place,
            Element::Rect(e) | Element::Ellipse(e) => &e.place,
            Element::Path(e) => &e.place,
            Element::Svg(e) => &e.place,
            Element::Group(e) => &e.place,
            Element::Use(e) => &e.place,
            Element::Qr(e) => &e.place,
            Element::Lettering(e) => &e.place,
            Element::LetterGrid(e) => &e.place,
            Element::Line(_) => return None,
        })
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Element::Line(l) => l.id.as_deref(),
            other => other.place().and_then(|p| p.id.as_deref()),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Element::Text(_) => "text",
            Element::Image(_) => "image",
            Element::Rect(_) => "rect",
            Element::Ellipse(_) => "ellipse",
            Element::Line(_) => "line",
            Element::Path(_) => "path",
            Element::Svg(_) => "svg",
            Element::Group(_) => "group",
            Element::Use(_) => "use",
            Element::Qr(_) => "qr",
            Element::Lettering(_) => "lettering",
            Element::LetterGrid(_) => "letter-grid",
        }
    }
}

/// A frame of text: paragraphs (`blocks`), or a shortcut (`text` or
/// `content`) for one paragraph in the frame's style.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextFrame {
    #[serde(flatten)]
    pub place: Place,
    /// Paragraph style of the frame's text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Plain text: a blank line starts a new paragraph, a newline breaks the line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Inline>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>,
    /// Local overrides of the style.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<crate::style::Style>,
    /// `"top"` (default), `"middle"` or `"bottom"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valign: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inset: Option<Insets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Border>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
    /// `"none"` (default; overflow is reported), `"shrink"` (the type
    /// shrinks until it fits `h`, down to `min_size`) or `"width"` (the type
    /// grows or shrinks so the longest line fills `w`, between `min_size`
    /// and `max_size`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_size: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gutter: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageEl {
    #[serde(flatten)]
    pub place: Place,
    /// A path relative to the document's asset root.
    pub src: String,
    /// `"cover"` (default: fills the box, cropping), `"contain"`, `"stretch"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<String>,
    /// For `cover`: which part stays in view, as fractions (0.5, 0.5 = centre).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<(f64, f64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Border>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flip: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShapeEl {
    #[serde(flatten)]
    pub place: Place,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Border>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LineEl {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub x1: Option<Length>,
    pub y1: Option<Length>,
    pub x2: Option<Length>,
    pub y2: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Stroke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

/// An SVG path (`d`) in a `w` × `h` box whose coordinates are points
/// (or `viewbox` units when given).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PathEl {
    #[serde(flatten)]
    pub place: Place,
    pub d: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewbox: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Stroke>,
}

/// An SVG drawing, from a file (`src`) or written inline (`markup`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SvgEl {
    #[serde(flatten)]
    pub place: Place,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    /// Show this part of the drawing: `[x, y, w, h]` in its own units.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewbox: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GroupEl {
    #[serde(flatten)]
    pub place: Place,
    pub children: Vec<Element>,
    /// Clip the children to the group's `w` × `h`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<bool>,
    /// Scale the group about its top-left corner (1 = actual size).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Border>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
}

/// An instance of a component from `design.components`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UseEl {
    #[serde(flatten)]
    pub place: Place,
    pub component: String,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub props: IndexMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QrEl {
    #[serde(flatten)]
    pub place: Place,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Color>,
    /// Quiet zone in modules (default 0: the caller leaves white around it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quiet: Option<u32>,
    /// Error correction: `"L"`, `"M"` (default), `"Q"` or `"H"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecc: Option<String>,
}

/// Display text drawn as vector outlines, not live text: for decorative
/// type that runs into the bleed or must never reflow. `x`/`y` place the
/// top-left of the text's box: the font's ascender line, as in CSS.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lettering {
    #[serde(flatten)]
    pub place: Place,
    pub text: String,
    #[serde(flatten)]
    pub style: TextStyle,
    /// A named character style to start from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<String>,
    /// Line height for multi-line lettering (multiple of the size).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f64>,
    /// `"left"` (default), `"center"`, `"right"` within `w`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
}

/// A grid of letters: word searches, crosswords, letter puzzles.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LetterGrid {
    #[serde(flatten)]
    pub place: Place,
    /// One string per row, one letter per character; or rows of cells.
    pub rows: Vec<GridRow>,
    /// Cell size (square unless `cell_h` is set).
    pub cell: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell_h: Option<Length>,
    /// Paragraph or character style of the letters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<TextStyle>,
    /// Lines between cells.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<Stroke>,
    /// The grid's outer border.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<Stroke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
    /// Marks over found words, drawn under the letters.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<Highlight>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GridRow {
    Letters(String),
    Cells(Vec<GridCell>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GridCell {
    Text(String),
    Full(GridCellSpec),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GridCellSpec {
    pub text: Option<String>,
    pub fill: Option<Fill>,
    /// A small number in the corner (crosswords).
    pub number: Option<String>,
    pub color: Option<Color>,
    /// A blocked cell (crosswords): filled, no letter.
    pub block: Option<bool>,
    pub weight: Option<serde_json::Value>,
    /// A character style for this cell's letter.
    pub span: Option<String>,
}

/// A capsule from one cell to another (rows and columns from 0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Highlight {
    pub from: (usize, usize),
    pub to: (usize, usize),
    #[serde(default)]
    pub fill: Option<Color>,
    #[serde(default)]
    pub stroke: Option<Stroke>,
    /// Thickness as a fraction of the cell (default 0.78).
    #[serde(default)]
    pub thickness: Option<f64>,
    /// `"capsule"` (default) or `"cells"` (each cell filled).
    #[serde(default)]
    pub shape: Option<String>,
}
