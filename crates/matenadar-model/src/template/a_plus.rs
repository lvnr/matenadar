//! The A+ template: a book's A+ modules as a store shows them, for a proof
//! before upload. One page as wide as the store's module area (970 px on a
//! desktop, 480 px on a phone) that grows with the modules. Each module's
//! pictures are set in their upload slots, at the store's sizes and
//! cropped as it crops them, with the module's words in the store's places.

use serde::Deserialize;
use serde_json::{Value, json};

use super::merge;
use crate::units::num;

/// The store's upload size of each layout's pictures, in pixels.
pub const SLOTS: [(&str, f64, f64); 11] = [
    ("image-header-with-text", 970.0, 600.0),
    ("image-text-overlay", 970.0, 300.0),
    ("single-left-image", 300.0, 300.0),
    ("single-right-image", 300.0, 300.0),
    ("single-image-highlights", 300.0, 300.0),
    ("single-image-sidebar", 300.0, 400.0),
    ("three-images-text", 300.0, 300.0),
    ("four-images-text", 220.0, 220.0),
    ("comparison-chart", 150.0, 300.0),
    ("company-logo", 600.0, 180.0),
    ("multiple-images", 300.0, 300.0),
];

/// A layout's slot in pixels (layouts the store has added since: 970 × 300).
pub fn slot(layout: &str) -> (f64, f64) {
    SLOTS
        .iter()
        .find(|(name, _, _)| *name == layout)
        .map(|(_, w, h)| (*w, *h))
        .unwrap_or((970.0, 300.0))
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct APlus {
    /// `"desktop"` (default, 970 px), `"mobile"` (480 px), or a width in pixels.
    #[serde(default)]
    pub width: Option<Value>,
    #[serde(default)]
    pub title: Option<String>,
    /// The store the proof is for, shown in its heading (`amazon.com`).
    #[serde(default)]
    pub marketplace: Option<String>,
    /// A heading over the modules naming the proof (default true).
    #[serde(default)]
    pub heading: Option<bool>,
    pub modules: Vec<Module>,
    /// The text's family (default the store's sans: Amazon Ember, then Arial and its likes).
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default)]
    pub font_dirs: Vec<String>,
    #[serde(default)]
    pub lang: Option<String>,
    /// Changes to the template's design, merged into it object by object.
    #[serde(default)]
    pub design: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Module {
    pub id: Option<String>,
    /// The store's layout name: `image-header-with-text`, `image-text-overlay`,
    /// `single-left-image`, `single-right-image`, `three-images-text`,
    /// `four-images-text`, `comparison-chart`, `company-logo`, …
    pub layout: String,
    /// A small line over the module (its name, what it does for the buyer).
    pub label: Option<String>,
    pub heading: Option<String>,
    /// The module's words; a blank line starts a paragraph.
    pub body: Option<String>,
    pub images: Vec<Picture>,
    /// A table under the words (a comparison's or a book's details).
    pub rows: Vec<Row>,
    /// The books a comparison chart sets side by side.
    pub comparisons: Vec<Compared>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Picture {
    pub src: String,
    pub alt: Option<String>,
    pub caption: Option<String>,
    /// The upload's own size in pixels, where the store has changed the slot.
    pub w: Option<f64>,
    pub h: Option<f64>,
    /// Where the crop keeps the picture: `[0.5, 0.5]` is its centre.
    pub focus: Option<[f64; 2]>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Row {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Compared {
    pub title: String,
    pub asin: Option<String>,
    pub src: Option<String>,
}

fn px(v: f64) -> String {
    format!("{}px", num(v))
}

pub fn compose(spec: &APlus) -> Result<Value, String> {
    let width = match &spec.width {
        None => 970.0,
        Some(Value::String(s)) if s == "desktop" => 970.0,
        Some(Value::String(s)) if s == "mobile" => 480.0,
        Some(Value::Number(n)) if n.as_f64().is_some_and(|v| (240.0..=1940.0).contains(&v)) => {
            n.as_f64().unwrap_or(970.0)
        }
        Some(other) => {
            return Err(format!(
                "width is \"desktop\", \"mobile\" or 240-1940 pixels, not {other}"
            ));
        }
    };
    if spec.modules.is_empty() {
        return Err("an A+ proof needs modules: [{\"layout\": …}]".into());
    }
    if spec.modules.len() > 7 {
        return Err(format!(
            "the store takes up to 7 modules, not {}",
            spec.modules.len()
        ));
    }
    let wide = width >= 600.0;
    let (pad_x, pad_y) = if wide { (34.0, 28.0) } else { (16.0, 20.0) };
    let inner = width - 2.0 * pad_x;
    let mut flow: Vec<Value> = Vec::new();

    if spec.heading != Some(false) {
        let mut sub = vec![spec.title.clone().unwrap_or_default()];
        if let Some(m) = &spec.marketplace {
            sub.push(m.clone());
        }
        let sub: Vec<String> = sub.into_iter().filter(|s| !s.trim().is_empty()).collect();
        let mut blocks = vec![
            json!({"type": "p", "id": "proof-head", "style": "proof-head", "text": "A+ proof"}),
        ];
        if !sub.is_empty() {
            blocks.push(json!({"type": "p", "id": "proof-sub", "style": "proof-sub", "text": sub.join(" · ")}));
        }
        flow.push(json!({"type": "box", "id": "proof", "fill": "head", "inset": {"x": px(pad_x), "y": px(16.0)}, "w": "100%", "blocks": blocks}));
    }

    for (i, m) in spec.modules.iter().enumerate() {
        let id =
            m.id.clone()
                .unwrap_or_else(|| format!("module-{:02}", i + 1));
        if i > 0 || spec.heading != Some(false) {
            flow.push(json!({"type": "box", "id": format!("{id}-gap"), "fill": "gap", "w": "100%", "h": px(if wide { 12.0 } else { 8.0 })}));
        }
        let (sw, sh) = slot(&m.layout);
        let picture = |p: &Picture, k: usize, w: f64| -> Value {
            let (pw, ph) = (p.w.unwrap_or(sw), p.h.unwrap_or(sh));
            let mut f = json!({"type": "figure", "id": format!("{id}-image-{}", k + 1), "src": p.src, "w": px(w), "h": px(w * ph / pw), "fit": "cover", "align": "center"});
            if let Some(a) = &p.alt {
                f["alt"] = json!(a);
            }
            if let Some(c) = &p.caption {
                f["caption"] = json!(c);
                f["caption_style"] = json!("caption");
            }
            if let Some(focus) = p.focus {
                f["focus"] = json!(focus);
            }
            f
        };
        let words = |with_heading: bool| -> Vec<Value> {
            let mut out = Vec::new();
            if with_heading && let Some(h) = m.heading.as_ref().filter(|h| !h.trim().is_empty()) {
                out.push(json!({"type": "h", "id": format!("{id}-heading"), "level": 2, "style": "heading", "outline": false, "text": h}));
            }
            for (k, para) in m
                .body
                .as_deref()
                .unwrap_or("")
                .split("\n\n")
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .enumerate()
            {
                out.push(json!({"type": "p", "id": format!("{id}-body-{}", k + 1), "style": "body", "text": para}));
            }
            out
        };
        let mut blocks: Vec<Value> = Vec::new();
        let full_width = matches!(
            m.layout.as_str(),
            "image-header-with-text" | "image-text-overlay"
        ) && !m.images.is_empty();
        if let Some(l) = &m.label {
            let label =
                json!({"type": "p", "id": format!("{id}-label"), "style": "label", "text": l});
            if full_width {
                // Over a picture that runs the width, the label has a strip of its own.
                flow.push(json!({"type": "box", "id": format!("{id}-head"), "w": "100%", "inset": {"x": px(pad_x), "top": px(pad_y), "bottom": 0}, "blocks": [label]}));
            } else {
                blocks.push(label);
            }
        }
        match m.layout.as_str() {
            _ if full_width => {
                // The picture runs the module's width, the words under it (or over it).
                let p = &m.images[0];
                let (pw, ph) = (p.w.unwrap_or(sw), p.h.unwrap_or(sh));
                let h = width * ph / pw;
                if m.layout == "image-text-overlay" && wide {
                    let mut art = json!({"type": "image", "id": format!("{id}-image-1"), "src": p.src, "x": 0, "y": 0, "w": px(width), "h": px(h), "fit": "cover"});
                    if let Some(a) = &p.alt {
                        art["alt"] = json!(a);
                    }
                    if let Some(focus) = p.focus {
                        art["focus"] = json!(focus);
                    }
                    let frame = json!({"type": "text", "id": format!("{id}-words"), "x": px(width * 0.4), "y": px(24.0), "w": px(width * 0.6 - 24.0), "h": px(h - 48.0),
                        "valign": "middle", "fill": "panel", "inset": px(16.0), "blocks": words(true)});
                    flow.push(json!({"type": "canvas", "id": format!("{id}-art"), "w": px(width), "h": px(h), "clip": true, "elements": [art, frame]}));
                } else {
                    let mut f = picture(p, 0, width);
                    f["id"] = json!(format!("{id}-image-1"));
                    flow.push(f);
                    blocks.extend(words(true));
                }
            }
            "single-left-image"
            | "single-right-image"
            | "single-image-sidebar"
            | "single-image-highlights"
                if wide && !m.images.is_empty() =>
            {
                let w = sw.min(inner * 0.45);
                let image = json!({"blocks": [picture(&m.images[0], 0, w)]});
                let text = json!({"blocks": words(true)});
                let cells = if m.layout == "single-right-image" {
                    vec![text, image]
                } else {
                    vec![image, text]
                };
                let cols = if m.layout == "single-right-image" {
                    json!(["1fr", px(w)])
                } else {
                    json!([px(w), "1fr"])
                };
                blocks.push(json!({"type": "table", "id": format!("{id}-row"), "columns": cols, "rows": [cells], "stroke": "none",
                    "inset": {"x": px(10.0), "y": 0}, "valign": "top"}));
            }
            _ => {
                blocks.extend(words(true));
                if !m.images.is_empty() {
                    let n = m.images.len().min(if wide { 4 } else { 1 });
                    let gap = 16.0;
                    // A picture's share of the width, no wider than its slot on a
                    // desktop (a phone scales every picture to the screen).
                    let share = (inner - gap * (n as f64 - 1.0)) / n as f64;
                    let w = if wide { share.min(sw) } else { share };
                    let figures: Vec<Value> = m
                        .images
                        .iter()
                        .enumerate()
                        .map(|(k, p)| picture(p, k, w))
                        .collect();
                    if n == 1 {
                        blocks.extend(figures);
                    } else {
                        let rows: Vec<Vec<Value>> = figures
                            .chunks(n)
                            .map(|c| c.iter().map(|f| json!({"blocks": [f]})).collect())
                            .collect();
                        blocks.push(json!({"type": "table", "id": format!("{id}-images"), "columns": vec![json!("1fr"); n], "rows": rows,
                            "stroke": "none", "inset": {"x": px(gap / 2.0), "y": px(6.0)}, "valign": "top"}));
                    }
                }
            }
        }
        if !m.comparisons.is_empty() {
            let n = m.comparisons.len().min(if wide { 6 } else { 2 });
            let cells: Vec<Value> = m
                .comparisons
                .iter()
                .enumerate()
                .map(|(k, c)| {
                    let mut b = Vec::new();
                    if let Some(src) = &c.src {
                        let w = (inner / n as f64 - 12.0).min(sw);
                        b.push(json!({"type": "figure", "id": format!("{id}-compared-{}-image", k + 1), "src": src, "w": px(w), "h": px(w * sh / sw), "fit": "cover", "align": "center"}));
                    }
                    b.push(json!({"type": "p", "id": format!("{id}-compared-{}", k + 1), "style": "compared", "text": c.title}));
                    if let Some(a) = &c.asin {
                        b.push(json!({"type": "p", "id": format!("{id}-compared-{}-asin", k + 1), "style": "small", "text": a}));
                    }
                    json!({"blocks": b})
                })
                .collect();
            let rows: Vec<Vec<Value>> = cells.chunks(n).map(|c| c.to_vec()).collect();
            blocks.push(json!({"type": "table", "id": format!("{id}-compared"), "columns": vec![json!("1fr"); n], "rows": rows,
                "stroke": {"width": "1px", "color": "rule"}, "inset": px(10.0), "valign": "top"}));
        }
        if !m.rows.is_empty() {
            let rows: Vec<Value> = m
                .rows
                .iter()
                .map(|r| json!([{"text": r.label, "style": "row-label"}, r.value]))
                .collect();
            blocks.push(json!({"type": "table", "id": format!("{id}-rows"), "columns": ["auto", "1fr"], "rows": rows,
                "stroke": {"width": "1px", "color": "rule"}, "inset": px(10.0)}));
        }
        if !blocks.is_empty() {
            flow.push(json!({"type": "box", "id": id, "w": "100%", "inset": {"x": px(pad_x), "y": px(pad_y)}, "blocks": blocks}));
        }
    }

    let font = spec.font.clone().map(|f| json!([f])).unwrap_or_else(|| {
        json!([
            "Amazon Ember",
            "Arial",
            "Arimo",
            "Liberation Sans",
            "Helvetica",
            "Nimbus Sans",
            "DejaVu Sans"
        ])
    });
    let mut font = font;
    font.as_array_mut()
        .map(|a| a.push(json!("Libertinus Serif")));
    let (size, heading) = if wide { (16.0, 30.0) } else { (15.0, 24.0) };
    let mut doc = json!({
        "matenadar": "1",
        "meta": {"title": spec.title.clone().unwrap_or_else(|| "A+ proof".into()), "lang": spec.lang.clone().unwrap_or_else(|| "en-US".into())},
        "fonts": spec.font_dirs,
        "design": {
            "colors": {"ink": "#171717", "muted": "#666666", "head": "#222222", "gap": "#ececec", "rule": "#bbbbbb", "panel": "#ffffffdb", "paper": "#ffffff"},
            "fonts": {"text": font},
            "base": {"font": "text", "size": px(size), "line_height": 1.5, "color": "ink", "hyphenate": false},
            "styles": {
                "proof-head": {"size": px(14.0), "weight": "bold", "case": "upper", "tracking": "0.06em", "color": "#ffffff"},
                "proof-sub": {"size": px(13.0), "color": "#ffffff", "space_before": px(3.0)},
                "label": {"size": px(12.0), "color": "muted", "space_after": px(14.0)},
                "heading": {"size": px(heading), "line_height": 1.14, "weight": "bold", "space_before": px(18.0), "space_after": px(10.0)},
                "body": {"space_after": px(8.0)},
                "caption": {"size": px(14.0), "space_before": px(7.0)},
                "compared": {"weight": "bold", "space_before": px(6.0)},
                "row-label": {"weight": "bold"},
                "small": {"size": px(12.0), "color": "muted"}
            },
            "masters": {"page": {"width": px(width), "grow": true, "margins": {"top": 0, "bottom": 0, "left": 0, "right": 0}, "fill": "paper"}}
        },
        "content": [{"type": "section", "id": "a-plus", "master": "page", "flow": flow}]
    });
    if let Some(over) = &spec.design {
        merge(&mut doc["design"], over);
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn compose(spec: Value) -> Value {
        serde_json::to_value(crate::template::compose("a-plus", &spec).expect("composes")).unwrap()
    }

    #[test]
    fn modules_are_set_at_the_stores_slots_on_a_page_that_grows() {
        let doc = compose(json!({"title": "Lights", "modules": [
            {"layout": "image-header-with-text", "heading": "A winter of lanterns", "body": "One.\n\nTwo.", "images": [{"src": "a.jpg", "alt": "Lanterns"}]},
            {"layout": "three-images-text", "images": [{"src": "b.jpg"}, {"src": "c.jpg"}, {"src": "d.jpg"}]}
        ]}));
        let master = &doc["design"]["masters"]["page"];
        // Lengths come back in points: 970 px is 727.5 pt.
        assert_eq!(
            (master["width"].clone(), master["grow"].clone()),
            (json!(727.5), json!(true))
        );
        let flow = doc["content"][0]["flow"].as_array().unwrap();
        // The header's picture runs the width at the store's 970 × 600.
        let header = flow
            .iter()
            .find(|b| b["id"] == "module-01-image-1")
            .unwrap();
        assert_eq!(
            (
                header["w"].clone(),
                header["h"].clone(),
                header["fit"].clone()
            ),
            (json!(727.5), json!(450.0), json!("cover"))
        );
        // Three pictures side by side, each in a third of the text width, square as the slot.
        let row = flow.iter().find(|b| b["id"] == "module-02").unwrap()["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["id"] == "module-02-images")
            .unwrap()
            .clone();
        let fig = &row["rows"][0][2]["blocks"][0];
        assert_eq!(fig["w"], fig["h"], "{fig}");
    }

    #[test]
    fn a_phone_stacks_what_a_desktop_sets_side_by_side() {
        let doc = compose(json!({"width": "mobile", "heading": false, "modules": [
            {"layout": "single-left-image", "heading": "H", "images": [{"src": "a.jpg"}]},
            {"layout": "image-text-overlay", "heading": "Over", "images": [{"src": "b.jpg"}]}
        ]}));
        assert_eq!(doc["design"]["masters"]["page"]["width"], json!(360.0));
        let text = doc.to_string();
        assert!(
            !text.contains("module-01-row"),
            "no side-by-side row on a phone"
        );
        assert!(
            !text.contains("module-02-words"),
            "the overlay's words go under its picture on a phone"
        );
        // One picture in a layout of square slots keeps the slot's size on a desktop.
        let one = compose(json!({"modules": [{"layout": "three-images-text", "images": [{"src": "a.jpg"}]}]})).to_string();
        assert!(one.contains("\"w\":225.0"), "300 px is 225 pt: {one}");
        let err = crate::template::compose(
            "a-plus",
            &json!({"width": 90, "modules": [{"layout": "x"}]}),
        )
        .unwrap_err();
        assert!(err.contains("width"), "{err}");
    }
}
