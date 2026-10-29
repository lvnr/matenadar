//! Inline content: runs of text with their marks, notes, fields, page
//! references, line breaks and inline images.

use matenadar_model::block::{Inline, Run};

use super::escape::escape;
use super::typ::str_lit;
use super::{Gen, label_safe};

impl Gen<'_> {
    /// Writes inline content as Typst markup. `node` is the paragraph's id,
    /// for the source map; offsets count the plain text.
    pub fn inlines(&mut self, node: Option<&str>, content: &[Inline]) {
        self.inlines_at(node, 0, content);
    }

    /// Inline content whose words start at `base` of the node's words (a
    /// paragraph of a text frame is the frame's words from there on).
    pub fn inlines_at(&mut self, node: Option<&str>, base: usize, content: &[Inline]) {
        let mut at = base;
        for (i, x) in content.iter().enumerate() {
            // Typst tracks every letter of a run but its last, so a word split
            // into runs would lose letter spacing at each split: `mt-tr` adds
            // the run's own spacing after it, without a place to break.
            let glue = self.tracked && matches!(content.get(i + 1), Some(Inline::Plain(_) | Inline::Run(_) | Inline::Field(_) | Inline::Ref(_)));
            match x {
                Inline::Plain(s) => {
                    self.plain(node, at, s);
                    if glue {
                        self.raw("#mt-tr;");
                    }
                    at += s.len();
                }
                Inline::Run(r) => {
                    self.run(node, at, r, glue);
                    at += r.text.len();
                }
                Inline::Note(n) => {
                    let nbsp = self.nbsp_at.take();
                    self.raw("#footnote[");
                    self.inlines(None, &n.note);
                    self.raw("]");
                    self.nbsp_at = nbsp;
                }
                Inline::Field(f) => {
                    let wrap = self.run_wrap(f.span.as_deref(), &f.set, false, false);
                    let expr = format!("context mt-field({})", str_lit(&f.field));
                    match wrap {
                        Some(w) => self.raw(&format!("#mt-span({w}, {expr})")),
                        None => self.raw(&format!("#{expr}")),
                    }
                }
                Inline::Ref(r) => {
                    if label_safe(&r.target) {
                        if r.show.as_deref() == Some("title") {
                            self.raw(&format!(
                                "#context {{ let f = query(<{}>); if f.len() > 0 {{ f.first().value }} else {{ \"??\" }} }}",
                                r.target
                            ));
                        } else {
                            self.raw(&format!("#mt-page-of(<{}>)", r.target));
                        }
                    } else {
                        self.raw("??");
                    }
                }
                Inline::Break(_) => self.raw("#linebreak()"),
                Inline::Space(sp) => self.raw(&format!("#h({})", sp.space.typst())),
                Inline::Leader(l) => {
                    if l.leader.trim().is_empty() {
                        self.raw("#h(1fr)");
                    } else if l.leader == "rule" {
                        let stroke = l.stroke.as_ref().map(|s| self.pal.stroke(s, false)).unwrap_or_else(|| "0.5pt".into());
                        let gap = l.gap.map(|g| g.typst()).unwrap_or_else(|| "0.3em".into());
                        let lift = l.lift.map(|g| g.typst()).unwrap_or_else(|| "0.3em".into());
                        self.raw(&format!("#box(width: 1fr, inset: (x: {gap}), move(dy: -{lift}, line(length: 100%, stroke: {stroke})))"));
                    } else {
                        let mark = format!("[{}]", super::escape::escape(&l.leader).0);
                        let mark = match &l.span {
                            Some(sp) => format!("mt-span({}, {mark})", self.span_var(sp)),
                            None => mark,
                        };
                        let gap = l.gap.map(|g| g.typst()).unwrap_or_else(|| "0.2em".into());
                        let justify = l.fixed != Some(true);
                        self.raw(&format!("#box(width: 1fr, repeat(gap: {gap}, justify: {justify}, {mark}))"));
                    }
                }
                Inline::Image(img) => {
                    let path = if img.image.is_empty() { None } else { self.asset(&img.image) };
                    let mut args = Vec::new();
                    if let Some(h) = &img.h {
                        args.push(format!("height: {}", h.typst()));
                    }
                    if let Some(w) = &img.w {
                        args.push(format!("width: {}", w.typst()));
                    }
                    let baseline = img.baseline.map(|b| b.typst()).unwrap_or_else(|| "0pt".into());
                    let path = match &img.markup {
                        Some(m) if img.image.is_empty() => Some(self.file("svg", m.clone().into_bytes())),
                        _ => path,
                    };
                    match path {
                        Some(p) => self.raw(&format!("#box(baseline: {baseline}, image({}, {}))", str_lit(&p), args.join(", "))),
                        None => self.raw("#box(width: 1em, height: 1em, fill: red)"),
                    }
                }
            }
        }
    }

    pub(crate) fn plain(&mut self, node: Option<&str>, at: usize, text: &str) {
        let mut offset = at;
        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                self.raw("#linebreak()");
                offset += 1;
            }
            if part.is_empty() {
                continue;
            }
            // A space that must not break: written as Typst's `~` between the halves.
            match self.nbsp_at.filter(|&n| n >= offset && n < offset + part.len() && part.as_bytes()[n - offset] == b' ') {
                Some(n) => {
                    let cut = n - offset;
                    self.text_piece(node, offset, &part[..cut]);
                    self.raw("~");
                    self.text_piece(node, n + 1, &part[cut + 1..]);
                }
                None => self.text_piece(node, offset, part),
            }
            offset += part.len();
        }
    }

    fn text_piece(&mut self, node: Option<&str>, offset: usize, part: &str) {
        if part.is_empty() {
            return;
        }
        let (part, inserted) = if self.hyphenating && !self.hyphenation.is_empty() {
            super::escape::with_breaks(part, &self.hyphenation)
        } else {
            (part.to_string(), Vec::new())
        };
        let (escaped, mut escapes) = escape(&part);
        if !inserted.is_empty() {
            // Where the added characters landed after escaping: each byte
            // moves right by the backslashes added before it.
            let sources: Vec<usize> = escapes.iter().enumerate().map(|(i, e)| e - i).collect();
            for q in inserted {
                escapes.push(q + sources.partition_point(|&p| p < q));
            }
            escapes.sort_unstable();
        }
        match node {
            Some(n) => self.text_segment(n, offset, &escaped, escapes),
            None => self.raw(&escaped),
        }
    }

    /// The span dictionary for a run's character style and overrides, if any.
    fn run_wrap(
        &mut self,
        span: Option<&str>,
        set: &matenadar_model::TextStyle,
        _em: bool,
        _strong: bool,
    ) -> Option<String> {
        let has_set = *set != matenadar_model::TextStyle::default();
        match (span, has_set) {
            (None, false) => None,
            (Some(name), false) => Some(self.span_var(name)),
            (name, true) => {
                let base = name.and_then(|n| self.doc.design.spans.get(n).cloned()).unwrap_or_default();
                let merged = base.with(set);
                Some(self.text_dict(&merged, None))
            }
        }
    }

    fn run(&mut self, node: Option<&str>, at: usize, r: &Run, glue: bool) {
        let mut closers = Vec::new();
        if let Some(w) = self.run_wrap(r.span.as_deref(), &r.set, r.em, r.strong) {
            self.raw(&format!("#mt-span({w})["));
            closers.push("]");
        }
        if r.em {
            self.raw("#emph[");
            closers.push("]");
        }
        if r.strong {
            self.raw("#text(weight: \"bold\")[");
            closers.push("]");
        }
        if r.sup {
            self.raw("#super[");
            closers.push("]");
        }
        if r.sub {
            self.raw("#sub[");
            closers.push("]");
        }
        if let Some(url) = &r.link {
            self.raw(&format!("#link({})[", str_lit(url)));
            closers.push("]");
        }
        if r.nobreak {
            self.raw("#box[");
            closers.push("]");
        }
        if let Some(m) = &r.mark {
            let stroke = m.stroke.as_ref().map(|s| self.pal.stroke(s, false));
            let mut args = Vec::new();
            let call = match m.kind.as_str() {
                "underline" | "strike" => {
                    if let Some(s) = &stroke {
                        args.push(format!("stroke: {s}"));
                    }
                    if let Some(o) = &m.offset {
                        args.push(format!("offset: {}", o.typst()));
                    }
                    if let Some(e) = &m.extent {
                        args.push(format!("extent: {}", e.typst()));
                    }
                    if m.kind == "underline" {
                        args.push("evade: false".into());
                    }
                    m.kind.clone()
                }
                "highlight" => {
                    args.push(format!("fill: {}", m.fill.as_ref().map(|f| self.pal.fill(f)).unwrap_or_else(|| "rgb(\"#fff3a8\")".into())));
                    if let Some(e) = &m.extent {
                        args.push(format!("extent: {}", e.typst()));
                    }
                    if let Some(r) = &m.radius {
                        args.push(format!("radius: {}", super::typ::radius(r, false)));
                    }
                    "highlight".into()
                }
                kind => {
                    // A box or a ring around the run.
                    args.push(format!("stroke: {}", stroke.clone().unwrap_or_else(|| "0.8pt".into())));
                    if let Some(f) = &m.fill {
                        args.push(format!("fill: {}", self.pal.fill(f)));
                    }
                    let inset = m.inset.map(|i| i.typst()).unwrap_or_else(|| "0.15em".into());
                    args.push(format!("outset: (x: {inset}, y: {inset})"));
                    let radius = match (&m.radius, kind) {
                        (Some(r), _) => super::typ::radius(r, false),
                        (None, "ring") => "50%".into(),
                        _ => "0pt".into(),
                    };
                    args.push(format!("radius: {radius}"));
                    "box".into()
                }
            };
            self.raw(&format!("#{call}({})[", args.join(", ")));
            closers.push("]");
        }
        self.plain(node, at, &r.text);
        if glue {
            self.raw("#mt-tr;");
        }
        for c in closers.iter().rev() {
            self.raw(c);
        }
    }

    /// An asset's path for Typst, if the file exists under the root.
    pub fn asset(&mut self, src: &str) -> Option<String> {
        let rel = src.trim_start_matches('/');
        if rel.contains("..") {
            self.problem(format!("asset paths may not leave the asset root: {src}"));
            return None;
        }
        if self.assets.exists(rel) {
            Some(format!("/{rel}"))
        } else {
            self.problem(format!("missing file: {src}"));
            None
        }
    }
}
