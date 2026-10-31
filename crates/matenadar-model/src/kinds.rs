//! Elements for book kinds, each drawn from its data: puzzles (mazes, logic
//! grids, dot-to-dot), journal and planner pages (rulings, calendars),
//! paper-craft sheets (faces, cut and fold lines, glue tabs, guides) and
//! comic pages (panels, balloons). Everything here is vector and live: an
//! edit to the data redraws it.

use serde::{Deserialize, Serialize};

use crate::block::{Inline, Track};
use crate::element::{Crop, Element, Filter, Place};
use crate::style::{Fill, Stroke, Style};
use crate::units::{Color, Length};

/// A maze, from its walls, with its way through.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Maze {
    #[serde(flatten)]
    pub place: Place,
    /// One string per row of cells, a hexadecimal digit per cell giving its
    /// walls: 1 top, 2 right, 4 bottom, 8 left, added up (`f` is closed).
    /// Neighbours may both name the wall they share or only one of them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<String>,
    /// Or the walls as segments, `[x1, y1, x2, y2]` in cells, for mazes that
    /// aren't a square grid (with `size`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<[f64; 4]>,
    /// The maze's size in cells with `segments`: `[columns, rows]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<[f64; 2]>,
    /// Cell size (default: the width over the columns).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell: Option<Length>,
    /// The start and the end: a cell `[row, column]`, or with `segments` a
    /// point `[x, y]` in cells. The start is a dot, the end a ring.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<[f64; 2]>,
    /// The way through: cells (or points), in order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub solution: Vec<[f64; 2]>,
    /// Draw the way through (a solution page).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solved: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walls: Option<Stroke>,
    /// The way through's line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Stroke>,
    /// The start's and end's colour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marks: Option<Color>,
}

/// A logic-grid puzzle: every pair of categories as a block of cells, in
/// the usual staircase (the first category down the side, the others
/// across the top, the last ones down the side under the first).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LogicGrid {
    #[serde(flatten)]
    pub place: Place,
    /// At least three categories, all with the same number of items.
    pub categories: Vec<Category>,
    /// Cell size (default 14pt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell: Option<Length>,
    /// Marks in cells: a solution, or what a puzzle starts with.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<LogicMark>,
    /// The labels' style (a paragraph or character style).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Lines between cells, and around each block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<Stroke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<Stroke>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Category {
    pub name: String,
    pub items: Vec<String>,
}

/// A mark where two items meet: `a` and `b` are `[category, item]`
/// (indices from 0); `mark` is `"o"` (they go together, a dot), `"x"` (a
/// cross) or up to three letters, written in the cell.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogicMark {
    pub a: [usize; 2],
    pub b: [usize; 2],
    #[serde(default = "default_mark")]
    pub mark: String,
}

fn default_mark() -> String {
    "o".into()
}

/// Dot-to-dot: numbered dots joined in order; solved, the lines are drawn.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Dots {
    #[serde(flatten)]
    pub place: Place,
    /// The dots, in order, in `viewbox` units (default points from the
    /// element's corner).
    pub points: Vec<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewbox: Option<[f64; 4]>,
    /// The first dot's number (default 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<u32>,
    /// Where the pen lifts: the indices of dots that begin a new line.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub breaks: Vec<usize>,
    /// Draw the lines (a solution page).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solved: Option<bool>,
    /// The dots' diameter (default 3pt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dot: Option<Length>,
    /// The numbers' size (default 7pt) and style.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<Stroke>,
    /// Where each number sits, as `[dx, dy]` in points from its dot. Without
    /// them, numbers sit outside the turn the line makes at their dot.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<[f64; 2]>,
}

/// A ruled area for writing and drawing: lines, dots or a grid.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ruling {
    #[serde(flatten)]
    pub place: Place,
    /// `"lines"` (default), `"dots"`, `"grid"`, `"graph"` (a grid with every
    /// `major`th line heavier) or `"isodots"` (dots in triangles, for
    /// isometric drawing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// The spacing (default 0.3 in for lines, 0.2 in otherwise).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pitch: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Stroke>,
    /// Graph paper: every this many lines is heavier (default 5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub major: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub major_stroke: Option<Stroke>,
    /// Lines: a margin line this far in from the left.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_stroke: Option<Stroke>,
    /// Lines: the first line's distance from the top (default one pitch).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<Length>,
}

/// A month as a calendar grid: weekday names above, a cell per day.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Calendar {
    #[serde(flatten)]
    pub place: Place,
    pub year: i32,
    /// 1 to 12.
    pub month: u32,
    /// `"monday"` (default) or `"sunday"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub week_start: Option<String>,
    /// The weekday names, from the week's first day (default Mon, Tue…).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub days: Vec<String>,
    /// The day numbers' style, and the weekday names'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_style: Option<String>,
    /// The weekday row's height (default 18pt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<Stroke>,
    /// Fill the weeks with the days of the months around, greyed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outside: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outside_color: Option<Color>,
}

/// A paper-craft sheet: faces to print, and the lines to cut, fold and glue
/// along, drawn as the trade draws them (cuts solid, mountain folds
/// dash-dot, valley folds dashed, scores dotted, glue tabs shaded).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Craft {
    #[serde(flatten)]
    pub place: Place,
    /// The coordinates' box (default points from the element's corner).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewbox: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub faces: Vec<CraftFace>,
    pub edges: Vec<CraftEdge>,
    /// Guides: where to line parts up.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<CraftMark>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cut: Option<Stroke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mountain: Option<Stroke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valley: Option<Stroke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<Stroke>,
    /// The glue tabs' fill.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glue: Option<Color>,
    /// A pair of scissors on the first cut line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scissors: Option<bool>,
    /// The labels' style.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

/// A face: a polygon, filled or with a picture cut to its shape.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CraftFace {
    pub points: Vec<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop: Option<Crop>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<(f64, f64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    /// Words at the face's middle (a letter to match, a number).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CraftEdge {
    pub from: [f64; 2],
    pub to: [f64; 2],
    /// `"cut"` (default), `"mountain"`, `"valley"` or `"score"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// A glue tab on this edge: the edge becomes its fold.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab: Option<Tab>,
}

/// A glue tab: a trapezoid standing off its edge, to the left going from
/// `from` to `to` (outside a face whose points run clockwise), or the
/// `side` given.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tab {
    /// How far it stands off the edge, in the sheet's units.
    pub depth: f64,
    /// The angle of its sides to the edge (default 45 degrees).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub angle: Option<f64>,
    /// `"left"` (default) or `"right"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    /// Words on the tab: the letter of the face it glues to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CraftMark {
    pub at: [f64; 2],
    /// `"cross"` (default: a registration cross) or `"dot"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A comic page's panels, in rows, with gutters between them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Panels {
    #[serde(flatten)]
    pub place: Place,
    pub rows: Vec<PanelRow>,
    /// Between panels, across and down (default 9pt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gutter: Option<Length>,
    /// Every panel's border (default 1.5pt black).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<Stroke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Length>,
    /// A panel's ground under its picture (default white).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelRow {
    /// A length, or a share of what is left (`"1fr"`, the default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<Track>,
    pub panels: Vec<Panel>,
}

/// A panel: a picture framed (cropped and focused as an image is), and what
/// is set over it, placed from the panel's corner. Balloons may cross its
/// border; the picture never does.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Panel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// A length, or a share of the row (`"2fr"`; default `"1fr"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<Track>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop: Option<Crop>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<(f64, f64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<Stroke>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<Element>,
}

/// Words in a balloon: speech, thought, a shout, a whisper or a caption,
/// its tail pointing at who says them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Balloon {
    /// `w` is the widest the words may run; the balloon fits the lines,
    /// balanced. `x` and `y` place the balloon (`anchor` as for any element).
    #[serde(flatten)]
    pub place: Place,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<Style>,
    /// `"speech"` (default), `"thought"`, `"shout"`, `"whisper"` or
    /// `"caption"` (a box, no tail).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    /// The point the tail reaches, in the same frame as `x` and `y`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tail: Option<[Length; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Stroke>,
    /// Room between the words and the outline (default 0.5 em).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inset: Option<Length>,
}
