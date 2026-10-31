//! The coloring-book template. Each picture has a right-hand page to itself
//! and its back is left blank, so markers that bleed through spoil nothing
//! (`single_sided`, the default). Front matter is typeset: the title page,
//! the copyright page, and a "this book belongs to" page; a page of
//! squares to try colors on can close the book.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Faces, centred, merge, points, pt, trim};
use crate::units::Length;

pub const TRIMS: [&str; 5] = ["8.5x11", "8x10", "8.5x8.5", "7x10", "6x9"];

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coloring {
    /// `"8.5x11"` (default), `"8x10"`, `"8.5x8.5"`, `"7x10"` or `"6x9"`.
    #[serde(default)]
    pub trim: Option<String>,
    /// Pictures that run off the page: a 0.125 in bleed on the top, bottom
    /// and outside edges (default: none, every picture inside the margins).
    #[serde(default)]
    pub bleed: bool,
    /// The space round each picture (default 0.5 in), and at the spine
    /// (default 0.75 in).
    #[serde(default)]
    pub margin: Option<Length>,
    #[serde(default)]
    pub inside: Option<Length>,
    pub title: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub publisher: Option<String>,
    /// The copyright page's words (a newline breaks the line).
    #[serde(default)]
    pub copyright: Option<String>,
    /// A "this book belongs to" page (default true), or its words.
    #[serde(default)]
    pub belongs_to: Option<Value>,
    /// Each picture on a right-hand page, its back blank (default true).
    #[serde(default)]
    pub single_sided: Option<bool>,
    /// Words on every blank back, small at its foot.
    #[serde(default)]
    pub back_note: Option<String>,
    /// Page numbers on the pictures' pages.
    #[serde(default)]
    pub folios: bool,
    /// A page of squares to try colors on, at the end: true, or its heading.
    #[serde(default)]
    pub test_page: Option<Value>,
    pub pages: Vec<Picture>,
    #[serde(default)]
    pub fonts: Faces,
    #[serde(default)]
    pub font_dirs: Vec<String>,
    #[serde(default)]
    pub lang: Option<String>,
    /// Changes to the template's design, merged into it object by object.
    #[serde(default)]
    pub design: Option<Value>,
}

/// A picture to colour.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Picture {
    pub id: Option<String>,
    pub src: String,
    /// A line above the picture.
    pub title: Option<String>,
    pub alt: Option<String>,
    /// The whole page rather than inside the margins: to the bleed with
    /// `bleed`, else to the trim (art drawn with its own white edge).
    pub full: bool,
}

pub fn compose(spec: &Coloring) -> Result<Value, String> {
    let (w, h) = trim(spec.trim.as_deref().unwrap_or("8.5x11"), &TRIMS)?;
    let bleed = if spec.bleed { 9.0 } else { 0.0 };
    let margin = points(spec.margin, 36.0);
    let inside = points(spec.inside, 54.0);
    let single = spec.single_sided.unwrap_or(true);
    if spec.pages.is_empty() {
        return Err("a coloring book needs pages: [{\"src\": …}]".into());
    }
    let safe = |recto: bool| if recto { (inside, w - inside - margin) } else { (margin, w - inside - margin) };
    let mut content: Vec<Value> = Vec::new();

    // The title page, its back the copyright.
    let (x, sw) = safe(true);
    let mut title = vec![centred(x, h * 0.3, sw, "title", &spec.title, "title-title")];
    if let Some(s) = &spec.subtitle {
        title.push(centred(x, h * 0.3 + 120.0, sw, "subtitle", s, "title-subtitle"));
    }
    if let Some(a) = &spec.author {
        title.push(centred(x, h * 0.62, sw, "byline", a, "title-author"));
    }
    if let Some(p) = &spec.publisher {
        title.push(centred(x, h - margin - 24.0, sw, "publisher", p, "title-publisher"));
    }
    content.push(page("title", title, false));
    let (x, sw) = safe(false);
    let words = spec.copyright.clone().unwrap_or_else(|| format!("{}\nAll rights reserved.", spec.title));
    content.push(page(
        "copyright",
        vec![json!({"type": "text", "id": "copyright-words", "x": pt(x), "y": pt(h - margin - 110.0), "w": pt(sw), "h": pt(110.0), "valign": "bottom", "style": "small", "text": words})],
        false,
    ));
    let belongs = match &spec.belongs_to {
        None | Some(Value::Bool(true)) => Some("This book belongs to".to_string()),
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Bool(false)) | Some(Value::Null) => None,
        Some(_) => return Err("belongs_to is true, false or its words".into()),
    };
    if let Some(b) = belongs {
        let (x, sw) = safe(true);
        content.push(page(
            "belongs-to",
            vec![
                centred(x, h * 0.36, sw, "belongs", &b, "belongs-to-words"),
                json!({"type": "line", "id": "belongs-to-line", "x1": pt(x + sw * 0.15), "y1": pt(h * 0.5), "x2": pt(x + sw * 0.85), "y2": pt(h * 0.5), "stroke": {"width": 0.8, "color": "ink"}}),
            ],
            false,
        ));
    }
    // Pictures open on a right-hand page.
    if single && content.len() % 2 == 1 {
        content.push(page("front-blank", Vec::new(), false));
    }
    for (i, p) in spec.pages.iter().enumerate() {
        let id = p.id.clone().unwrap_or_else(|| format!("picture-{:02}", i + 1));
        let recto = single || content.len().is_multiple_of(2);
        let (x, sw) = safe(recto);
        let mut elements = Vec::new();
        let top = match &p.title {
            Some(t) => {
                elements.push(centred(x, margin, sw, "picture-title", t, &format!("{id}-title")));
                margin + 42.0
            }
            None => margin,
        };
        let mut img = if p.full {
            json!({"type": "image", "id": format!("{id}-art"), "src": p.src, "fit": "cover",
                "x": pt(if recto { 0.0 } else { -bleed }), "y": pt(-bleed), "w": pt(w + bleed), "h": pt(h + 2.0 * bleed)})
        } else {
            let bottom = if spec.folios { margin + 18.0 } else { margin };
            json!({"type": "image", "id": format!("{id}-art"), "src": p.src, "fit": "contain",
                "x": pt(x), "y": pt(top), "w": pt(sw), "h": pt(h - top - bottom)})
        };
        if let Some(a) = &p.alt {
            img["alt"] = json!(a);
        }
        elements.insert(0, img);
        content.push(page(&id, elements, spec.folios));
        if single {
            let note = spec.back_note.as_ref().map(|n| {
                let (x, sw) = safe(false);
                vec![centred(x, h - margin - 14.0, sw, "back-note", n, &format!("{id}-back-note"))]
            });
            content.push(page(&format!("{id}-back"), note.unwrap_or_default(), false));
        }
    }
    let test = match &spec.test_page {
        None | Some(Value::Bool(false)) | Some(Value::Null) => None,
        Some(Value::Bool(true)) => Some("Try your colors here".to_string()),
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => return Err("test_page is true, false or its heading".into()),
    };
    if let Some(heading) = test {
        let recto = content.len().is_multiple_of(2);
        let (x, sw) = safe(recto);
        let (cols, size) = (5usize, (sw - 4.0 * 14.0) / 5.0);
        let rows = ((h - 2.0 * margin - 60.0 + 14.0) / (size + 14.0)).floor().max(1.0) as usize;
        let mut els = vec![centred(x, margin, sw, "picture-title", &heading, "test-title")];
        for r in 0..rows {
            for c in 0..cols {
                els.push(json!({"type": "rect", "x": pt(x + c as f64 * (size + 14.0)), "y": pt(margin + 60.0 + r as f64 * (size + 14.0)),
                    "w": pt(size), "h": pt(size), "stroke": {"width": 0.8, "color": "ink"}, "radius": 4}));
            }
        }
        content.push(page("test-colours", els, false));
        if single {
            content.push(page("test-colours-back", Vec::new(), false));
        }
    }

    let faces = spec.fonts.stacks();
    let mut doc = json!({
        "matenadar": "1",
        "meta": {"title": spec.title, "author": spec.author, "lang": spec.lang.clone().unwrap_or_else(|| "en".into())},
        "fonts": spec.font_dirs,
        "design": {
            "colors": {"ink": "#000000"},
            "fonts": faces,
            "base": {"font": "text", "size": 12, "line_height": 1.4, "color": "ink"},
            "styles": {
                "title": {"font": "display", "size": 44, "weight": "bold", "line_height": 1.08, "align": "center", "balance": true},
                "subtitle": {"font": "display", "size": 18, "line_height": 1.25, "align": "center", "balance": true},
                "byline": {"size": 16, "align": "center"},
                "publisher": {"size": 10, "align": "center", "case": "upper", "tracking": "0.12em"},
                "small": {"size": 9, "line_height": 1.4},
                "belongs": {"font": "display", "size": 24, "align": "center"},
                "picture-title": {"font": "display", "size": 22, "align": "center"},
                "back-note": {"size": 8, "align": "center", "color": "#777777"},
                "folio": {"size": 9, "align": "center"}
            },
            "masters": {"page": {
                "width": pt(w), "height": pt(h), "bleed": pt(bleed),
                "margins": {"top": pt(margin), "bottom": pt(margin), "inside": pt(inside), "outside": pt(margin)},
                "foreground": [{"type": "text", "id": "folio", "style": "folio", "x": pt(margin), "y": pt(h - margin + 2.0), "w": pt(w - 2.0 * margin), "content": [{"field": "page"}]}]
            }}
        },
        "content": content
    });
    if let Some(over) = &spec.design {
        merge(&mut doc["design"], over);
    }
    Ok(doc)
}

fn page(id: &str, elements: Vec<Value>, layers: bool) -> Value {
    let mut p = json!({"type": "page", "id": id, "master": "page", "elements": elements});
    if layers {
        p["layers"] = json!(true);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn plan(spec: Value) -> Vec<String> {
        let doc = crate::template::compose("coloring", &spec).expect("composes");
        doc.content.iter().map(|i| i.id().to_string()).collect()
    }

    #[test]
    fn pictures_open_on_right_hand_pages_with_blank_backs() {
        let ids = plan(json!({"title": "Lights", "pages": [{"src": "a.svg"}, {"src": "b.svg", "title": "Lanterns"}]}));
        assert_eq!(ids, ["title", "copyright", "belongs-to", "front-blank", "picture-01", "picture-01-back", "picture-02", "picture-02-back"]);
        // Page 5 is the first picture: a right-hand page.
        assert_eq!(ids.iter().position(|i| i == "picture-01").unwrap() % 2, 0);
    }

    #[test]
    fn double_sided_books_and_their_checks() {
        let ids = plan(json!({"title": "Lights", "belongs_to": false, "single_sided": false, "test_page": true, "pages": [{"src": "a.svg"}, {"src": "b.svg"}]}));
        assert_eq!(ids, ["title", "copyright", "picture-01", "picture-02", "test-colours"]);
        let doc = crate::template::compose("coloring", &json!({"title": "x", "pages": [{"src": "a.png", "full": true}]})).unwrap();
        let art = serde_json::to_value(&doc.content[4]).unwrap();
        assert_eq!((art["elements"][0]["x"].clone(), art["elements"][0]["w"].clone()), (json!(0.0), json!(612.0)), "to the trim: {art}");
        let err = crate::template::compose("coloring", &json!({"title": "x", "trim": "5x8", "pages": [{"src": "a.png"}]})).unwrap_err();
        assert!(err.contains("trim \"5x8\""), "{err}");
    }
}
