//! Generates the Typst program for a document, with a source map back to
//! the document's nodes.

mod blocks;
mod escape;
mod elements;
mod inline;
mod kinds;
mod pictures;
pub mod typ;

use std::collections::HashMap;
use std::ops::Range;

use indexmap::IndexMap;
use matenadar_model::style::{Align, Case, LineHeight, Master, Style, TextStyle};
use matenadar_model::units::num;
use matenadar_model::{Document, Item, Length};

use crate::glyphs::{VMetrics, pick, weight_number};
use crate::world::{Files, Fonts, Virtual};
use typ::{Palette, str_lit};

pub const PRELUDE: &str = include_str!("../prelude.typ");

/// Typst source generated from a document, with its source map.
pub struct Generated {
    pub source: String,
    pub map: SourceMap,
    /// Things the compiler noticed (missing images, unknown fonts…).
    pub problems: Vec<String>,
    /// Opacity and outlining, by the number in an `mt-fx-<n>` label.
    pub effects: Vec<crate::effects::Effect>,
}

/// Maps byte offsets in the generated source back to nodes, and to offsets
/// in a node's plain text.
#[derive(Default, Debug)]
pub struct SourceMap {
    segments: Vec<Segment>,
    /// Sorted by start (outer before inner); ranges nest.
    nodes: Vec<NodeRange>,
    /// Each node's enclosing node.
    parents: Vec<Option<usize>>,
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
    /// An element placed on a page or in a canvas, rather than a block.
    pub placed: bool,
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
        self.nodes_at(src_offset).next()
    }

    /// The innermost placed element whose source contains `src_offset`.
    pub fn element_at(&self, src_offset: usize) -> Option<&NodeRange> {
        self.nodes_at(src_offset).find(|n| n.placed)
    }

    /// Every node whose source contains `src_offset`, innermost first.
    pub fn nodes_at(&self, src_offset: usize) -> impl Iterator<Item = &NodeRange> {
        // The last node starting at or before the offset is the innermost
        // containing node, or lies inside it: walk out until one contains it.
        let mut at = self.nodes.partition_point(|n| n.src.start <= src_offset).checked_sub(1);
        while let Some(i) = at
            && src_offset >= self.nodes[i].src.end
        {
            at = self.parents[i];
        }
        std::iter::successors(at, |&i| self.parents[i]).map(|i| &self.nodes[i])
    }

    /// Sorts the nodes and links each to the node around it.
    fn link(&mut self) {
        self.nodes.sort_by(|a, b| a.src.start.cmp(&b.src.start).then(b.src.end.cmp(&a.src.end)));
        self.parents = vec![None; self.nodes.len()];
        let mut open: Vec<usize> = Vec::new();
        for i in 0..self.nodes.len() {
            while let Some(&top) = open.last()
                && self.nodes[top].src.end <= self.nodes[i].src.start
            {
                open.pop();
            }
            self.parents[i] = open.last().copied();
            open.push(i);
        }
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

/// The design's hyphenation exceptions, by lowercase word.
fn exceptions(doc: &Document) -> HashMap<String, Vec<usize>> {
    let mut out = HashMap::new();
    for w in doc.design.hyphenation.iter().flat_map(|h| &h.exceptions) {
        let mut breaks = Vec::new();
        let mut word = String::new();
        for c in w.trim().chars() {
            if c == '-' {
                breaks.push(word.chars().count());
            } else {
                word.push(c);
            }
        }
        if !word.is_empty() {
            out.insert(word.to_lowercase(), breaks);
        }
    }
    out
}

/// Whether any style, span or run of the document sets letter spacing.
fn uses_tracking(doc: &Document) -> bool {
    let d = &doc.design;
    d.base.text.tracking.is_some()
        || d.styles.values().any(|s| s.text.tracking.is_some())
        || d.spans.values().any(|s| s.tracking.is_some())
        || serde_json::to_string(&doc.content).is_ok_and(|c| c.contains("\"tracking\""))
}

/// A font size as Typst takes it: a percentage is of the surrounding size.
pub fn font_size(l: Length) -> Length {
    match l {
        Length::Pct(p) => Length::Em(p / 100.0),
        other => other,
    }
}

/// Generates the Typst program for `doc`, whose pictures and drawings are in `files`.
pub fn generate(doc: &Document, fonts: &Fonts, virt: &Virtual, files: &Files) -> Generated {
    virt.clear();
    let mut g = Gen::new(doc, fonts, virt, files);
    g.document();
    g.segments_sorted();
    Generated { source: g.out, map: g.map, problems: g.problems, effects: g.effects }
}

pub(crate) struct Gen<'a> {
    pub doc: &'a Document,
    pub fonts: &'a Fonts,
    pub virt: &'a Virtual,
    pub assets: &'a Files,
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
    /// Paragraphs are as wide as their text (inside frames fitted to a width).
    pub hug: bool,
    /// Content written to a scratch buffer, waiting to be spliced into the
    /// source: its text, and the source-map entries made while writing it.
    pub splices: Vec<(String, MapMark, MapMark)>,
    /// The document letter-spaces some text, so runs end with a word joiner
    /// (which keeps the run's last letter tracked; see `inlines`).
    pub tracked: bool,
    /// In the paragraph being written, the plain-text offset of a space that
    /// must not break (the last two words of a `wrap: pretty` paragraph).
    pub nbsp_at: Option<usize>,
    pub effects: Vec<crate::effects::Effect>,
    /// Hyphenation exceptions: a lowercase word and the character counts
    /// before its allowed breaks.
    pub hyphenation: HashMap<String, Vec<usize>>,
    /// The paragraph being written hyphenates (so exceptions apply).
    pub hyphenating: bool,
    /// The resolved style behind each style expression written, for settings
    /// the generator acts on itself (`wrap`).
    pub resolved: HashMap<String, Style>,
    /// The opacity of the groups around what is being written, for drawings
    /// (SVG), which fade themselves.
    pub opacity: f64,
    /// Paragraphs the generator makes whose words are another node's (a text
    /// frame's): paragraph id → the node, where in its words they start, and
    /// whether the paragraph is labelled itself.
    pub owners: HashMap<String, (String, usize, bool)>,
    /// The numbers of the figures the design numbers, by figure.
    pub fig_numbers: HashMap<usize, (String, String)>,
    /// The master of the flow being written (plates escape its margins).
    pub master_now: Option<Master>,
    /// The page an element is placed on directly, for frames kept in its
    /// safe zone; taken by the element, so what is inside it doesn't see it.
    pub page_geom: Option<PageGeom>,
    /// Some figure is a full-bleed plate, whose page hides the master's elements.
    pub bleed_plates: bool,
}

/// A page's trim and safe zone, in points.
#[derive(Clone, Debug)]
pub struct PageGeom {
    pub w: f64,
    pub h: f64,
    /// The master's margins: top, bottom, inside, outside (or left, right).
    pub margins: [f64; 4],
    pub mirrored: bool,
}

impl PageGeom {
    pub fn of(m: &Master) -> Self {
        let pt = |l: Option<Length>, d: f64| l.and_then(|v| v.pt()).unwrap_or(d);
        let mg = m.margins.clone().unwrap_or_default();
        let mirrored = mg.left.is_none() && mg.right.is_none();
        let (a, b) = if mirrored { (mg.inside, mg.outside) } else { (mg.left, mg.right) };
        PageGeom {
            w: pt(m.width, 432.0),
            h: pt(m.height, 648.0),
            margins: [pt(mg.top, 36.0), pt(mg.bottom, 36.0), pt(a, 36.0), pt(b, 36.0)],
            mirrored,
        }
    }
}

/// A position in the source map's lists.
#[derive(Clone, Copy, Debug)]
pub struct MapMark {
    segs: usize,
    nodes: usize,
}

impl<'a> Gen<'a> {
    fn new(doc: &'a Document, fonts: &'a Fonts, virt: &'a Virtual, assets: &'a Files) -> Self {
        Self {
            doc,
            fonts,
            virt,
            assets,
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
            hug: false,
            splices: Vec::new(),
            tracked: uses_tracking(doc),
            nbsp_at: None,
            effects: Vec::new(),
            hyphenation: exceptions(doc),
            hyphenating: false,
            resolved: HashMap::new(),
            opacity: 1.0,
            owners: HashMap::new(),
            fig_numbers: matenadar_model::figures::numbered(doc)
                .into_iter()
                .map(|n| (matenadar_model::figures::key(n.figure), (n.number, n.label)))
                .collect(),
            master_now: None,
            page_geom: None,
            bleed_plates: serde_json::to_string(&doc.content).is_ok_and(|c| c.contains("\"plate\":\"bleed\"")),
        }
    }

    pub fn raw(&mut self, s: &str) {
        self.out.push_str(s);
    }

    /// `body` (a Typst expression) in a labelled box that the effects pass
    /// will find after layout.
    pub fn with_effect(&mut self, body: &str, effect: crate::effects::Effect) -> String {
        self.effects.push(effect);
        format!("[#box({body})<mt-fx-{}>]", self.effects.len() - 1)
    }

    pub fn map_mark(&self) -> MapMark {
        MapMark { segs: self.map.segments.len(), nodes: self.map.nodes.len() }
    }

    /// Writes into a scratch buffer and returns what was written. Source-map
    /// entries made meanwhile are moved to the right place by `emit`.
    pub fn capture(&mut self, f: impl FnOnce(&mut Self)) -> String {
        let saved = std::mem::take(&mut self.out);
        let m0 = self.map_mark();
        f(self);
        let content = std::mem::replace(&mut self.out, saved);
        let m1 = self.map_mark();
        self.splices.push((content.clone(), m0, m1));
        content
    }

    /// Appends `text`, which may contain captured content; the source-map
    /// entries of that content are shifted to where it now stands.
    pub fn emit(&mut self, text: &str) {
        let base = self.out.len();
        for (content, m0, m1) in std::mem::take(&mut self.splices) {
            let Some(at) = text.find(&content) else { continue };
            let by = base + at;
            for seg in &mut self.map.segments[m0.segs..m1.segs] {
                seg.src = seg.src.start + by..seg.src.end + by;
            }
            for node in &mut self.map.nodes[m0.nodes..m1.nodes] {
                node.src = node.src.start + by..node.src.end + by;
            }
        }
        self.out.push_str(text);
    }

    pub fn problem(&mut self, msg: String) {
        if !self.problems.contains(&msg) {
            self.problems.push(msg);
        }
    }

    fn segments_sorted(&mut self) {
        self.map.segments.sort_by_key(|s| s.src.start);
        self.map.link();
    }

    /// Records that the source from `start` to here is the node `id`.
    pub fn node(&mut self, id: Option<&str>, kind: &'static str, start: usize) {
        self.push_node(id, kind, start, false);
    }

    pub fn placed_node(&mut self, id: Option<&str>, kind: &'static str, start: usize) {
        self.push_node(id, kind, start, true);
    }

    fn push_node(&mut self, id: Option<&str>, kind: &'static str, start: usize, placed: bool) {
        if let Some(id) = id
            && self.labels
        {
            self.map.nodes.push(NodeRange { id: id.to_string(), kind, src: start..self.out.len(), placed });
        }
    }

    /// A marker labelled with the node's id, for positions and references,
    /// and what the node's page leaves out (`on_page`).
    pub fn label(&mut self, id: Option<&str>) {
        if let Some(id) = id
            && self.labels
            && label_safe(id)
        {
            self.raw(&format!("#metadata({}) <{id}>", str_lit(id)));
            if let Some(on) = self.doc.on_page.get(id)
                && !on.hide.is_empty()
            {
                let list: Vec<String> = on.hide.iter().map(|h| str_lit(h)).collect();
                self.raw(&format!("#metadata(({},)) <mt-hide>", list.join(", ")));
            }
        }
    }

    /// The names `on_page.hide` knows a master element by (its id and, for
    /// text, its style), when some node's page hides one of them.
    fn hide_names(&self, e: &matenadar_model::Element) -> Option<Vec<String>> {
        let mut names: Vec<String> = e.id().map(str::to_string).into_iter().collect();
        if let matenadar_model::Element::Text(t) = e
            && let Some(st) = &t.style
        {
            names.push(st.clone());
        }
        let hidden = self.bleed_plates || self.doc.on_page.values().any(|on| on.hide.iter().any(|h| h == "all" || names.contains(h)));
        hidden.then_some(names)
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
            let h = if m.grow == Some(true) { "auto".to_string() } else { h.typst() };
            let bleed = self.bleed(&m);
            self.raw(&format!("#set page(width: {}, height: {h}, bleed: {bleed}, margin: 0pt)\n", w.typst()));
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
                    self.master_now = doc.design.master(&s.master);
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
                    // A repeated page is one page in a loop, each time with its own vars.
                    let times = p.repeat.as_ref().map(|r| repeat_vars(&p.id, r, &mut self.problems));
                    if let Some(list) = &times {
                        self.raw(&format!("#for mt-v in ({},) [", list.join(", ")));
                    }
                    self.raw(&format!("#page(..(mt-m-{mi} + ({})))[", over.join(", ")));
                    self.raw(&format!(
                        "#mt-sec((id: {}, kind: \"page\", title: {}, vars: {}{}, numbering: {}))",
                        str_lit(&p.id),
                        str_lit(p.title.as_deref().unwrap_or("")),
                        vars(&p.vars),
                        if times.is_some() { " + mt-v" } else { "" },
                        str_lit(&numbering)
                    ));
                    if times.is_some() {
                        self.raw("#if mt-v.n == \"1\" [");
                    }
                    self.label(Some(&p.id));
                    if let Some(n) = p.page_from {
                        self.raw(&format!("#counter(page).update({n})"));
                    }
                    if times.is_some() {
                        self.raw("]");
                    }
                    self.raw("\n");
                    let geom = doc.design.master(&master_name).map(|m| PageGeom::of(&m));
                    for e in &p.elements {
                        self.page_geom = geom.clone();
                        self.element(e, false);
                    }
                    self.page_geom = None;
                    self.raw(&format!("#mt-end({})]\n", str_lit(&p.id)));
                    if times.is_some() {
                        self.raw("]\n");
                    }
                    self.node(Some(&p.id), "page", start);
                }
                Item::Spread(s) => {
                    if let Some(n) = &s.numbering {
                        numbering = n.clone();
                    }
                    if open_master.take().is_some() {
                        self.raw("]\n");
                    }
                    // A spread opens on a verso, so its pages face each other.
                    self.raw(if idx == 0 { "#pagebreak(to: \"even\")\n" } else { "#pagebreak(weak: true, to: \"even\")\n" });
                    let master_name = s.master.clone().or_else(|| masters.first().cloned()).unwrap_or_default();
                    let mi = masters.iter().position(|m| m == &master_name).unwrap_or(0);
                    let m = doc.design.master(&master_name).unwrap_or_default();
                    let mut over = vec!["margin: 0pt".to_string(), "columns: 1".to_string()];
                    if !s.layers.unwrap_or(false) {
                        over.push("background: none".into());
                        over.push("foreground: none".into());
                    }
                    if let Some(f) = &s.fill {
                        over.push(format!("fill: {}", self.pal.fill(f)));
                    }
                    let page = format!("#page(..(mt-m-{mi} + ({})))[", over.join(", "));
                    let geom = PageGeom::of(&m);
                    let start = self.out.len();
                    for recto in [false, true] {
                        self.raw(&page);
                        if !recto {
                            self.raw(&format!(
                                "#mt-sec((id: {}, kind: \"spread\", title: {}, vars: {}, numbering: {}))",
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
                        }
                        self.spread_part(s, &m, recto);
                        for e in if recto { &s.right.elements } else { &s.left.elements } {
                            self.page_geom = Some(geom.clone());
                            self.element(e, false);
                        }
                        self.page_geom = None;
                        if recto {
                            self.raw(&format!("#mt-end({})", str_lit(&s.id)));
                        }
                        self.raw("]\n");
                    }
                    self.node(Some(&s.id), "spread", start);
                }
            }
        }
        if open_master.is_some() {
            self.raw("]\n");
        }
    }

    fn start_break(&mut self, idx: usize, start: Option<&str>) {
        if idx == 0 {
            // The first page is a recto; to open on a verso, leave it blank.
            if start == Some("verso") {
                self.raw("#pagebreak(to: \"even\")\n");
            }
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
        parts.push(if m.grow == Some(true) { "height: auto".into() } else { format!("height: {}", m.height.unwrap_or(Length::Pt(648.0)).typst()) });
        parts.push(format!("bleed: {}", self.bleed(m)));
        let mg = m.margins.clone().unwrap_or_default();
        let z = Length::Pt(36.0);
        if mg.left.is_some() || mg.right.is_some() {
            parts.push(format!(
                "margin: (top: {}, bottom: {}, left: {}, right: {})",
                mg.top.unwrap_or(z).typst(),
                mg.bottom.unwrap_or(z).typst(),
                mg.left.unwrap_or(z).typst(),
                mg.right.unwrap_or(z).typst()
            ));
        } else {
            parts.push(format!(
                "margin: (top: {}, bottom: {}, inside: {}, outside: {})",
                mg.top.unwrap_or(z).typst(),
                mg.bottom.unwrap_or(z).typst(),
                mg.inside.unwrap_or(z).typst(),
                mg.outside.unwrap_or(z).typst()
            ));
        }
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
                if let Some(names) = self.hide_names(e) {
                    let at = self.out.len();
                    self.element(e, false);
                    let body = self.out.split_off(at);
                    let names: String = names.iter().map(|n| format!("{}, ", str_lit(n))).collect();
                    self.raw(&format!("#context if not mt-hidden(({names})) [{body}]\n"));
                } else {
                    self.element(e, false);
                }
            }
            self.raw("]");
            self.labels = labels;
            let body = std::mem::replace(&mut self.out, saved);
            // Page layers are drawn from the bleed's corner; shift them so
            // their elements are placed from the trim's, like everything else.
            let b = m.bleed.and_then(|b| b.pt()).unwrap_or(0.0);
            let shifted = if b > 0.0 {
                format!(
                    "context place(top + left, dx: if calc.odd(here().page()) {{ 0pt }} else {{ {b}pt }}, dy: {b}pt, box({body}))",
                    b = matenadar_model::units::num(b)
                )
            } else {
                body
            };
            parts.push(format!("{key}: {shifted}"));
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
        let var = self.styles.get(name).cloned().unwrap_or_else(|| "mt-base".to_string());
        if !self.resolved.contains_key(&var) {
            let style = self.doc.design.style(if var == "mt-base" { "" } else { name });
            self.resolved.insert(var.clone(), style);
        }
        var
    }

    /// A style with local overrides, as a dictionary literal.
    pub fn style_with(&mut self, name: &str, set: Option<&Style>) -> String {
        match set {
            None => self.style_var(name),
            Some(over) => {
                let resolved = self.doc.design.style(name).with(over);
                let dict = self.style_dict(&resolved, false);
                self.resolved.insert(dict.clone(), resolved);
                dict
            }
        }
    }

    /// The style a style expression stands for (the base style if unknown).
    pub fn style_of(&self, expr: &str) -> Style {
        self.resolved.get(expr).cloned().unwrap_or_else(|| self.doc.design.style(""))
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
        // A family named as designers name it, by the name Typst knows it by.
        out.into_iter().map(|f| self.fonts.resolve(&f).unwrap_or(f)).collect()
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
            tx.push(format!("size: {}", font_size(*s).typst()));
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
        match t.ligatures {
            Some(l) => tx.push(format!("ligatures: {l}")),
            // Letter-spaced text drops its ligatures, as CSS and InDesign do.
            None if t.tracking.is_some_and(|tr| tr != Length::ZERO) => tx.push("ligatures: false".into()),
            None => {}
        }
        if let Some(o) = t.overhang {
            tx.push(format!("overhang: {o}"));
        }
        if let Some(s) = &t.outline {
            tx.push(format!("stroke: {}", self.pal.stroke(s, false)));
        }
        if let Some(v) = &t.variations
            && !v.is_empty()
        {
            let axes: Vec<String> = v.iter().map(|(k, val)| format!("{}: {}", str_lit(k), num(*val))).collect();
            tx.push(format!("variations: ({})", axes.join(", ")));
        }
        // A paragraph's line height comes in as `line_height_em`; a run's
        // own `line_height` sets the edges of that run alone.
        let run_lh = if line_height_em.is_none() { t.line_height } else { None };
        if let Some(LineHeight::Length(l)) = run_lh
            && l.resolve(0.0, 0.0) == 0.0
        {
            // A run that never opens its line.
            tx.push("top-edge: 0pt".into());
            tx.push("bottom-edge: 0pt".into());
        } else if let Some(LineHeight::Factor(f)) = run_lh
            && f == 0.0
        {
            tx.push("top-edge: 0pt".into());
            tx.push("bottom-edge: 0pt".into());
        } else if let Some(lh) = line_height_em.or_else(|| {
            let size = t.size.map(font_size).and_then(|s| s.pt()).unwrap_or(0.0);
            match run_lh {
                Some(LineHeight::Factor(f)) => Some(f),
                Some(LineHeight::Length(l)) if size > 0.0 => Some(l.resolve(size, size * 100.0) / size),
                _ => None,
            }
        }) {
            let mut m = self.metrics(t);
            let size = t.size.map(font_size).and_then(|s| s.pt()).filter(|s| *s > 0.0);
            // Browsers round a font's ascent and descent to whole CSS pixels
            // before sharing out the leading; do the same where the size is known.
            if let Some(size) = size {
                let px = size / 0.75;
                m.ascender = (m.ascender * px).round() / px;
                m.descender = (m.descender * px).round() / px;
            }
            let half = (lh - (m.ascender + m.descender)) / 2.0;
            // In em, so a larger run opens its line in proportion, as CSS does.
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
        let halo = t.halo.as_ref().map(|h| self.pal.stroke(h, false)).filter(|h| h != "none");
        let wrap = if wrap.is_empty() && halo.is_none() {
            "none".to_string()
        } else {
            let mut expr = "b".to_string();
            for w in wrap {
                expr = w.replace("(b)", &format!("({expr})")).replace(", b)", &format!(", {expr})"));
            }
            if let Some(h) = halo {
                expr = format!("mt-halo({h}, {expr})");
            }
            format!("b => {expr}")
        };
        format!("(text: ({}), wrap: {wrap})", if tx.is_empty() { ":".into() } else { tx.join(", ") })
    }

    /// How far below a paragraph's top its first baseline sits, in points
    /// (the top edge of the CSS line box `text_dict` sets up).
    pub fn first_baseline(&mut self, s: &Style) -> f64 {
        let size = s.text.size.map_or(11.0, |l| font_size(l).resolve(11.0, 0.0));
        let mut m = self.metrics(&s.text);
        let px = size / 0.75;
        m.ascender = (m.ascender * px).round() / px;
        m.descender = (m.descender * px).round() / px;
        let lh = match s.text.line_height {
            Some(LineHeight::Factor(f)) => f,
            Some(LineHeight::Length(l)) => l.resolve(size, size * 100.0) / size,
            None => m.ascender + m.descender,
        };
        let half = (lh - (m.ascender + m.descender)) / 2.0;
        (m.ascender + half) * size
    }

    /// A paragraph style as the dictionary `mt-par` reads.
    pub fn style_dict(&mut self, s: &Style, _scaled: bool) -> String {
        let mut t = s.text.clone();
        if t.lang.is_none()
            && let Some(lang) = &self.doc.meta.lang
        {
            t.lang = Some(lang.clone());
        }
        let size = t.size.map_or(11.0, |l| font_size(l).resolve(11.0, 0.0));
        let lh_em = match s.text.line_height {
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
        match s.linebreaks.as_deref() {
            Some("optimized") | Some("simple") => parts.push(format!("linebreaks: {}", str_lit(s.linebreaks.as_deref().unwrap()))),
            _ => {}
        }
        if let Some(j) = &s.justification {
            let lim = |l: &Option<matenadar_model::style::Limits>, min: &str, max: &str| {
                let l = l.unwrap_or_default();
                format!(
                    "(min: {}, max: {})",
                    l.min.map(|v| v.typst()).unwrap_or_else(|| min.into()),
                    l.max.map(|v| v.typst()).unwrap_or_else(|| max.into())
                )
            };
            parts.push(format!(
                "limits: (spacing: {}, tracking: {})",
                lim(&j.spacing, "66.67%", "150%"),
                lim(&j.tracking, "0em", "0em")
            ));
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
            d.push(format!("lh: {}", num(lh_em.unwrap_or(1.2))));
            parts.push(format!("drop: ({})", d.join(", ")));
        }
        format!("({})", parts.join(", "))
    }

}

/// Each time's vars for a repeated page: `n`, `date` with dates, and that
/// time's own.
fn repeat_vars(id: &str, r: &matenadar_model::Repeat, problems: &mut Vec<String>) -> Vec<String> {
    let first = r.dates.as_ref().and_then(|d| match matenadar_model::parse_date(&d.from) {
        Some((y, m, day)) => Some((matenadar_model::days_from_civil(y, m, day), d.step.unwrap_or(1))),
        None => {
            problems.push(format!("{id}: repeat.dates.from must be a date, YYYY-MM-DD"));
            None
        }
    });
    (0..r.times())
        .map(|i| {
            let mut v = IndexMap::new();
            v.insert("n".to_string(), (i + 1).to_string());
            if let Some((day0, step)) = first {
                let (y, m, d) = matenadar_model::civil_from_days(day0 + i as i64 * step);
                v.insert("date".to_string(), format!("{y:04}-{m:02}-{d:02}"));
            }
            if let Some(own) = r.vars.get(i) {
                v.extend(own.iter().map(|(k, val)| (k.clone(), val.clone())));
            }
            vars(&v)
        })
        .collect()
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
