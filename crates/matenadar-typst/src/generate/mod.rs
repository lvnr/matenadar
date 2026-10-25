//! Generates the Typst program for a document, with a source map back to
//! the document's nodes.

mod blocks;
mod escape;
mod elements;
mod inline;
pub mod typ;

use std::collections::HashMap;
use std::ops::Range;

use indexmap::IndexMap;
use matenadar_model::style::{Align, Case, LineHeight, Master, Style, TextStyle};
use matenadar_model::units::num;
use matenadar_model::{Document, Item, Length};

use crate::glyphs::{VMetrics, pick, weight_number};
use crate::world::{Fonts, Virtual};
use typ::{Palette, str_lit};

pub const PRELUDE: &str = include_str!("../prelude.typ");

/// Typst source generated from a document, with its source map.
pub struct Generated {
    pub source: String,
    pub map: SourceMap,
    /// Things the compiler noticed (missing images, unknown fonts…).
    pub problems: Vec<String>,
}

/// Maps byte offsets in the generated source back to nodes, and to offsets
/// in a node's plain text.
#[derive(Default, Debug)]
pub struct SourceMap {
    segments: Vec<Segment>,
    nodes: Vec<NodeRange>,
}

#[derive(Debug)]
struct Segment {
    src: Range<usize>,
    node: String,
    text_start: usize,
    /// Offsets, relative to `src.start`, of backslashes added by escaping.
    escapes: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct NodeRange {
    pub id: String,
    pub kind: &'static str,
    pub src: Range<usize>,
}

impl SourceMap {
    /// The node and plain-text offset at `src_offset`, if it falls in text.
    pub fn locate(&self, src_offset: usize) -> Option<(&str, usize)> {
        let i = self.segments.partition_point(|s| s.src.start <= src_offset).checked_sub(1)?;
        let s = &self.segments[i];
        if src_offset > s.src.end {
            return None;
        }
        let rel = src_offset - s.src.start;
        let added = s.escapes.partition_point(|&e| e < rel);
        Some((&s.node, s.text_start + rel - added))
    }

    /// The innermost node whose source contains `src_offset`.
    pub fn node_at(&self, src_offset: usize) -> Option<&NodeRange> {
        self.nodes
            .iter()
            .filter(|n| n.src.start <= src_offset && src_offset < n.src.end)
            .min_by_key(|n| n.src.end - n.src.start)
    }

    pub fn nodes(&self) -> &[NodeRange] {
        &self.nodes
    }

    pub fn len(&self) -> usize {
        self.segments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

/// Generates the Typst program for `doc`, whose files are under `root`.
pub fn generate(doc: &Document, fonts: &Fonts, virt: &Virtual, root: &std::path::Path) -> Generated {
    virt.clear();
    let mut g = Gen::new(doc, fonts, virt, root.to_path_buf());
    g.document();
    g.segments_sorted();
    Generated { source: g.out, map: g.map, problems: g.problems }
}

pub(crate) struct Gen<'a> {
    pub doc: &'a Document,
    pub fonts: &'a Fonts,
    pub virt: &'a Virtual,
    pub root: std::path::PathBuf,
    pub pal: Palette<'a>,
    pub out: String,
    pub map: SourceMap,
    pub problems: Vec<String>,
    /// Named styles that have been defined: name → Typst variable.
    styles: IndexMap<String, String>,
    spans: IndexMap<String, String>,
    metrics: HashMap<(String, u16, bool), VMetrics>,
    pub anchors: usize,
    pub files: usize,
    pub depth: usize,
    /// The note column of the master the current flow runs on.
    pub notes: Option<matenadar_model::style::NoteColumn>,
    /// Whether emitted text should get labels and source-map entries.
    pub labels: bool,
    /// The marker of the block a following margin note sits level with.
    pub margin_anchor: Option<String>,
}

impl<'a> Gen<'a> {
    fn new(doc: &'a Document, fonts: &'a Fonts, virt: &'a Virtual, root: std::path::PathBuf) -> Self {
        Self {
            doc,
            fonts,
            virt,
            root,
            pal: Palette { names: &doc.design.colors },
            out: String::with_capacity(64 * 1024),
            map: SourceMap::default(),
            problems: Vec::new(),
            styles: IndexMap::new(),
            spans: IndexMap::new(),
            metrics: HashMap::new(),
            anchors: 0,
            files: 0,
            depth: 0,
            notes: None,
            labels: true,
            margin_anchor: None,
        }
    }

    pub fn raw(&mut self, s: &str) {
        self.out.push_str(s);
    }

    pub fn problem(&mut self, msg: String) {
        if !self.problems.contains(&msg) {
            self.problems.push(msg);
        }
    }

    fn segments_sorted(&mut self) {
        self.map.segments.sort_by_key(|s| s.src.start);
    }

    /// Records that the source from `start` to here is the node `id`.
    pub fn node(&mut self, id: Option<&str>, kind: &'static str, start: usize) {
        if let Some(id) = id
            && self.labels
        {
            self.map.nodes.push(NodeRange { id: id.to_string(), kind, src: start..self.out.len() });
        }
    }

    /// A marker labelled with the node's id, for positions and references.
    pub fn label(&mut self, id: Option<&str>) {
        if let Some(id) = id
            && self.labels
            && label_safe(id)
        {
            self.raw(&format!("#metadata({}) <{id}>", str_lit(id)));
        }
    }

    pub fn text_segment(&mut self, node: &str, text_start: usize, escaped: &str, escapes: Vec<usize>) {
        let start = self.out.len();
        self.out.push_str(escaped);
        if self.labels {
            self.map.segments.push(Segment { src: start..self.out.len(), node: node.to_string(), text_start, escapes });
        }
    }

    // ---- The document ----

    fn document(&mut self) {
        let doc = self.doc;
        self.raw(PRELUDE);
        let author = doc.meta.author.clone().unwrap_or_default();
        self.raw(&format!("\n#set document(title: {}, author: {})\n", str_lit(&doc.meta.title), str_lit(&author)));
        // Blank pages inserted to start a section on the right side take
        // the first master's size.
        if let Some(m) = self.first_master() {
            let (w, h) = (m.width.unwrap_or(Length::Pt(432.0)), m.height.unwrap_or(Length::Pt(648.0)));
            let bleed = self.bleed(&m);
            self.raw(&format!("#set page(width: {}, height: {}, bleed: {bleed}, margin: 0pt)\n", w.typst(), h.typst()));
        }
        let base = self.style_dict(&doc.design.style(""), false);
        self.raw(&format!("#let mt-base = {base}\n#set text(..mt-base.text)\n"));
        // Every named style and character style, defined once.
        let names: Vec<String> = doc.design.styles.keys().cloned().collect();
        for (i, name) in names.iter().enumerate() {
            let dict = self.style_dict(&doc.design.style(name), false);
            self.raw(&format!("#let mt-st-{i} = {dict}\n"));
            self.styles.insert(name.clone(), format!("mt-st-{i}"));
        }
        let spans: Vec<String> = doc.design.spans.keys().cloned().collect();
        for (i, name) in spans.iter().enumerate() {
            let span = doc.design.spans[name].clone();
            let dict = self.text_dict(&span, None);
            self.raw(&format!("#let mt-sp-{i} = {dict}\n"));
            self.spans.insert(name.clone(), format!("mt-sp-{i}"));
        }

        let masters: Vec<String> = doc.design.masters.keys().cloned().collect();
        for (i, name) in masters.iter().enumerate() {
            let m = doc.design.master(name).unwrap_or_default();
            let dict = self.master_dict(&m);
            self.raw(&format!("#let mt-m-{i} = {dict}\n"));
        }

        let mut numbering = "1".to_string();
        let mut open_master: Option<String> = None;
        for (idx, item) in doc.content.iter().enumerate() {
            match item {
                Item::Section(s) => {
                    if let Some(n) = &s.numbering {
                        numbering = n.clone();
                    }
                    let cont = s.start.as_deref() == Some("continue") && open_master.as_deref() == Some(s.master.as_str());
                    if !cont {
                        if open_master.is_some() {
                            self.raw("]\n");
                        }
                        self.start_break(idx, s.start.as_deref());
                        let mi = masters.iter().position(|m| m == &s.master).unwrap_or(0);
                        let m = doc.design.master(&s.master).unwrap_or_default();
                        let gutter = m.gutter.map(|g| g.typst()).unwrap_or_else(|| "12pt".into());
                        self.raw(&format!("#page(..mt-m-{mi})[#set columns(gutter: {gutter})\n"));
                        self.notes = m.notes.clone();
                        open_master = Some(s.master.clone());
                    }
                    let start = self.out.len();
                    self.raw(&format!(
                        "#mt-sec((id: {}, kind: \"section\", title: {}, vars: {}, numbering: {}))",
                        str_lit(&s.id),
                        str_lit(s.title.as_deref().unwrap_or("")),
                        vars(&s.vars),
                        str_lit(&numbering)
                    ));
                    self.label(Some(&s.id));
                    if let Some(n) = s.page_from {
                        self.raw(&format!("#counter(page).update({n})"));
                    }
                    self.raw("\n");
                    let style = s
                        .style
                        .clone()
                        .or_else(|| doc.design.master(&s.master).and_then(|m| m.style))
                        .or_else(|| doc.design.styles.contains_key("body").then(|| "body".to_string()));
                    let st = match style.as_deref() {
                        Some(name) => self.style_var(name),
                        None => "mt-base".to_string(),
                    };
                    self.flow(&s.flow, &st, false);
                    self.raw(&format!("#mt-end({})\n", str_lit(&s.id)));
                    self.node(Some(&s.id), "section", start);
                }
                Item::Page(p) => {
                    if let Some(n) = &p.numbering {
                        numbering = n.clone();
                    }
                    if open_master.take().is_some() {
                        self.raw("]\n");
                    }
                    self.start_break(idx, p.start.as_deref());
                    let master_name = p.master.clone().or_else(|| masters.first().cloned()).unwrap_or_default();
                    let mi = masters.iter().position(|m| m == &master_name).unwrap_or(0);
                    let mut over = vec!["margin: 0pt".to_string(), "columns: 1".to_string()];
                    if !p.layers.unwrap_or(false) {
                        over.push("background: none".into());
                        over.push("foreground: none".into());
                    }
                    if let Some(f) = &p.fill {
                        over.push(format!("fill: {}", self.pal.fill(f)));
                    }
                    let start = self.out.len();
                    self.raw(&format!("#page(..(mt-m-{mi} + ({})))[", over.join(", ")));
                    self.raw(&format!(
                        "#mt-sec((id: {}, kind: \"page\", title: {}, vars: {}, numbering: {}))",
                        str_lit(&p.id),
                        str_lit(p.title.as_deref().unwrap_or("")),
                        vars(&p.vars),
                        str_lit(&numbering)
                    ));
                    self.label(Some(&p.id));
                    if let Some(n) = p.page_from {
                        self.raw(&format!("#counter(page).update({n})"));
                    }
                    self.raw("\n");
                    for e in &p.elements {
                        self.element(e, false);
                    }
                    self.raw(&format!("#mt-end({})]\n", str_lit(&p.id)));
                    self.node(Some(&p.id), "page", start);
                }
            }
        }
        if open_master.is_some() {
            self.raw("]\n");
        }
    }

    fn start_break(&mut self, idx: usize, start: Option<&str>) {
        if idx == 0 {
            return;
        }
        match start {
            Some("recto") => self.raw("#pagebreak(weak: true, to: \"odd\")\n"),
            Some("verso") => self.raw("#pagebreak(weak: true, to: \"even\")\n"),
            _ => {}
        }
    }

    fn first_master(&self) -> Option<Master> {
        let name = self.doc.design.masters.keys().next()?;
        self.doc.design.master(name)
    }

    fn bleed(&self, m: &Master) -> String {
        match m.bleed {
            Some(b) if b.pt().unwrap_or(0.0) > 0.0 => {
                format!("(top: {b}, bottom: {b}, outside: {b}, inside: 0pt)", b = b.typst())
            }
            _ => "0pt".into(),
        }
    }

    fn master_dict(&mut self, m: &Master) -> String {
        let mut parts = Vec::new();
        parts.push(format!("width: {}", m.width.unwrap_or(Length::Pt(432.0)).typst()));
        parts.push(format!("height: {}", m.height.unwrap_or(Length::Pt(648.0)).typst()));
        parts.push(format!("bleed: {}", self.bleed(m)));
        let mg = m.margins.clone().unwrap_or_default();
        let z = Length::Pt(36.0);
        parts.push(format!(
            "margin: (top: {}, bottom: {}, inside: {}, outside: {})",
            mg.top.unwrap_or(z).typst(),
            mg.bottom.unwrap_or(z).typst(),
            mg.inside.unwrap_or(z).typst(),
            mg.outside.unwrap_or(z).typst()
        ));
        parts.push("binding: left".into());
        parts.push(format!("columns: {}", m.columns.unwrap_or(1)));
        if let Some(f) = &m.fill {
            parts.push(format!("fill: {}", self.pal.fill(f)));
        }
        parts.push("header: none".into());
        parts.push("footer: none".into());
        for (key, list) in [("background", &m.background), ("foreground", &m.foreground)] {
            if list.is_empty() {
                parts.push(format!("{key}: none"));
                continue;
            }
            let saved = std::mem::take(&mut self.out);
            let labels = std::mem::replace(&mut self.labels, false);
            self.raw("[");
            for e in list {
                self.element(e, false);
            }
            self.raw("]");
            self.labels = labels;
            let body = std::mem::replace(&mut self.out, saved);
            parts.push(format!("{key}: {body}"));
        }
        format!("({})", parts.join(", "))
    }

    // ---- Styles ----

    /// Named styles and their Typst variables.
    pub fn style_names(&self) -> impl Iterator<Item = (&str, &str)> {
        self.styles.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// The Typst variable holding the named style (the base for an unknown name).
    pub fn style_var(&mut self, name: &str) -> String {
        self.styles.get(name).cloned().unwrap_or_else(|| "mt-base".to_string())
    }

    /// A style with local overrides, as a dictionary literal.
    pub fn style_with(&mut self, name: &str, set: Option<&Style>) -> String {
        match set {
            None => self.style_var(name),
            Some(over) => {
                let resolved = self.doc.design.style(name).with(over);
                self.style_dict(&resolved, false)
            }
        }
    }

    pub fn span_var(&mut self, name: &str) -> String {
        self.spans.get(name).cloned().unwrap_or_else(|| "(text: (:), wrap: none)".to_string())
    }

    /// Font families of a stack, with the design's named stacks expanded.
    pub fn families(&self, t: &TextStyle) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(stack) = &t.font {
            for f in stack.families() {
                match self.doc.design.fonts.get(f) {
                    Some(named) => out.extend(named.families().into_iter().map(str::to_string)),
                    None => out.push(f.to_string()),
                }
            }
        }
        out
    }

    pub fn metrics(&mut self, t: &TextStyle) -> VMetrics {
        let fams = self.families(t);
        let weight = t.weight.as_ref().map(weight_number).unwrap_or(400);
        let italic = t.italic.unwrap_or(false);
        let key = (fams.join(","), weight, italic);
        if let Some(m) = self.metrics.get(&key) {
            return *m;
        }
        let m = match pick(self.fonts, &fams, weight, italic) {
            Some(f) => VMetrics::of(&f),
            None => {
                if !fams.is_empty() {
                    self.problem(format!("no font found for {}", fams.join(", ")));
                }
                pick(self.fonts, &["Libertinus Serif".to_string()], 400, false).map(|f| VMetrics::of(&f)).unwrap_or(VMetrics::DEFAULT)
            }
        };
        self.metrics.insert(key, m);
        m
    }

    /// `text` settings and wrappers (case, underline) of a character style.
    /// With a line height (in em), adds the top and bottom edges that make
    /// lines sit that far apart, CSS-style.
    pub fn text_dict(&mut self, t: &TextStyle, line_height_em: Option<f64>) -> String {
        let mut tx = Vec::new();
        let fams = self.families(t);
        if !fams.is_empty() {
            tx.push(format!("font: ({},)", fams.iter().map(|f| str_lit(f)).collect::<Vec<_>>().join(", ")));
        }
        if let Some(s) = &t.size {
            tx.push(format!("size: {}", s.typst()));
        }
        if let Some(w) = &t.weight {
            match w {
                serde_json::Value::Number(n) => tx.push(format!("weight: {}", n.as_f64().unwrap_or(400.0) as i64)),
                serde_json::Value::String(s) => tx.push(format!("weight: {}", str_lit(s))),
                _ => {}
            }
        }
        if let Some(i) = t.italic {
            tx.push(format!("style: {}", if i { "\"italic\"" } else { "\"normal\"" }));
        }
        if let Some(s) = t.stretch {
            tx.push(format!("stretch: {}%", num(s)));
        }
        if let Some(tr) = &t.tracking {
            tx.push(format!("tracking: {}", tr.typst()));
        }
        if let Some(ws) = &t.word_spacing {
            tx.push(format!("spacing: 100% + {}", ws.typst()));
        }
        if let Some(c) = &t.color {
            tx.push(format!("fill: {}", self.pal.color(c)));
        }
        if let Some(f) = &t.features {
            let feats: Vec<String> = f
                .iter()
                .map(|f| {
                    let (tag, val) = match f.strip_prefix('-') {
                        Some(t) => (t.to_string(), "0".to_string()),
                        None => match f.split_once('=') {
                            Some((t, v)) => (t.to_string(), v.to_string()),
                            None => (f.clone(), "1".to_string()),
                        },
                    };
                    format!("{}: {val}", str_lit(&tag))
                })
                .collect();
            if !feats.is_empty() {
                tx.push(format!("features: ({})", feats.join(", ")));
            }
        }
        if let Some(b) = &t.baseline {
            tx.push(format!("baseline: -{}", b.typst()));
        }
        if let Some(lang) = &t.lang {
            push_lang(&mut tx, lang);
        }
        if let Some(h) = t.hyphenate {
            tx.push(format!("hyphenate: {h}"));
        }
        if let Some(l) = t.ligatures {
            tx.push(format!("ligatures: {l}"));
        }
        if let Some(s) = &t.outline {
            tx.push(format!("stroke: {}", self.pal.stroke(s, false)));
        }
        if let Some(lh) = line_height_em {
            let m = self.metrics(t);
            let half = (lh - (m.ascender + m.descender)) / 2.0;
            tx.push(format!("top-edge: {}em", num(m.ascender + half)));
            tx.push(format!("bottom-edge: -{}em", num(m.descender + half)));
        }
        let mut wrap = Vec::new();
        match t.case {
            Some(Case::Upper) => wrap.push("upper(b)"),
            Some(Case::Lower) => wrap.push("lower(b)"),
            Some(Case::SmallCaps) => wrap.push("smallcaps(b)"),
            Some(Case::AllSmallCaps) => wrap.push("smallcaps(all: true, b)"),
            _ => {}
        }
        if t.underline == Some(true) {
            wrap.push("underline(b)");
        }
        if t.strike == Some(true) {
            wrap.push("strike(b)");
        }
        let wrap = if wrap.is_empty() {
            "none".to_string()
        } else {
            let mut expr = "b".to_string();
            for w in wrap {
                expr = w.replace("(b)", &format!("({expr})")).replace(", b)", &format!(", {expr})"));
            }
            format!("b => {expr}")
        };
        format!("(text: ({}), wrap: {wrap})", if tx.is_empty() { ":".into() } else { tx.join(", ") })
    }

    /// A paragraph style as the dictionary `mt-par` reads.
    pub fn style_dict(&mut self, s: &Style, _scaled: bool) -> String {
        let mut t = s.text.clone();
        if t.lang.is_none()
            && let Some(lang) = &self.doc.meta.lang
        {
            t.lang = Some(lang.clone());
        }
        let size = t.size.map_or(11.0, |l| l.resolve(11.0, 0.0));
        let lh_em = match s.line_height {
            Some(LineHeight::Factor(f)) => Some(f),
            Some(LineHeight::Length(l)) => Some(l.resolve(size, size * 100.0) / size),
            None => {
                let m = self.metrics(&t);
                Some(m.ascender + m.descender)
            }
        };
        let text = self.text_dict(&t, lh_em);
        let em = |l: &Option<Length>| l.map(|l| matenadar_model::units::num(l.resolve(size, 0.0)) + "pt");
        let inner = text.strip_prefix('(').and_then(|t| t.strip_suffix(')')).unwrap_or(&text).to_string();
        let mut parts = vec![inner];
        if let Some(v) = em(&s.space_before) {
            parts.push(format!("above: {v}"));
        }
        if let Some(v) = em(&s.space_after) {
            parts.push(format!("below: {v}"));
        }
        if let Some(i) = &s.indent {
            parts.push(format!("indent: {}", i.typst()));
        }
        if s.indent_first == Some(true) {
            parts.push("indent-first: true".into());
        }
        if let Some(h) = &s.hanging {
            parts.push(format!("hanging: {}", h.typst()));
        }
        if let Some(v) = em(&s.margin_left) {
            parts.push(format!("left: {v}"));
        }
        if let Some(v) = em(&s.margin_right) {
            parts.push(format!("right: {v}"));
        }
        let align = match s.align {
            Some(Align::Left) | None => "left",
            Some(Align::Right) => "right",
            Some(Align::Center) => "center",
            Some(Align::Justify) => "justify",
            Some(Align::Outside) => "outside",
            Some(Align::Inside) => "inside",
        };
        parts.push(format!("align: \"{align}\""));
        if s.keep_with_next == Some(true) {
            parts.push("sticky: true".into());
        }
        if s.keep_together == Some(true) {
            parts.push("breakable: false".into());
        }
        if s.allow_widows == Some(true) || s.allow_orphans == Some(true) {
            parts.push(format!(
                "costs: (widow: {}, orphan: {})",
                if s.allow_widows == Some(true) { "0%" } else { "100%" },
                if s.allow_orphans == Some(true) { "0%" } else { "100%" }
            ));
        }
        if let Some(f) = &s.fill {
            parts.push(format!("fill: {}", self.pal.fill(f)));
        }
        if let Some(i) = &s.inset {
            parts.push(format!("inset: {}", typ::insets(i, false)));
        }
        for (key, rule) in [("rule-above", &s.rule_above), ("rule-below", &s.rule_below)] {
            if let Some(r) = rule {
                let stroke = r.stroke.as_ref().map(|st| self.pal.stroke(st, false)).unwrap_or_else(|| "0.5pt".into());
                let width = r.width.map(|w| w.typst()).unwrap_or_else(|| "100%".into());
                let gap = r.gap.map(|g| g.typst()).unwrap_or_else(|| "4pt".into());
                parts.push(format!("{key}: (stroke: {stroke}, width: {width}, gap: {gap})"));
            }
        }
        if s.balance == Some(true) {
            parts.push("balance: true".into());
        }
        if let Some(dc) = &s.drop_cap {
            let mut d = vec![format!("lines: {}", dc.lines)];
            let mut dt = TextStyle::default();
            if dc.font.is_some() {
                dt.font = dc.font.clone();
            }
            dt.color = dc.color.clone();
            dt.weight = dc.weight.clone();
            let fams = self.families(&dt);
            if !fams.is_empty() {
                d.push(format!("font: ({},)", fams.iter().map(|f| str_lit(f)).collect::<Vec<_>>().join(", ")));
            }
            if let Some(c) = &dc.color {
                d.push(format!("fill: {}", self.pal.color(c)));
            }
            if let Some(w) = &dc.weight {
                d.push(format!("weight: {}", weight_number(w)));
            }
            d.push(format!("gap: {}", dc.gap.map(|g| g.typst()).unwrap_or_else(|| "0.15em".into())));
            d.push(format!("lh: {}em", num(lh_em.unwrap_or(1.2))));
            parts.push(format!("drop: ({})", d.join(", ")));
        }
        format!("({})", parts.join(", "))
    }

}

fn vars(v: &IndexMap<String, String>) -> String {
    if v.is_empty() {
        return "(:)".into();
    }
    format!("({})", v.iter().map(|(k, val)| format!("{}: {}", str_lit(k), str_lit(val))).collect::<Vec<_>>().join(", "))
}

fn push_lang(tx: &mut Vec<String>, lang: &str) {
    match lang.split_once('-') {
        Some((l, r)) => {
            tx.push(format!("lang: {}", str_lit(&l.to_lowercase())));
            if r.len() == 2 {
                tx.push(format!("region: {}", str_lit(&r.to_lowercase())));
            }
        }
        None => tx.push(format!("lang: {}", str_lit(&lang.to_lowercase()))),
    }
}

/// Whether an id can be written as a Typst label (`<id>`).
pub fn label_safe(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && id.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
}
