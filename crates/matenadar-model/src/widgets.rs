//! Structured vector and layout elements: rulings, calendars, framed panel
//! grids, and callout/speech balloons. Everything here is live vector and text:
//! editing the data redraws the element cleanly.

use serde::{Deserialize, Serialize};

use crate::block::{Inline, Track};
use crate::element::{Crop, Element, Filter, Place};
use crate::style::{Fill, Stroke, Style};
use crate::units::{Color, Length};

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

/// Framed panels arranged in rows with gutters between them (storyboards,
/// comics, contact sheets, picture grids).
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
/// is set over it, placed from the panel's corner. Callouts and balloons may
/// cross its border; the picture never does.
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

/// Words in a callout or balloon: speech, thought, shout, whisper, or caption,
/// with an optional tail pointing to a coordinate.
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
