//! The picture-book template. Story pages are pictures to the bleed with
//! their words lettered in the art (the default); front and back matter is
//! typeset. The plan follows the trade's:
//!
//! - **Separate ends** (the endpapers are their own sheets): page 1 is the
//!   title page (or the half title, with the title on page 3), its back the
//!   copyright and dedication, and the story runs from there to the last
//!   page.
//! - **Self ends** (the first and last leaves are the endpapers): pages 1
//!   and N are pasted to the boards, 2–3 and N−2–N−1 are the endpapers,
//!   4 the copyright and dedication, 5 the title page, and the story runs
//!   from page 6 to N−3.
//!
//! A spread must open on a left-hand (even) page; the template says so
//! rather than leaving a page blank in the middle of a story. Pages the
//! story and back matter don't use are left blank at the end.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Faces, centred, merge, points, pt, trim};
use crate::element::Crop;
use crate::units::Length;

pub const TRIMS: [&str; 3] = ["8.5x8.5", "8x10", "8.5x11"];
pub const PAGES: [u32; 3] = [24, 32, 40];

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PictureBook {
    /// `"8.5x8.5"`, `"8x10"` or `"8.5x11"` (inches, width × height).
    pub trim: String,
    /// 24, 32 or 40.
    pub pages: u32,
    /// `"separate"` (default) or `"self"`.
    #[serde(default)]
    pub ends: Option<String>,
    /// Default 0.125 in, on the top, bottom and outside edges.
    #[serde(default)]
    pub bleed: Option<Length>,
    pub title: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub illustrator: Option<String>,
    #[serde(default)]
    pub publisher: Option<String>,
    /// The copyright page's words (a newline breaks the line).
    #[serde(default)]
    pub copyright: Option<String>,
    #[serde(default)]
    pub dedication: Option<String>,
    /// A page with the title alone before the title page (separate ends).
    #[serde(default)]
    pub half_title: bool,
    /// A picture behind the title page, the title set over it.
    #[serde(default)]
    pub title_art: Option<String>,
    /// Self ends: the endpapers' picture, front and back alike, or one each.
    #[serde(default)]
    pub endpapers: Option<Endpapers>,
    pub story: Vec<Story>,
    #[serde(default)]
    pub back: Vec<Back>,
    #[serde(default)]
    pub fonts: Faces,
    /// Font folders under the asset root (the document's `fonts`).
    #[serde(default)]
    pub font_dirs: Vec<String>,
    #[serde(default)]
    pub lang: Option<String>,
    /// Changes to the template's design, merged into it object by object:
    /// `{"styles": {"title": {"size": 48}}, "colors": {"ink": "#2b2140"}}`.
    #[serde(default)]
    pub design: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Endpapers {
    pub src: Option<String>,
    pub front: Option<String>,
    pub back: Option<String>,
}

/// A story page, or a spread of two.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Story {
    pub id: Option<String>,
    /// The picture, its words lettered in.
    pub src: Option<String>,
    /// Across two facing pages: one picture (`src`), or `left` and `right`.
    pub spread: bool,
    pub left: Option<Side>,
    pub right: Option<Side>,
    /// What the binding hides at the spine (see the spread item).
    pub gutter: Option<Length>,
    pub crop: Option<Crop>,
    pub focus: Option<(f64, f64)>,
    pub alt: Option<String>,
    /// Words typeset over the art (on request): a frame at the top of the
    /// page, in the safe zone, on a soft panel.
    pub text: Option<String>,
    /// Anything else set over the art: elements placed from the trim.
    pub words: Vec<Value>,
}

/// One page of a spread.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Side {
    pub src: Option<String>,
    pub crop: Option<Crop>,
    pub focus: Option<(f64, f64)>,
    pub alt: Option<String>,
    pub text: Option<String>,
    pub words: Vec<Value>,
}

/// A typeset page after the story: about the author, a note, a glossary.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Back {
    pub id: Option<String>,
    pub heading: Option<String>,
    /// A blank line starts a paragraph.
    pub text: String,
}

/// The page plan's geometry.
struct Geo {
    w: f64,
    h: f64,
    bleed: f64,
    /// The safe zone: top, bottom, inside, outside.
    top: f64,
    inside: f64,
    outside: f64,
}

impl Geo {
    /// The safe area's left edge and width on a page.
    fn safe_x(&self, recto: bool) -> (f64, f64) {
        let left = if recto { self.inside } else { self.outside };
        (left, self.w - self.inside - self.outside)
    }
}

pub fn compose(spec: &PictureBook) -> Result<Value, String> {
    let (w, h) = trim(&spec.trim, &TRIMS)?;
    if !PAGES.contains(&spec.pages) {
        return Err(format!("a picture book has 24, 32 or 40 pages, not {}", spec.pages));
    }
    let self_ends = match spec.ends.as_deref() {
        None | Some("separate") => false,
        Some("self") => true,
        Some(other) => return Err(format!("ends are \"self\" or \"separate\", not \"{other}\"")),
    };
    if self_ends && spec.half_title {
        return Err("a self-ended book has no room for a half title (its title spread is pages 4–5)".into());
    }
    let geo = Geo { w, h, bleed: points(spec.bleed, 9.0), top: 36.0, inside: 45.0, outside: 36.0 };
    let n = spec.pages as usize;
    let mut content: Vec<Value> = Vec::new();
    let mut page: usize;
    let mut problems: Vec<String> = Vec::new();

    if self_ends {
        let ends = spec.endpapers.clone().unwrap_or_default();
        content.push(blank("pastedown-front"));
        content.extend(endpapers("endpapers-front", ends.front.as_ref().or(ends.src.as_ref())));
        content.push(copyright_page(spec, &geo, false));
        content.push(title_page(spec, &geo, "title", true));
        page = 6;
    } else {
        if spec.half_title {
            content.push(half_title(spec, &geo));
            content.push(copyright_page(spec, &geo, false));
            content.push(title_page(spec, &geo, "title", true));
        } else {
            content.push(title_page(spec, &geo, "title", true));
            content.push(copyright_page(spec, &geo, false));
        }
        page = content.len() + 1;
    }
    let first_story = page;
    let last_free = if self_ends { n - 3 } else { n };

    for (i, s) in spec.story.iter().enumerate() {
        let id = s.id.clone().unwrap_or_else(|| format!("story-{:02}", i + 1));
        if s.spread {
            if page % 2 == 1 {
                problems.push(format!(
                    "story[{i}] is a spread but would open on page {page}, a right-hand page: add or take away a single page before it"
                ));
            }
            content.push(spread(&id, s, &geo));
            page += 2;
        } else {
            content.push(story_page(&id, s, &geo, page % 2 == 1));
            page += 1;
        }
    }
    for (i, b) in spec.back.iter().enumerate() {
        let id = b.id.clone().unwrap_or_else(|| format!("back-{:02}", i + 1));
        content.push(back_page(&id, b, &geo, page % 2 == 1));
        page += 1;
    }
    if page - 1 > last_free {
        problems.push(format!(
            "the story and back matter run from page {first_story} to {}, but a {n}-page book with {} ends has room to page {last_free}",
            page - 1,
            if self_ends { "self" } else { "separate" }
        ));
    }
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    while page <= last_free {
        content.push(blank(&format!("blank-{page:02}")));
        page += 1;
    }
    if self_ends {
        let ends = spec.endpapers.clone().unwrap_or_default();
        content.extend(endpapers("endpapers-back", ends.back.as_ref().or(ends.src.as_ref())));
        content.push(blank("pastedown-back"));
    }

    let faces = spec.fonts.stacks();
    let mut doc = json!({
        "matenadar": "1",
        "meta": {"title": spec.title, "author": spec.author, "lang": spec.lang.clone().unwrap_or_else(|| "en".into())},
        "fonts": spec.font_dirs,
        "design": {
            "colors": {"ink": "#1f1d1a"},
            "fonts": faces,
            "base": {"font": "text", "size": 14, "line_height": 1.35, "color": "ink"},
            "styles": {
                "title": {"font": "display", "size": 40, "weight": "bold", "line_height": 1.08, "align": "center", "balance": true},
                "subtitle": {"font": "display", "size": 17, "line_height": 1.25, "align": "center", "space_before": 12, "balance": true},
                "byline": {"size": 15, "line_height": 1.45, "align": "center"},
                "publisher": {"size": 10, "align": "center", "case": "upper", "tracking": "0.12em"},
                "dedication": {"size": 15, "italic": true, "line_height": 1.4, "align": "center", "balance": true},
                "small": {"size": 8.5, "line_height": 1.4},
                "story": {"size": 20, "line_height": 1.3, "align": "center", "balance": true},
                "heading": {"font": "display", "size": 22, "weight": "bold", "line_height": 1.2, "space_after": 12},
                "body": {"size": 12.5, "line_height": 1.45, "space_after": 7}
            },
            "masters": {"page": {
                "width": pt(geo.w), "height": pt(geo.h), "bleed": pt(geo.bleed),
                "margins": {"top": pt(geo.top), "bottom": pt(geo.top), "inside": pt(geo.inside), "outside": pt(geo.outside)}
            }}
        },
        "content": content
    });
    if let Some(over) = &spec.design {
        merge(&mut doc["design"], over);
    }
    Ok(doc)
}

fn page(id: &str, elements: Vec<Value>) -> Value {
    json!({"type": "page", "id": id, "master": "page", "elements": elements})
}

fn blank(id: &str) -> Value {
    page(id, Vec::new())
}

/// A picture to the page's bleed: the outside edge, top and bottom.
fn full_bleed(src: &str, geo: &Geo, recto: bool, crop: Option<Crop>, focus: Option<(f64, f64)>, alt: Option<&String>) -> Value {
    let mut img = json!({
        "type": "image", "src": src, "fit": "cover",
        "x": pt(if recto { 0.0 } else { -geo.bleed }), "y": pt(-geo.bleed),
        "w": pt(geo.w + geo.bleed), "h": pt(geo.h + 2.0 * geo.bleed)
    });
    if let Some(c) = crop {
        img["crop"] = json!(c);
    }
    if let Some(f) = focus {
        img["focus"] = json!(f);
    }
    if let Some(a) = alt {
        img["alt"] = json!(a);
    }
    img
}

/// Words over the art at the top of the page, kept in the safe zone.
fn words_over(id: &str, text: &str, geo: &Geo, recto: bool) -> Value {
    let (x, w) = geo.safe_x(recto);
    json!({
        "type": "text", "id": id, "x": pt(x), "y": pt(geo.top), "w": pt(w), "style": "story", "text": text, "safe": true,
        "scrim": {"opacity": 0.82, "feather": "10pt", "inset": "10pt", "radius": "10pt"}
    })
}

fn story_page(id: &str, s: &Story, geo: &Geo, recto: bool) -> Value {
    let mut elements = Vec::new();
    if let Some(src) = &s.src {
        elements.push(full_bleed(src, geo, recto, s.crop, s.focus, s.alt.as_ref()));
    }
    if let Some(t) = &s.text {
        elements.push(words_over(&format!("{id}-words"), t, geo, recto));
    }
    elements.extend(s.words.iter().cloned());
    page(id, elements)
}

fn spread(id: &str, s: &Story, geo: &Geo) -> Value {
    let side = |side: &Option<Side>, recto: bool| {
        let sd = side.clone().unwrap_or_default();
        let mut elements: Vec<Value> = Vec::new();
        if let Some(t) = &sd.text {
            elements.push(words_over(&format!("{id}-{}-words", if recto { "right" } else { "left" }), t, geo, recto));
        }
        elements.extend(sd.words.iter().cloned());
        let mut v = json!({"elements": elements});
        for (key, val) in [("src", json!(sd.src)), ("crop", json!(sd.crop)), ("focus", json!(sd.focus)), ("alt", json!(sd.alt))] {
            if !val.is_null() {
                v[key] = val;
            }
        }
        v
    };
    let mut v = json!({"type": "spread", "id": id, "master": "page", "left": side(&s.left, false), "right": side(&s.right, true)});
    for (key, val) in [("src", json!(s.src)), ("gutter", json!(s.gutter)), ("crop", json!(s.crop)), ("focus", json!(s.focus)), ("alt", json!(s.alt))] {
        if !val.is_null() {
            v[key] = val;
        }
    }
    v
}

/// Endpapers: a spread of their picture, or two plain pages.
fn endpapers(id: &str, src: Option<&String>) -> Vec<Value> {
    match src {
        Some(src) => vec![json!({"type": "spread", "id": id, "master": "page", "src": src, "left": {}, "right": {}})],
        None => vec![blank(&format!("{id}-left")), blank(&format!("{id}-right"))],
    }
}

fn half_title(spec: &PictureBook, geo: &Geo) -> Value {
    let (x, w) = geo.safe_x(true);
    page("half-title", vec![centred(x, geo.h * 0.36, w, "title", &spec.title, "half-title-words")])
}

fn title_page(spec: &PictureBook, geo: &Geo, id: &str, recto: bool) -> Value {
    let (x, w) = geo.safe_x(recto);
    let mut elements = Vec::new();
    if let Some(art) = &spec.title_art {
        elements.push(full_bleed(art, geo, recto, None, None, None));
    }
    let mut blocks = vec![json!({"type": "p", "style": "title", "text": spec.title})];
    if let Some(sub) = &spec.subtitle {
        blocks.push(json!({"type": "p", "style": "subtitle", "text": sub}));
    }
    let mut head = json!({"type": "text", "id": format!("{id}-words"), "x": pt(x), "y": pt(geo.h * 0.24), "w": pt(w), "blocks": blocks, "safe": true});
    let mut by = Vec::new();
    match (&spec.author, &spec.illustrator) {
        (Some(a), Some(i)) if a != i => {
            by.push(format!("written by {a}"));
            by.push(format!("illustrated by {i}"));
        }
        (Some(a), _) => by.push(a.clone()),
        (None, Some(i)) => by.push(format!("illustrated by {i}")),
        (None, None) => {}
    }
    let mut byline = (!by.is_empty()).then(|| {
        json!({"type": "text", "id": format!("{id}-byline"), "x": pt(x), "y": pt(geo.h * 0.62), "w": pt(w), "style": "byline", "text": by.join("\n"), "safe": true})
    });
    if spec.title_art.is_some() {
        // Over a picture, on soft panels.
        let scrim = json!({"opacity": 0.84, "feather": "12pt", "inset": "14pt", "radius": "12pt"});
        head["scrim"] = scrim.clone();
        if let Some(b) = byline.as_mut() {
            b["scrim"] = scrim;
        }
    }
    elements.push(head);
    elements.extend(byline);
    if let Some(p) = &spec.publisher {
        elements.push(json!({"type": "text", "id": format!("{id}-publisher"), "x": pt(x), "y": pt(geo.h - geo.top), "anchor": "bottom-left", "w": pt(w), "style": "publisher", "text": p}));
    }
    page(id, elements)
}

fn copyright_page(spec: &PictureBook, geo: &Geo, recto: bool) -> Value {
    let (x, w) = geo.safe_x(recto);
    let mut elements = Vec::new();
    if let Some(d) = &spec.dedication {
        elements.push(centred(x + w * 0.1, geo.h * 0.28, w * 0.8, "dedication", d, "dedication"));
    }
    if let Some(c) = &spec.copyright {
        elements.push(json!({"type": "text", "id": "copyright-words", "x": pt(x), "y": pt(geo.h - geo.top), "anchor": "bottom-left", "w": pt(w * 0.82), "style": "small", "text": c}));
    }
    page("copyright", elements)
}

fn back_page(id: &str, b: &Back, geo: &Geo, recto: bool) -> Value {
    let (x, w) = geo.safe_x(recto);
    let mut blocks = Vec::new();
    if let Some(hd) = &b.heading {
        blocks.push(json!({"type": "p", "style": "heading", "text": hd}));
    }
    for para in b.text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        blocks.push(json!({"type": "p", "style": "body", "text": para}));
    }
    page(id, vec![json!({"type": "text", "id": format!("{id}-words"), "x": pt(x), "y": pt(geo.top + 18.0), "w": pt(w), "h": pt(geo.h - 2.0 * geo.top - 18.0), "blocks": blocks, "fit": "shrink", "min_size": 8})])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Item;

    fn spec(v: Value) -> PictureBook {
        serde_json::from_value(v).unwrap()
    }

    fn pages_of(doc: &crate::Document) -> usize {
        doc.content.iter().map(|i| if matches!(i, Item::Spread(_)) { 2 } else { 1 }).sum()
    }

    fn story(n: usize) -> Value {
        Value::Array((0..n).map(|i| json!({"src": format!("art/p{i:02}.jpg")})).collect())
    }

    #[test]
    fn a_separate_ended_book_fills_every_page() {
        let s = spec(json!({"trim": "8x10", "pages": 32, "title": "T", "author": "A", "copyright": "©", "story": story(29)}));
        let v = compose(&s).unwrap();
        let doc = crate::Document::from_json(&v.to_string()).unwrap();
        assert_eq!(crate::validate(&doc), vec![]);
        assert_eq!(pages_of(&doc), 32);
        assert_eq!(doc.content[0].id(), "title", "page 1 is the title page");
        assert_eq!(doc.content[1].id(), "copyright", "its back the copyright");
        assert_eq!(doc.design.masters["page"].width, Some(Length::Pt(576.0)));
    }

    #[test]
    fn a_self_ended_book_keeps_its_endpapers_and_title_spread() {
        let s = spec(json!({"trim": "8.5x8.5", "pages": 32, "ends": "self", "title": "T", "endpapers": {"src": "art/ends.png"},
            "story": [{"src": "a"}, {"src": "b"}, {"spread": true, "src": "c"}, {"src": "d"}]}));
        let doc = crate::Document::from_json(&compose(&s).unwrap().to_string()).unwrap();
        assert_eq!(pages_of(&doc), 32);
        let ids: Vec<&str> = doc.content.iter().map(|i| i.id()).collect();
        assert_eq!(&ids[..5], &["pastedown-front", "endpapers-front", "copyright", "title", "story-01"]);
        assert_eq!(&ids[ids.len() - 2..], &["endpapers-back", "pastedown-back"]);
        assert!(matches!(&doc.content[6], Item::Spread(_)), "the spread opens on page 8");
    }

    #[test]
    fn a_spread_on_a_right_hand_page_or_too_long_a_story_is_refused() {
        let s = spec(json!({"trim": "8.5x11", "pages": 24, "title": "T", "story": [{"spread": true, "src": "b"}]}));
        let err = compose(&s).unwrap_err();
        assert!(err.contains("story[0] is a spread but would open on page 3"), "{err}");
        let s = spec(json!({"trim": "8.5x11", "pages": 24, "ends": "self", "title": "T", "story": story(20)}));
        let err = compose(&s).unwrap_err();
        assert!(err.contains("room to page 21"), "{err}");
        assert!(compose(&spec(json!({"trim": "6x9", "pages": 32, "title": "T", "story": []}))).is_err());
        assert!(compose(&spec(json!({"trim": "8x10", "pages": 30, "title": "T", "story": []}))).is_err());
    }
}
