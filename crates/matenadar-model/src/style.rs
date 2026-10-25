//! Type styles, strokes, fills and page masters.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::element::Element;
use crate::units::{Color, Degrees, Length};

/// One family, or a list tried in order (later ones fill missing glyphs).
/// A name that is a key of `design.fonts` stands for that stack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FontStack {
    One(String),
    Many(Vec<String>),
}

impl FontStack {
    pub fn families(&self) -> Vec<&str> {
        match self {
            FontStack::One(f) => vec![f.as_str()],
            FontStack::Many(v) => v.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Case {
    Upper,
    Lower,
    /// Small capitals from the font (`smcp`), lowercase letters only.
    SmallCaps,
    /// Every letter as a small capital (`c2sc` + `smcp`).
    AllSmallCaps,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Align {
    Left,
    Right,
    Center,
    Justify,
    /// Toward the outside edge: right on rectos, left on versos.
    Outside,
    Inside,
}

/// Character-level formatting. Every field is optional; unset fields come
/// from the enclosing style.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<FontStack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Length>,
    /// 100–900, or "regular", "medium", "bold"…
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// Width axis in percent (100 = normal), for fonts with widths.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stretch: Option<f64>,
    /// Letter spacing (`"0.12em"`, `"0.5pt"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracking: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_spacing: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case: Option<Case>,
    /// OpenType features: `"onum"`, `"ss01"`, `"-liga"` (turned off), `"salt=2"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    /// Raise (positive) or lower the text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strike: Option<bool>,
    /// BCP 47 language for hyphenation and shaping (`"hy"`, `"ar"`, `"ja"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// A stroke around each glyph (outlined type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline: Option<Stroke>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyphenate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ligatures: Option<bool>,
    /// A halo behind each glyph (for type over pictures): a stroke whose
    /// colour and width give the halo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halo: Option<Stroke>,
    /// Variable-font axes: `{"opsz": 12, "SOFT": 50}`. Weight, stretch and
    /// size already set `wght`, `wdth` and `opsz`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variations: Option<indexmap::IndexMap<String, f64>>,
    /// Baseline to baseline. A number without a unit is a multiple of the
    /// size (`1.4`): a larger run in the line opens it. A length (`"14pt"`)
    /// is fixed leading: runs of any size keep to it. On a run, `0` keeps
    /// the run from opening its line at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_height: Option<LineHeight>,
    /// Let punctuation and hyphens at line ends hang slightly into the
    /// margin, so edges look straight (default true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overhang: Option<bool>,
}

/// A paragraph style: text formatting plus how lines and paragraphs sit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    /// Another style whose settings this one starts from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherits: Option<String>,
    #[serde(flatten)]
    pub text: TextStyle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    /// First-line indent. Book convention: a paragraph that follows a
    /// heading, a break or anything but another paragraph of an indented
    /// style is not indented (see `indent_first`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indent: Option<Length>,
    /// Indent even the first paragraph after a heading or break.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indent_first: Option<bool>,
    /// Every line but the first is indented this much.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hanging: Option<Length>,
    /// Extra left and right indents of the whole paragraph.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_before: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_after: Option<Length>,
    /// Don't end a page between this paragraph and the next block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_with_next: Option<bool>,
    /// Never split this paragraph across pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_together: Option<bool>,
    /// Allow a paragraph's last line alone at the top of a page, or its first
    /// line alone at the foot (both off by default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_widows: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_orphans: Option<bool>,
    /// Start this paragraph on a new page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_break_before: Option<bool>,
    /// A drop cap over the first `lines` lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drop_cap: Option<DropCap>,
    /// A rule above or below the paragraph.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_above: Option<Rule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_below: Option<Rule>,
    /// Lines of equal length: the paragraph takes the narrowest width that
    /// keeps its number of lines (CSS `text-wrap: balance`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<bool>,
    /// A background behind the paragraph's box.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inset: Option<Insets>,
    /// `"simple"` breaks lines one at a time; `"optimized"` chooses breaks
    /// for the whole paragraph at once (Knuth–Plass), for evener lines. By
    /// default justified text is optimized and ragged text is simple.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linebreaks: Option<String>,
    /// `"pretty"`: never end the paragraph on a lone short word (the last
    /// two words stay together).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<String>,
    /// How far justification may stretch and shrink word spaces and letter
    /// spacing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<Justification>,
}

/// Limits for justified lines: word spaces as a share of a normal space
/// (`"80%"` to `"150%"`), letter spacing as a length (`"-0.01em"`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Justification {
    pub spacing: Option<Limits>,
    pub tracking: Option<Limits>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub min: Option<Length>,
    pub max: Option<Length>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LineHeight {
    Factor(f64),
    Length(Length),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DropCap {
    pub lines: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<FontStack>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<serde_json::Value>,
    /// Space between the cap and the text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap: Option<Length>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    #[serde(default)]
    pub stroke: Option<Stroke>,
    /// Length of the rule (default: the paragraph's width).
    #[serde(default)]
    pub width: Option<Length>,
    /// Gap between the rule and the text.
    #[serde(default)]
    pub gap: Option<Length>,
}

/// A stroke: a colour (`"#000"`), or width, colour and dash.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Stroke {
    Color(Color),
    Full(StrokeSpec),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrokeSpec {
    #[serde(default)]
    pub width: Option<Length>,
    #[serde(default)]
    pub color: Option<Color>,
    /// `"solid"`, `"dashed"`, `"dotted"`, `"densely-dashed"`… or an array of
    /// dash and gap lengths.
    #[serde(default)]
    pub dash: Option<serde_json::Value>,
    /// `"butt"`, `"round"`, `"square"`.
    #[serde(default)]
    pub cap: Option<String>,
    /// `"miter"`, `"round"`, `"bevel"`.
    #[serde(default)]
    pub join: Option<String>,
    /// Where the dash pattern starts along the stroke.
    #[serde(default)]
    pub dash_offset: Option<Length>,
}

/// Per-side strokes for boxes: one stroke for every side, or some sides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Border {
    All(Stroke),
    Sides(BorderSides),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BorderSides {
    pub top: Option<Stroke>,
    pub right: Option<Stroke>,
    pub bottom: Option<Stroke>,
    pub left: Option<Stroke>,
}

/// A fill: a colour, or a gradient.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Fill {
    Color(Color),
    Gradient(Gradient),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "gradient", rename_all = "kebab-case")]
pub enum Gradient {
    /// `angle` in degrees: 0 runs left to right, 90 top to bottom.
    Linear {
        #[serde(default)]
        angle: Degrees,
        stops: Vec<(Color, f64)>,
    },
    Radial {
        stops: Vec<(Color, f64)>,
        /// Centre in fractions of the box (default 0.5, 0.5).
        #[serde(default)]
        center: Option<(f64, f64)>,
        /// Radius as a fraction of the box (default 0.5).
        #[serde(default)]
        radius: Option<f64>,
    },
}

/// Insets: one length for every side, or per side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Insets {
    All(Length),
    Sides(InsetSides),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InsetSides {
    pub top: Option<Length>,
    pub right: Option<Length>,
    pub bottom: Option<Length>,
    pub left: Option<Length>,
    pub x: Option<Length>,
    pub y: Option<Length>,
}

/// Corner radius: one for every corner, or per corner.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Radius {
    All(Length),
    Corners(RadiusCorners),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RadiusCorners {
    pub top_left: Option<Length>,
    pub top_right: Option<Length>,
    pub bottom_right: Option<Length>,
    pub bottom_left: Option<Length>,
}

/// Where a flow's text goes on the page.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Margins {
    pub top: Option<Length>,
    pub bottom: Option<Length>,
    /// The binding side.
    pub inside: Option<Length>,
    pub outside: Option<Length>,
    /// Margins that are the same on every page instead of mirrored. Setting
    /// either takes the place of `inside` and `outside`.
    pub left: Option<Length>,
    pub right: Option<Length>,
}

/// A page master: the trim, the flow area, and what is drawn on every page.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Master {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherits: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<Length>,
    /// Bleed on the three outside edges (none at the spine).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bleed: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margins: Option<Margins>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gutter: Option<Length>,
    /// The page's colour, under everything (into the bleed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Fill>,
    /// Drawn behind the page's content, on every page. Elements are placed
    /// from the trim's top-left corner and may carry `when` conditions.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub background: Vec<Element>,
    /// Drawn over the page's content.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub foreground: Vec<Element>,
    /// Margin notes beside the flow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<NoteColumn>,
    /// The default paragraph style of flows on this master.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoteColumn {
    /// `"outside"` (default), `"inside"`, `"left"` or `"right"`.
    #[serde(default)]
    pub side: Option<String>,
    pub width: Length,
    /// Between the text and the notes.
    #[serde(default)]
    pub gap: Option<Length>,
    /// Paragraph style of note text.
    #[serde(default)]
    pub style: Option<String>,
    /// Space between stacked notes.
    #[serde(default)]
    pub spacing: Option<Length>,
}

/// The design: colours, fonts, styles, masters and components.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Design {
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub colors: IndexMap<String, Color>,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub fonts: IndexMap<String, FontStack>,
    /// Defaults for all text.
    pub base: Style,
    /// Paragraph styles by name.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub styles: IndexMap<String, Style>,
    /// Character styles by name.
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub spans: IndexMap<String, TextStyle>,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub masters: IndexMap<String, Master>,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub components: IndexMap<String, crate::component::Component>,
}
