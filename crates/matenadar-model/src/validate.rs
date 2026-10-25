//! Checks a document for what the type system can't: references that
//! resolve, unique ids, colours, sizes elements need, components that expand.

use std::collections::HashMap;

use serde::Serialize;

use crate::block::{Block, Inline};
use crate::element::Element;
use crate::style::{Border, Fill, Gradient, Stroke, TextStyle};
use crate::units::Color;
use crate::walk::{Node, walk};
use crate::{Document, Item};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Problem {
    pub path: String,
    pub message: String,
}

const STARTS: [&str; 4] = ["next", "recto", "verso", "continue"];
const NUMBERINGS: [&str; 5] = ["1", "i", "I", "a", "A"];

pub fn validate(doc: &Document) -> Vec<Problem> {
    let mut out = Vec::new();
    let d = &doc.design;
    let mut report = |path: &str, message: String| out.push(Problem { path: path.to_string(), message });

    // The design.
    let color_name = |n: &str| d.colors.contains_key(n);
    for (name, c) in &d.colors {
        if d.colors.contains_key(c.0.trim()) {
            report(&format!("design.colors.{name}"), "a colour can't name another colour".into());
        } else if let Err(e) = c.check(&|_| false) {
            report(&format!("design.colors.{name}"), e);
        }
    }
    for (name, s) in &d.styles {
        if let Some(parent) = &s.inherits
            && !d.styles.contains_key(parent)
        {
            report(&format!("design.styles.{name}.inherits"), format!("no style \"{parent}\""));
        }
        if d.style_cycle(name) {
            report(&format!("design.styles.{name}.inherits"), "styles inherit from each other in a circle".into());
        }
        check_text(&format!("design.styles.{name}"), &s.text, &color_name, &mut report);
        if let Some(f) = &s.fill {
            check_fill(&format!("design.styles.{name}.fill"), f, &color_name, &mut report);
        }
    }
    for (name, s) in &d.spans {
        check_text(&format!("design.spans.{name}"), s, &color_name, &mut report);
    }
    for (name, m) in &d.masters {
        let path = format!("design.masters.{name}");
        if let Some(parent) = &m.inherits
            && !d.masters.contains_key(parent)
        {
            report(&format!("{path}.inherits"), format!("no master \"{parent}\""));
        }
        let resolved = d.master(name).unwrap_or_default();
        if resolved.width.is_none() || resolved.height.is_none() {
            report(&path, "a master needs a width and a height (its own or inherited)".into());
        }
        if let Some(st) = &m.style
            && !d.styles.contains_key(st)
        {
            report(&format!("{path}.style"), format!("no style \"{st}\""));
        }
        if let Some(notes) = &m.notes
            && let Some(st) = &notes.style
            && !d.styles.contains_key(st)
        {
            report(&format!("{path}.notes.style"), format!("no style \"{st}\""));
        }
        if let Some(f) = &m.fill {
            check_fill(&format!("{path}.fill"), f, &color_name, &mut report);
        }
    }
    for (name, c) in &d.components {
        let defaults = c.params.iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.clone(), v.clone())).collect();
        let needed: Vec<_> = c.params.iter().filter(|(_, v)| v.is_null()).map(|(k, _)| k.clone()).collect();
        if needed.is_empty()
            && let Err(e) = c.expand(name, &defaults)
        {
            report(&format!("design.components.{name}"), e);
        }
    }
    if d.masters.is_empty() {
        report("design.masters", "the design needs at least one master (a page size)".into());
    }

    // The content.
    let mut ids: HashMap<String, String> = HashMap::new();
    let mut seen = |id: &str, path: &str, report: &mut dyn FnMut(&str, String)| {
        if id.trim().is_empty() {
            report(path, "an id can't be empty".into());
        } else if let Some(first) = ids.get(id) {
            report(path, format!("the id \"{id}\" is already used at {first}"));
        } else {
            ids.insert(id.to_string(), path.to_string());
        }
    };
    let has_style = |n: &str| d.styles.contains_key(n);
    walk(doc, &mut |path, node| match node {
        Node::Item(item) => {
            seen(item.id(), &format!("{path}.id"), &mut report);
            match item {
                Item::Section(s) => {
                    if !d.masters.contains_key(&s.master) {
                        report(&format!("{path}.master"), format!("no master \"{}\"", s.master));
                    }
                    check_word(&format!("{path}.start"), &s.start, &STARTS, &mut report);
                    check_word(&format!("{path}.numbering"), &s.numbering, &NUMBERINGS, &mut report);
                    if let Some(st) = &s.style
                        && !has_style(st)
                    {
                        report(&format!("{path}.style"), format!("no style \"{st}\""));
                    }
                }
                Item::Page(p) => {
                    if let Some(m) = &p.master
                        && !d.masters.contains_key(m)
                    {
                        report(&format!("{path}.master"), format!("no master \"{m}\""));
                    }
                    check_word(&format!("{path}.start"), &p.start, &STARTS[..3], &mut report);
                    check_word(&format!("{path}.numbering"), &p.numbering, &NUMBERINGS, &mut report);
                    if let Some(f) = &p.fill {
                        check_fill(&format!("{path}.fill"), f, &color_name, &mut report);
                    }
                }
            }
        }
        Node::Block(b) => {
            if let Some(id) = b.id() {
                seen(id, &format!("{path}.id"), &mut report);
            }
            for (field, st) in block_styles(b) {
                if !has_style(st) {
                    report(&format!("{path}.{field}"), format!("no style \"{st}\""));
                }
            }
            if let Block::Use(u) = b {
                match d.components.get(&u.component) {
                    None => report(&format!("{path}.component"), format!("no component \"{}\"", u.component)),
                    Some(c) => {
                        if let Err(e) = c.expand(&u.component, &u.props) {
                            report(path, e);
                        }
                    }
                }
            }
            if let Block::P(p) = b
                && let Some(set) = &p.set
            {
                check_text(&format!("{path}.set"), &set.text, &color_name, &mut report);
            }
        }
        Node::Element(e) => {
            if let Some(id) = e.id() {
                seen(id, &format!("{path}.id"), &mut report);
            }
            check_element(path, e, d, &color_name, &mut report);
        }
        Node::Inline(i) => match i {
            Inline::Run(r) => {
                if let Some(span) = &r.span
                    && !d.spans.contains_key(span)
                {
                    report(&format!("{path}.span"), format!("no character style \"{span}\""));
                }
                check_text(path, &r.set, &color_name, &mut report);
            }
            Inline::Field(f) => {
                let ok = matches!(f.field.as_str(), "page" | "pages" | "heading" | "doc.title" | "doc.author" | "section.title")
                    || f.field.strip_prefix("section.").is_some_and(|v| !v.is_empty());
                if !ok {
                    report(&format!("{path}.field"), format!("unknown field \"{}\"", f.field));
                }
            }
            _ => {}
        },
    });
    out
}

fn check_word(path: &str, value: &Option<String>, allowed: &[&str], report: &mut dyn FnMut(&str, String)) {
    if let Some(v) = value
        && !allowed.contains(&v.as_str())
    {
        report(path, format!("\"{v}\" is not one of {}", allowed.join(", ")));
    }
}

fn block_styles(b: &Block) -> Vec<(&'static str, &str)> {
    let mut v: Vec<(&'static str, &str)> = Vec::new();
    fn push<'a>(v: &mut Vec<(&'static str, &'a str)>, field: &'static str, s: &'a Option<String>) {
        if let Some(s) = s {
            v.push((field, s.as_str()));
        }
    }
    match b {
        Block::P(p) => push(&mut v, "style", &p.style),
        Block::H(h) => push(&mut v, "style", &h.style),
        Block::Break(x) => push(&mut v, "style", &x.style),
        Block::Figure(x) => push(&mut v, "caption_style", &x.caption_style),
        Block::Box(x) => push(&mut v, "style", &x.style),
        Block::Table(x) => {
            push(&mut v, "style", &x.style);
            push(&mut v, "header_style", &x.header_style);
        }
        Block::List(x) => push(&mut v, "style", &x.style),
        Block::Code(x) => push(&mut v, "style", &x.style),
        Block::Margin(x) => push(&mut v, "style", &x.style),
        _ => {}
    }
    v
}

fn check_element(
    path: &str,
    e: &Element,
    d: &crate::Design,
    color_name: &dyn Fn(&str) -> bool,
    report: &mut dyn FnMut(&str, String),
) {
    let need = |p: &crate::Place, w: bool, h: bool, report: &mut dyn FnMut(&str, String)| {
        if w && p.w.is_none() {
            report(path, format!("a {} element needs a width (w)", e.kind()));
        }
        if h && p.h.is_none() {
            report(path, format!("a {} element needs a height (h)", e.kind()));
        }
    };
    match e {
        Element::Text(t) => {
            if let Some(st) = &t.style
                && !d.styles.contains_key(st)
            {
                report(&format!("{path}.style"), format!("no style \"{st}\""));
            }
            if t.text.is_some() && (!t.blocks.is_empty() || !t.content.is_empty()) {
                report(path, "a text frame has either text, content or blocks, not several".into());
            }
            if let Some(f) = &t.fill {
                check_fill(&format!("{path}.fill"), f, color_name, report);
            }
            if let Some(b) = &t.stroke {
                check_border(&format!("{path}.stroke"), b, color_name, report);
            }
        }
        Element::Image(i) => {
            if i.place.w.is_none() && i.place.h.is_none() {
                report(path, "an image needs a width (w), a height (h) or both".into());
            }
            if i.src.is_empty() {
                report(&format!("{path}.src"), "an image needs a src".into());
            }
        }
        Element::Rect(s) | Element::Ellipse(s) => {
            need(&s.place, true, true, report);
            if let Some(f) = &s.fill {
                check_fill(&format!("{path}.fill"), f, color_name, report);
            }
            if let Some(b) = &s.stroke {
                check_border(&format!("{path}.stroke"), b, color_name, report);
            }
        }
        Element::Line(l) => {
            if l.x1.is_none() || l.y1.is_none() || l.x2.is_none() || l.y2.is_none() {
                report(path, "a line needs x1, y1, x2 and y2".into());
            }
            if let Some(s) = &l.stroke {
                check_stroke(&format!("{path}.stroke"), s, color_name, report);
            }
        }
        Element::Path(p) => need(&p.place, true, true, report),
        Element::Svg(s) => {
            if s.src.is_none() == s.markup.is_none() {
                report(path, "an svg element needs either src or markup".into());
            }
            if s.place.w.is_none() && s.place.h.is_none() {
                report(path, "an svg element needs a width (w), a height (h) or both".into());
            }
        }
        Element::Use(u) => match d.components.get(&u.component) {
            None => report(&format!("{path}.component"), format!("no component \"{}\"", u.component)),
            Some(c) => {
                if let Err(err) = c.expand(&u.component, &u.props) {
                    report(path, err);
                }
            }
        },
        Element::Qr(q) => {
            need(&q.place, true, false, report);
            if q.data.is_empty() {
                report(&format!("{path}.data"), "a QR code needs data".into());
            }
            if let Some(ecc) = &q.ecc
                && !matches!(ecc.as_str(), "L" | "M" | "Q" | "H")
            {
                report(&format!("{path}.ecc"), "ecc is L, M, Q or H".into());
            }
        }
        Element::Lettering(l) => {
            if l.text.is_empty() {
                report(&format!("{path}.text"), "lettering needs text".into());
            }
            if let Some(span) = &l.span
                && !d.spans.contains_key(span)
            {
                report(&format!("{path}.span"), format!("no character style \"{span}\""));
            }
            check_text(path, &l.style, color_name, report);
        }
        Element::LetterGrid(g) => {
            if g.cell.is_none() && g.place.w.is_none() {
                report(path, "a letter grid needs a cell size or a width".into());
            }
            if g.rows.is_empty() {
                report(&format!("{path}.rows"), "a letter grid needs rows".into());
            }
            if let Some(st) = &g.style
                && !d.styles.contains_key(st)
                && !d.spans.contains_key(st)
            {
                report(&format!("{path}.style"), format!("no style \"{st}\""));
            }
        }
        Element::Group(_) => {}
    }
}

fn check_text(path: &str, t: &TextStyle, color_name: &dyn Fn(&str) -> bool, report: &mut dyn FnMut(&str, String)) {
    if let Some(c) = &t.color {
        check_color(&format!("{path}.color"), c, color_name, report);
    }
    if let Some(s) = &t.outline {
        check_stroke(&format!("{path}.outline"), s, color_name, report);
    }
    if let Some(w) = &t.weight {
        let ok = match w {
            serde_json::Value::Number(n) => n.as_f64().is_some_and(|v| (1.0..=1000.0).contains(&v)),
            serde_json::Value::String(s) => {
                ["thin", "extralight", "light", "regular", "medium", "semibold", "bold", "extrabold", "black"].contains(&s.as_str())
            }
            _ => false,
        };
        if !ok {
            report(&format!("{path}.weight"), "weight is 100–900 or thin, extralight, light, regular, medium, semibold, bold, extrabold, black".into());
        }
    }
    if let Some(f) = &t.features {
        for feat in f {
            let tag = feat.trim_start_matches('-').split('=').next().unwrap_or("");
            if tag.len() != 4 || !tag.is_ascii() {
                report(&format!("{path}.features"), format!("\"{feat}\" is not an OpenType feature tag (four letters, such as \"onum\")"));
            }
        }
    }
}

fn check_color(path: &str, c: &Color, color_name: &dyn Fn(&str) -> bool, report: &mut dyn FnMut(&str, String)) {
    if let Err(e) = c.check(color_name) {
        report(path, e);
    }
}

fn check_stroke(path: &str, s: &Stroke, color_name: &dyn Fn(&str) -> bool, report: &mut dyn FnMut(&str, String)) {
    match s {
        Stroke::Color(c) => check_color(path, c, color_name, report),
        Stroke::Full(spec) => {
            if let Some(c) = &spec.color {
                check_color(&format!("{path}.color"), c, color_name, report);
            }
        }
    }
}

fn check_border(path: &str, b: &Border, color_name: &dyn Fn(&str) -> bool, report: &mut dyn FnMut(&str, String)) {
    match b {
        Border::All(s) => check_stroke(path, s, color_name, report),
        Border::Sides(sides) => {
            for (name, s) in [("top", &sides.top), ("right", &sides.right), ("bottom", &sides.bottom), ("left", &sides.left)] {
                if let Some(s) = s {
                    check_stroke(&format!("{path}.{name}"), s, color_name, report);
                }
            }
        }
    }
}

fn check_fill(path: &str, f: &Fill, color_name: &dyn Fn(&str) -> bool, report: &mut dyn FnMut(&str, String)) {
    match f {
        Fill::Color(c) => check_color(path, c, color_name, report),
        Fill::Gradient(Gradient::Linear { stops, .. } | Gradient::Radial { stops, .. }) => {
            if stops.len() < 2 {
                report(path, "a gradient needs at least two stops".into());
            }
            for (i, (c, _)) in stops.iter().enumerate() {
                check_color(&format!("{path}.stops[{i}]"), c, color_name, report);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc(content: serde_json::Value) -> Document {
        serde_json::from_value(json!({
            "matenadar": "1",
            "meta": {"title": "T"},
            "design": {
                "colors": {"ink": "#111"},
                "styles": {"body": {"size": 10}},
                "masters": {"page": {"width": "6in", "height": "9in"}}
            },
            "content": content
        }))
        .unwrap()
    }

    #[test]
    fn a_valid_document_has_no_problems() {
        let d = doc(json!([
            {"type": "section", "id": "s1", "master": "page", "flow": [{"type": "p", "id": "p1", "style": "body", "text": "Hi"}]},
            {"type": "page", "id": "pg", "master": "page", "elements": [{"type": "rect", "x": 0, "y": 0, "w": 10, "h": 10, "fill": "ink"}]}
        ]));
        assert_eq!(validate(&d), vec![]);
    }

    #[test]
    fn reports_missing_references_duplicate_ids_and_bad_colours() {
        let d = doc(json!([
            {"type": "section", "id": "s1", "master": "nope", "flow": [
                {"type": "p", "id": "x", "style": "lede", "text": "Hi"},
                {"type": "p", "id": "x", "content": [{"text": "a", "span": "caps", "color": "paper"}]}
            ]},
            {"type": "page", "id": "pg", "master": "page", "elements": [{"type": "rect", "x": 0, "y": 0, "w": 10}]}
        ]));
        let msgs: Vec<String> = validate(&d).into_iter().map(|p| format!("{}: {}", p.path, p.message)).collect();
        assert!(msgs.contains(&"content[0].master: no master \"nope\"".to_string()), "{msgs:?}");
        assert!(msgs.contains(&"content[0].flow[0].style: no style \"lede\"".to_string()), "{msgs:?}");
        assert!(msgs.iter().any(|m| m.contains("the id \"x\" is already used")), "{msgs:?}");
        assert!(msgs.contains(&"content[0].flow[1].content[0].span: no character style \"caps\"".to_string()), "{msgs:?}");
        assert!(msgs.iter().any(|m| m.contains("unknown colour \"paper\"")), "{msgs:?}");
        assert!(msgs.iter().any(|m| m.contains("needs a height")), "{msgs:?}");
    }
}
