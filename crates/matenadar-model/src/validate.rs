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
const STROKE_ALIGNS: [&str; 3] = ["center", "inside", "outside"];
const ORIGINS: [&str; 9] = ["top-left", "top", "top-right", "left", "center", "right", "bottom-left", "bottom", "bottom-right"];

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
        check_word(&format!("design.styles.{name}.linebreaks"), &s.linebreaks, &["simple", "optimized"], &mut report);
        check_word(&format!("design.styles.{name}.wrap"), &s.wrap, &["normal", "pretty"], &mut report);
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
        if resolved.width.is_none() || (resolved.height.is_none() && resolved.grow != Some(true)) {
            report(&path, "a master needs a width and a height (its own or inherited), or a width and grow".into());
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
    for (i, w) in d.hyphenation.iter().flat_map(|h| h.exceptions.iter()).enumerate() {
        if w.is_empty() || !w.chars().all(|c| c.is_alphabetic() || c == '-') || w.starts_with('-') || w.ends_with('-') {
            report(&format!("design.hyphenation.exceptions[{i}]"), format!("\"{w}\" should be one word, with hyphens only where it may break"));
        }
    }

    for (id, on) in &doc.on_page {
        if on.hide.iter().any(|h| h.trim().is_empty()) {
            report(&format!("on_page.{id}.hide"), "hide lists the ids of master elements".into());
        }
    }
    if let Some(f) = &d.figures {
        check_word("design.figures.per", &f.per, &["book", "chapter"], &mut report);
        if let Some(sp) = &f.span
            && !d.spans.contains_key(sp)
        {
            report("design.figures.span", format!("no character style \"{sp}\""));
        }
    }
    if let Some(p) = &doc.print {
        check_word("print.ink", &p.ink, &["black", "color"], &mut report);
        if let Some(Err(e)) = p.filter.as_ref().map(|f| f.check()) {
            report("print.filter", e);
        }
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
                    if let Some(r) = &p.repeat {
                        if let Some(dates) = &r.dates
                            && crate::parse_date(&dates.from).is_none()
                        {
                            report(&format!("{path}.repeat.dates.from"), format!("\"{}\" is not a day as YYYY-MM-DD", dates.from));
                        }
                        if r.count.is_some_and(|c| c == 0 || (!r.vars.is_empty() && c as usize != r.vars.len())) {
                            report(&format!("{path}.repeat.count"), "a page is made at least once, and once per vars when they are given".into());
                        }
                    }
                    check_word(&format!("{path}.start"), &p.start, &STARTS[..3], &mut report);
                    check_word(&format!("{path}.numbering"), &p.numbering, &NUMBERINGS, &mut report);
                    if let Some(f) = &p.fill {
                        check_fill(&format!("{path}.fill"), f, &color_name, &mut report);
                    }
                }
                Item::Spread(s) => {
                    if let Some(m) = &s.master
                        && !d.masters.contains_key(m)
                    {
                        report(&format!("{path}.master"), format!("no master \"{m}\""));
                    }
                    check_word(&format!("{path}.numbering"), &s.numbering, &NUMBERINGS, &mut report);
                    if let Some(f) = &s.fill {
                        check_fill(&format!("{path}.fill"), f, &color_name, &mut report);
                    }
                    if s.src.is_none() && (s.left.src.is_none() || s.right.src.is_none()) {
                        report(path, "a spread needs a picture: src, or a src on each side".into());
                    }
                    let in_unit = |v: f64| (0.0..=1.0).contains(&v);
                    for (field, focus) in [("focus", s.focus), ("left.focus", s.left.focus), ("right.focus", s.right.focus)] {
                        if let Some((fx, fy)) = focus
                            && !(in_unit(fx) && in_unit(fy))
                        {
                            report(&format!("{path}.{field}"), "a focus is two fractions from 0 to 1".into());
                        }
                    }
                    for (field, crop) in [("crop", s.crop), ("left.crop", s.left.crop), ("right.crop", s.right.crop)] {
                        if let Some(Err(e)) = crop.map(|c| c.check()) {
                            report(&format!("{path}.{field}"), e);
                        }
                    }
                    for (field, filter) in [("filter", &s.filter), ("left.filter", &s.left.filter), ("right.filter", &s.right.filter)] {
                        if let Some(Err(e)) = filter.as_ref().map(|f| f.check()) {
                            report(&format!("{path}.{field}"), e);
                        }
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
            if let Block::Box(x) = b {
                check_word(&format!("{path}.stroke_align"), &x.stroke_align, &STROKE_ALIGNS, &mut report);
            }
            if let Block::Figure(f) = b {
                if let Some(Err(e)) = f.crop.map(|c| c.check()) {
                    report(&format!("{path}.crop"), e);
                }
                if let Some(Err(e)) = f.filter.as_ref().map(|c| c.check()) {
                    report(&format!("{path}.filter"), e);
                }
                check_word(&format!("{path}.plate"), &f.plate, &["half", "full", "bleed"], &mut report);
                check_word(&format!("{path}.float"), &f.float, &["here", "top", "bottom"], &mut report);
            }
            if let Block::Lof(l) = b
                && let Some(st) = &l.style
                && !has_style(st)
            {
                report(&format!("{path}.style"), format!("no style \"{st}\""));
            }
            if let Block::PageBreak(x) = b {
                check_word(&format!("{path}.to"), &x.to, &["recto", "verso", "odd", "even"], &mut report);
            }
            if let Block::List(l) = b {
                if let Some(sp) = &l.marker_style
                    && !d.spans.contains_key(sp)
                {
                    report(&format!("{path}.marker_style"), format!("no span \"{sp}\""));
                }
                check_word(&format!("{path}.marker_align"), &l.marker_align, &["left", "right"], &mut report);
                if let Some(m) = &l.marker_shape {
                    check_word(&format!("{path}.marker_shape.shape"), &m.shape, &["ellipse", "rect"], &mut report);
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
        Block::Lof(x) => push(&mut v, "style", &x.style),
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
    if let Some(p) = e.place() {
        check_word(&format!("{path}.stroke_align"), &p.stroke_align, &STROKE_ALIGNS, report);
        if let Some(crate::element::Origin::Word(w)) = &p.origin
            && !ORIGINS.contains(&w.as_str())
        {
            report(&format!("{path}.origin"), format!("\"{w}\" is not a point of the box (use {} or [x, y])", ORIGINS.join(", ")));
        }
        if let Some(o) = p.opacity
            && !(0.0..=1.0).contains(&o)
        {
            report(&format!("{path}.opacity"), "opacity runs from 0 to 1".into());
        }
    }
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
            if let Some(s) = &t.scrim {
                if let Some(c) = &s.color {
                    check_color(&format!("{path}.scrim.color"), c, color_name, report);
                }
                if s.opacity.is_some_and(|o| !(0.0..=1.0).contains(&o)) {
                    report(&format!("{path}.scrim.opacity"), "opacity runs from 0 to 1".into());
                }
                check_word(&format!("{path}.scrim.shape"), &s.shape, &["rect", "ellipse"], report);
            }
            if t.safe.is_some() && t.place.w.is_none() {
                report(&format!("{path}.safe"), "a frame kept in the safe zone needs a width (w)".into());
            }
        }
        Element::Image(i) => {
            if i.place.w.is_none() && i.place.h.is_none() {
                report(path, "an image needs a width (w), a height (h) or both".into());
            }
            if i.src.is_empty() {
                report(&format!("{path}.src"), "an image needs a src".into());
            }
            if let Some(Err(e)) = i.crop.map(|c| c.check()) {
                report(&format!("{path}.crop"), e);
            }
            if let Some(Err(e)) = i.filter.as_ref().map(|c| c.check()) {
                report(&format!("{path}.filter"), e);
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
            if let Some(t) = &s.tint {
                check_color(&format!("{path}.tint.color"), &t.color, color_name, report);
                if !(0.0..=1.0).contains(&t.amount) {
                    report(&format!("{path}.tint.amount"), "a tint's amount runs from 0 to 1".into());
                }
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
        Element::Maze(m) => {
            if m.rows.is_empty() == m.segments.is_empty() {
                report(path, "a maze needs rows (a grid) or segments, not both".into());
            }
            if !m.segments.is_empty() && m.size.is_none() {
                report(&format!("{path}.size"), "a maze of segments needs its size in cells".into());
            }
            if m.cell.is_none() && m.place.w.is_none() {
                report(path, "a maze needs a cell size or a width".into());
            }
            let cols = m.rows.first().map(|r| r.chars().count()).unwrap_or(0);
            for (i, r) in m.rows.iter().enumerate() {
                if r.chars().count() != cols {
                    report(&format!("{path}.rows[{i}]"), format!("every row has {cols} cells"));
                }
                if let Some(c) = r.chars().find(|c| !c.is_ascii_hexdigit()) {
                    report(&format!("{path}.rows[{i}]"), format!("\"{c}\" is not a hexadecimal digit (walls: 1 top, 2 right, 4 bottom, 8 left)"));
                }
            }
            if !m.rows.is_empty() {
                let inside = |p: &[f64; 2]| p[0] >= 0.0 && p[1] >= 0.0 && (p[0] as usize) < m.rows.len() && (p[1] as usize) < cols;
                for (field, cell) in [("start", m.start.as_ref()), ("end", m.end.as_ref())] {
                    if cell.is_some_and(|c| !inside(c)) {
                        report(&format!("{path}.{field}"), "a cell [row, column] of the maze".into());
                    }
                }
                if m.solution.iter().any(|c| !inside(c)) {
                    report(&format!("{path}.solution"), "the way through is cells [row, column] of the maze".into());
                }
            }
        }
        Element::LogicGrid(g) => {
            let n = g.categories.first().map(|c| c.items.len()).unwrap_or(0);
            if g.categories.len() < 2 || n < 2 || g.categories.iter().any(|c| c.items.len() != n) {
                report(&format!("{path}.categories"), "a logic grid needs two or more categories with the same number of items (two or more)".into());
            }
            for (i, m) in g.marks.iter().enumerate() {
                let ok = |a: &[usize; 2]| a[0] < g.categories.len() && a[1] < n;
                if !ok(&m.a) || !ok(&m.b) || m.a[0] == m.b[0] {
                    report(&format!("{path}.marks[{i}]"), "a mark joins items of two different categories: [category, item]".into());
                }
                if m.mark.trim().is_empty() || m.mark.chars().count() > 3 {
                    report(&format!("{path}.marks[{i}].mark"), "a mark is \"o\" (a dot), \"x\" (a cross) or up to three letters".into());
                }
            }
        }
        Element::Dots(d) => {
            if d.points.len() < 2 {
                report(&format!("{path}.points"), "a dot-to-dot needs two or more points".into());
            }
            if d.breaks.iter().any(|b| *b >= d.points.len()) {
                report(&format!("{path}.breaks"), "breaks are indices of points".into());
            }
            if !d.labels.is_empty() && d.labels.len() != d.points.len() {
                report(&format!("{path}.labels"), "give a label offset for every point, or none".into());
            }
            need(&d.place, true, true, report);
        }
        Element::Ruling(r) => {
            need(&r.place, true, true, report);
            check_word(&format!("{path}.pattern"), &r.pattern, &["lines", "dots", "grid", "graph", "isodots"], report);
        }
        Element::Calendar(c) => {
            need(&c.place, true, true, report);
            if !(1..=12).contains(&c.month) {
                report(&format!("{path}.month"), "a month is 1 to 12".into());
            }
            check_word(&format!("{path}.week_start"), &c.week_start, &["monday", "sunday"], report);
            if !c.days.is_empty() && c.days.len() != 7 {
                report(&format!("{path}.days"), "seven weekday names, from the week's first day".into());
            }
        }
        Element::Craft(c) => {
            need(&c.place, true, true, report);
            for (i, e) in c.edges.iter().enumerate() {
                check_word(&format!("{path}.edges[{i}].kind"), &e.kind, &["cut", "mountain", "valley", "score"], report);
                if let Some(t) = &e.tab {
                    if t.depth <= 0.0 {
                        report(&format!("{path}.edges[{i}].tab.depth"), "a tab stands off its edge: a depth above 0".into());
                    }
                    check_word(&format!("{path}.edges[{i}].tab.side"), &t.side, &["left", "right"], report);
                }
            }
            for (i, f) in c.faces.iter().enumerate() {
                if f.points.len() < 3 {
                    report(&format!("{path}.faces[{i}].points"), "a face is a polygon of three or more points".into());
                }
                if let Some(Err(e)) = f.crop.map(|c| c.check()) {
                    report(&format!("{path}.faces[{i}].crop"), e);
                }
            }
        }
        Element::Panels(p) => {
            need(&p.place, true, true, report);
            if p.rows.is_empty() || p.rows.iter().any(|r| r.panels.is_empty()) {
                report(&format!("{path}.rows"), "panels come in rows, each with one or more panels".into());
            }
            for (i, r) in p.rows.iter().enumerate() {
                for (j, pn) in r.panels.iter().enumerate() {
                    if let Some(Err(e)) = pn.crop.map(|c| c.check()) {
                        report(&format!("{path}.rows[{i}].panels[{j}].crop"), e);
                    }
                }
            }
        }
        Element::Balloon(b) => {
            if b.text.is_none() == b.content.is_empty() {
                report(path, "a balloon has text or content".into());
            }
            check_word(&format!("{path}.shape"), &b.shape, &["speech", "thought", "shout", "whisper", "caption"], report);
            if let Some(st) = &b.style
                && !d.styles.contains_key(st)
            {
                report(&format!("{path}.style"), format!("no style \"{st}\""));
            }
        }
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
