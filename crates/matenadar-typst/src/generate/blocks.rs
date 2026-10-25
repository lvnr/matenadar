//! Blocks: what flows through a section's pages and fills frames.

use matenadar_model::block::{Block, Cell, Inline, ListItem, Para, content_of, plain_text};
use matenadar_model::style::Align;
use matenadar_model::units::num;

use super::typ::{insets, radius, str_lit};
use super::{Gen, label_safe};

/// How a list of blocks is being written.
#[derive(Clone)]
pub struct FlowCx {
    /// The Typst expression of the default paragraph style.
    pub style: String,
    /// Inside a frame that shrinks to fit: styles take `k`.
    pub scaled: bool,
}

impl Gen<'_> {
    pub fn flow(&mut self, blocks: &[Block], style: &str, scaled: bool) {
        let cx = FlowCx { style: style.to_string(), scaled };
        let mut prev_para = false;
        for (i, b) in blocks.iter().enumerate() {
            // A margin note that sits level with this block needs a marker in it.
            let anchor = match blocks.get(i + 1) {
                Some(Block::Margin(m)) if m.at.as_deref() != Some("here") => {
                    self.anchors += 1;
                    Some(format!("mt-a-{}", self.anchors))
                }
                _ => None,
            };
            let first = !prev_para;
            self.block(b, &cx, first, anchor.as_deref());
            prev_para = match b {
                Block::P(_) => true,
                Block::Margin(_) | Block::Space(_) => prev_para,
                _ => false,
            };
        }
    }

    fn k(&self, cx: &FlowCx) -> &'static str {
        if cx.scaled { ", k: k" } else { "" }
    }

    fn style_expr(&mut self, name: Option<&str>, set: Option<&matenadar_model::Style>, cx: &FlowCx) -> String {
        match (name, set) {
            (Some(n), s) => self.style_with(n, s),
            (None, None) => cx.style.clone(),
            (None, Some(over)) => {
                // Overrides on the flow's default style: resolve that style's name when it is a named one.
                let base = self.styles_by_var(&cx.style);
                self.style_with(base.as_deref().unwrap_or(""), Some(over))
            }
        }
    }

    fn styles_by_var(&self, var: &str) -> Option<String> {
        self.style_names().find(|(_, v)| *v == var).map(|(n, _)| n.to_string())
    }

    pub fn block(&mut self, b: &Block, cx: &FlowCx, first: bool, anchor: Option<&str>) {
        let start = self.out.len();
        let k = self.k(cx);
        let marker = |g: &mut Self| {
            if let Some(a) = anchor {
                g.raw(&format!("#metadata(none) <{a}>"));
            }
        };
        match b {
            Block::P(p) => {
                self.paragraph(p, cx, first, anchor);
                return;
            }
            Block::H(h) => {
                let name = h.style.clone().or_else(|| {
                    let n = format!("h{}", h.level);
                    self.doc.design.styles.contains_key(&n).then_some(n)
                });
                let st = self.style_expr(name.as_deref(), h.set.as_ref(), cx);
                let content = content_of(&h.text, &h.content);
                let title = h.short.clone().unwrap_or_else(|| plain_text(&content));
                self.raw(&format!("#mt-head({}, {}, {}, {st}{k})[", h.level.max(1), h.outline.unwrap_or(true), str_lit(&title)));
                self.label(h.id.as_deref());
                marker(self);
                self.inlines(h.id.as_deref(), &content);
                self.raw("]\n");
            }
            Block::Break(x) => {
                let name = x.style.clone().or_else(|| self.doc.design.styles.contains_key("break").then(|| "break".to_string()));
                let st = match &name {
                    Some(n) => self.style_var(n),
                    None => "(above: 1em, below: 1em, align: \"center\", text: (:))".to_string(),
                };
                self.raw(&format!("#mt-par({st}, first: true{k})[#align(center)["));
                self.label(x.id.as_deref());
                marker(self);
                let text = x.text.clone().unwrap_or_else(|| "*  *  *".into());
                self.inlines(None, &[Inline::Plain(text)]);
                self.raw("]]\n");
            }
            Block::Figure(f) => {
                let mut args = vec!["width: 100%".to_string()];
                if let Some(w) = &f.w {
                    args[0] = format!("width: {}", w.typst());
                }
                if let Some(h) = &f.h {
                    args.push(format!("height: {}", h.typst()));
                }
                args.push(format!("fit: {}", str_lit(f.fit.as_deref().unwrap_or("contain"))));
                let img = match self.asset(&f.src) {
                    Some(p) => format!("image({}, {})", str_lit(&p), args.join(", ")),
                    None => format!("rect({}, fill: luma(85%))", args.iter().filter(|a| !a.starts_with("fit")).cloned().collect::<Vec<_>>().join(", ")),
                };
                let img = match (&f.radius, &f.stroke) {
                    (None, None) => img,
                    (r, s) => format!(
                        "box(clip: true{}{}, {img})",
                        r.as_ref().map(|r| format!(", radius: {}", radius(r, false))).unwrap_or_default(),
                        s.as_ref().map(|s| format!(", stroke: {}", self.pal.border(s, false))).unwrap_or_default()
                    ),
                };
                let al = align_word(f.align);
                let above = f.space_before.map(|l| l.typst()).unwrap_or_else(|| "0.8em".into());
                let below = f.space_after.map(|l| l.typst()).unwrap_or_else(|| "0.8em".into());
                let body_start = format!("block(above: {above}, below: {below}, breakable: false, width: 100%, align({al}, {{ ");
                let float = matches!(f.float.as_deref(), Some("top") | Some("bottom"));
                if float {
                    self.raw(&format!("#place({}, float: true, clearance: 1em, {body_start}", f.float.as_deref().unwrap_or("top")));
                } else {
                    self.raw(&format!("#{body_start}"));
                }
                self.raw("[");
                self.label(f.id.as_deref());
                marker(self);
                self.raw(&format!("]; {img}"));
                let caption = content_of(&f.caption, &f.caption_content);
                if !caption.is_empty() {
                    let st = match &f.caption_style {
                        Some(n) => self.style_var(n),
                        None if self.doc.design.styles.contains_key("caption") => self.style_var("caption"),
                        None => cx.style.clone(),
                    };
                    self.raw(&format!("; mt-par({st}, first: true{k})["));
                    self.inlines(None, &caption);
                    self.raw("]");
                }
                self.raw(if float { " }))\n" } else { " }))\n" });
                if float {
                    self.raw(")\n");
                }
            }
            Block::Box(x) => {
                let mut args = Vec::new();
                args.push(format!("width: {}", x.w.map(|w| w.typst()).unwrap_or_else(|| "100%".into())));
                if let Some(h) = &x.h {
                    args.push(format!("height: {}", h.typst()));
                }
                if let Some(f) = &x.fill {
                    args.push(format!("fill: {}", self.pal.fill(f)));
                }
                if let Some(s) = &x.stroke {
                    args.push(format!("stroke: {}", self.pal.border(s, cx.scaled)));
                }
                if let Some(r) = &x.radius {
                    args.push(format!("radius: {}", radius(r, cx.scaled)));
                }
                if let Some(i) = &x.inset {
                    args.push(format!("inset: {}", insets(i, cx.scaled)));
                }
                args.push(format!("breakable: {}", x.breakable.unwrap_or(false)));
                args.push(format!("above: {}", x.space_before.map(|l| l.typst()).unwrap_or_else(|| "0.8em".into())));
                args.push(format!("below: {}", x.space_after.map(|l| l.typst()).unwrap_or_else(|| "0.8em".into())));
                let al = align_word(x.align);
                let rot = x.rotate.filter(|r| *r != 0.0);
                if let Some(r) = rot {
                    self.raw(&format!("#block(width: 100%, align({al}, rotate({}deg, reflow: false, block({})[", num(r), args.join(", ")));
                } else {
                    self.raw(&format!("#align({al}, block({})[", args.join(", ")));
                }
                self.label(x.id.as_deref());
                marker(self);
                let inner = FlowCx { style: x.style.as_deref().map(|n| self.style_var(n)).unwrap_or_else(|| cx.style.clone()), scaled: cx.scaled };
                self.flow(&x.blocks, &inner.style.clone(), inner.scaled);
                self.raw(if rot.is_some() { "])))\n" } else { "])\n" });
            }
            Block::Table(t) => self.table(t, cx, anchor),
            Block::List(l) => {
                let st = l.style.as_deref().map(|n| self.style_var(n)).unwrap_or_else(|| cx.style.clone());
                let ordered = l.ordered.unwrap_or(false);
                let mut args = Vec::new();
                if ordered {
                    args.push(format!("numbering: {}", str_lit(l.marker.as_deref().unwrap_or("1."))));
                    if let Some(s) = l.start {
                        args.push(format!("start: {s}"));
                    }
                } else {
                    args.push(format!("marker: [{}]", super::escape::escape(l.marker.as_deref().unwrap_or("•")).0));
                }
                args.push(format!("indent: {}", l.indent.map(|i| i.typst()).unwrap_or_else(|| "0pt".into())));
                args.push(format!("body-indent: {}", l.gap.map(|i| i.typst()).unwrap_or_else(|| "0.5em".into())));
                args.push(format!("spacing: {}", l.spacing.map(|i| i.typst()).unwrap_or_else(|| "0pt".into())));
                self.raw(&format!("#block(width: 100%, above: 0.6em, below: 0.6em)[#set text(..{st}.text)"));
                self.label(l.id.as_deref());
                marker(self);
                self.raw(&format!("#{}({}", if ordered { "enum" } else { "list" }, args.join(", ")));
                for item in &l.items {
                    self.raw(", [");
                    match item {
                        ListItem::Text(t) => {
                            self.raw(&format!("#mt-par({st}, first: true{k})["));
                            self.inlines(None, &[Inline::Plain(t.clone())]);
                            self.raw("]");
                        }
                        ListItem::Blocks(bs) => self.flow(bs, &st.clone(), cx.scaled),
                    }
                    self.raw("]");
                }
                self.raw(")]\n");
            }
            Block::Code(c) => {
                let st = c.style.as_deref().map(|n| self.style_var(n)).unwrap_or_else(|| {
                    if self.doc.design.styles.contains_key("code") { self.style_var("code") } else { cx.style.clone() }
                });
                let mut args = vec!["width: 100%".to_string()];
                if let Some(f) = &c.fill {
                    args.push(format!("fill: {}", self.pal.fill(f)));
                }
                if let Some(s) = &c.stroke {
                    args.push(format!("stroke: {}", self.pal.border(s, false)));
                }
                if let Some(i) = &c.inset {
                    args.push(format!("inset: {}", insets(i, false)));
                }
                if let Some(r) = &c.radius {
                    args.push(format!("radius: {}", radius(r, false)));
                }
                args.push(format!("breakable: {}", c.breakable.unwrap_or(true)));
                args.push(format!("above: {}", c.space_before.map(|l| l.typst()).unwrap_or_else(|| "0.8em".into())));
                args.push(format!("below: {}", c.space_after.map(|l| l.typst()).unwrap_or_else(|| "0.8em".into())));
                self.raw(&format!("#block({})[", args.join(", ")));
                self.label(c.id.as_deref());
                marker(self);
                self.raw(&format!(
                    "#show raw: set text(..{st}.text)#set par(leading: 0pt, spacing: 0pt, justify: false)#raw({}, block: true)]\n",
                    str_lit(&c.text)
                ));
            }
            Block::Canvas(c) => {
                let mut args = vec![format!("width: {}", c.w.map(|w| w.typst()).unwrap_or_else(|| "100%".into()))];
                args.push(format!("height: {}", c.h.map(|h| h.typst()).unwrap_or_else(|| "auto".into())));
                if let Some(f) = &c.fill {
                    args.push(format!("fill: {}", self.pal.fill(f)));
                }
                args.push(format!("clip: {}", c.clip.unwrap_or(false)));
                args.push("breakable: false".into());
                args.push(format!("above: {}", c.space_before.map(|l| l.typst()).unwrap_or_else(|| "0.8em".into())));
                args.push(format!("below: {}", c.space_after.map(|l| l.typst()).unwrap_or_else(|| "0.8em".into())));
                self.raw(&format!("#align({}, block({})[", align_word(c.align), args.join(", ")));
                self.label(c.id.as_deref());
                marker(self);
                for e in &c.elements {
                    self.element(e, cx.scaled);
                }
                self.raw("])\n");
            }
            Block::Use(u) => {
                let comp = self.doc.design.components.get(&u.component).cloned();
                match comp.map(|c| c.expand(&u.component, &u.props)) {
                    Some(Ok(x)) if self.depth < 8 => {
                        self.depth += 1;
                        if !x.blocks.is_empty() {
                            self.raw("#block(width: 100%)[");
                            self.label(u.id.as_deref());
                            marker(self);
                            self.flow(&x.blocks, &cx.style.clone(), cx.scaled);
                            self.raw("]\n");
                        } else {
                            let w = x.w.map(|w| w.typst()).unwrap_or_else(|| "100%".into());
                            let h = x.h.map(|h| h.typst()).unwrap_or_else(|| "auto".into());
                            self.raw(&format!("#block(width: {w}, height: {h}, breakable: false)["));
                            self.label(u.id.as_deref());
                            marker(self);
                            for e in &x.elements {
                                self.element(e, cx.scaled);
                            }
                            self.raw("]\n");
                        }
                        self.depth -= 1;
                    }
                    Some(Err(e)) => self.problem(e),
                    _ => self.problem(format!("component \"{}\" is missing or nested too deeply", u.component)),
                }
            }
            Block::PageBreak(p) => match p.to.as_deref() {
                Some("recto") | Some("odd") => self.raw("#pagebreak(to: \"odd\")\n"),
                Some("verso") | Some("even") => self.raw("#pagebreak(to: \"even\")\n"),
                _ => self.raw("#pagebreak()\n"),
            },
            Block::Space(s) => self.raw(&format!("#v({}, weak: {})\n", s.h.typst(), s.weak.unwrap_or(true))),
            Block::ColumnBreak(_) => self.raw("#colbreak()\n"),
            Block::Columns(c) => {
                let gutter = c.gutter.map(|g| g.typst()).unwrap_or_else(|| "12pt".into());
                let rule = c.rule.as_ref().map(|r| self.pal.stroke(r, false)).unwrap_or_else(|| "none".into());
                if c.balance == Some(true) {
                    self.raw("#block(width: 100%)[");
                    self.label(c.id.as_deref());
                    marker(self);
                    self.raw(&format!("#mt-balance({}, {gutter}, {rule}, (", c.count.max(1)));
                    for b in &c.blocks {
                        self.raw("[");
                        self.flow(std::slice::from_ref(b), &cx.style.clone(), cx.scaled);
                        self.raw("], ");
                    }
                    self.raw("))]\n");
                } else {
                    self.raw(&format!("#block(width: 100%)[#set columns(gutter: {gutter})#columns({})[", c.count.max(1)));
                    self.label(c.id.as_deref());
                    marker(self);
                    self.flow(&c.blocks, &cx.style.clone(), cx.scaled);
                    self.raw("]]\n");
                }
            }
            Block::Margin(m) => {
                let notes = self.notes.clone();
                let (side, w, gap, spacing, nst) = match &notes {
                    Some(n) => (
                        n.side.clone().unwrap_or_else(|| "outside".into()),
                        n.width.typst(),
                        n.gap.map(|g| g.typst()).unwrap_or_else(|| "12pt".into()),
                        n.spacing.map(|g| g.typst()).unwrap_or_else(|| "6pt".into()),
                        n.style.clone(),
                    ),
                    None => {
                        self.problem("a margin note sits on a master without a note column".into());
                        ("outside".into(), "1in".into(), "12pt".into(), "6pt".into(), None)
                    }
                };
                let st = m
                    .style
                    .clone()
                    .or(nst)
                    .or_else(|| self.doc.design.styles.contains_key("note").then(|| "note".to_string()))
                    .map(|n| self.style_var(&n))
                    .unwrap_or_else(|| cx.style.clone());
                let anchor_label = self.margin_anchor.take();
                let anchor = match (&m.at, anchor_label) {
                    (Some(at), _) if at == "here" => "none".to_string(),
                    (_, Some(l)) => format!("<{l}>"),
                    _ => "none".to_string(),
                };
                let dy = m.dy.map(|d| d.typst()).unwrap_or_else(|| "0pt".into());
                self.raw(&format!("#mt-note({}, {w}, {gap}, {spacing}, {anchor}, {dy})[", str_lit(&side)));
                self.label(m.id.as_deref());
                if !m.blocks.is_empty() {
                    self.flow(&m.blocks, &st, false);
                } else {
                    let content = content_of(&m.text, &m.content);
                    self.raw(&format!("#mt-par({st}, first: true)["));
                    self.inlines(m.id.as_deref(), &content);
                    self.raw("]");
                }
                self.raw("]\n");
            }
            Block::Toc(t) => {
                let depth = t.depth.unwrap_or(1).max(1);
                let styles: Vec<String> = (1..=depth)
                    .map(|lvl| {
                        let name = match &t.style {
                            Some(s) if depth == 1 => s.clone(),
                            Some(s) => format!("{s}-{lvl}"),
                            None => format!("toc-{lvl}"),
                        };
                        if self.doc.design.styles.contains_key(&name) { self.style_var(&name) } else { cx.style.clone() }
                    })
                    .collect();
                let leader = match &t.leader {
                    Some(l) => format!("[{}]", super::escape::escape(l).0),
                    None => "none".into(),
                };
                self.raw(&format!("#block(width: 100%)["));
                self.label(t.id.as_deref());
                self.raw(&format!("#mt-toc({depth}, ({},), {leader})]\n", styles.join(", ")));
            }
            Block::Keep(x) => {
                self.raw("#block(breakable: false, width: 100%)[");
                self.label(x.id.as_deref());
                marker(self);
                self.flow(&x.blocks, &cx.style.clone(), cx.scaled);
                self.raw("]\n");
            }
        }
        if anchor.is_some() {
            self.margin_anchor = anchor.map(str::to_string);
        }
        self.node(b.id(), b.kind(), start);
    }

    fn paragraph(&mut self, p: &Para, cx: &FlowCx, first: bool, anchor: Option<&str>) {
        let start = self.out.len();
        let st = self.style_expr(p.style.as_deref(), p.set.as_ref(), cx);
        let page_break = p
            .style
            .as_deref()
            .map(|n| self.doc.design.style(n).page_break_before == Some(true))
            .unwrap_or(false);
        if page_break {
            self.raw("#pagebreak(weak: true)\n");
        }
        let k = self.k(cx);
        // A drop cap needs plain text it can split into words.
        let drop = p.style.as_deref().map(|n| self.doc.design.style(n).drop_cap.is_some()).unwrap_or(false)
            || p.set.as_ref().is_some_and(|s| s.drop_cap.is_some());
        let plain = match (&p.text, p.content.as_slice()) {
            (Some(t), _) => Some(t.clone()),
            (None, [matenadar_model::block::Inline::Plain(t)]) => Some(t.clone()),
            _ => None,
        };
        if drop && let Some(text) = plain.filter(|t| !t.trim().is_empty()) {
            let mut chars = text.trim_start().chars();
            let mut cap: String = chars.next().map(String::from).unwrap_or_default();
            // An opening quote goes with the cap.
            if matches!(cap.as_str(), "“" | "\"" | "‘" | "'" | "«") {
                cap.extend(chars.next());
            }
            let rest: String = chars.collect();
            self.raw(&format!("#mt-drop({st}, first: {first}{k}, {}, {})", str_lit(&cap), str_lit(&rest)));
            if let Some(id) = p.id.as_deref().filter(|id| self.labels && label_safe(id)) {
                self.raw(&format!("#metadata({}) <{id}>", str_lit(id)));
            }
            self.raw("\n");
            self.node(p.id.as_deref(), "p", start);
            return;
        }
        if let Some(a) = anchor {
            // At the paragraph's top edge, where CSS would put a note beside it.
            self.raw(&format!("#metadata(none) <{a}>\n"));
            self.margin_anchor = Some(a.to_string());
        }
        self.raw(&format!("#mt-par({st}, first: {first}{k})["));
        self.label(p.id.as_deref());
        let content = content_of(&p.text, &p.content);
        self.inlines(p.id.as_deref(), &content);
        self.raw("]\n");
        self.node(p.id.as_deref(), "p", start);
    }

    fn table(&mut self, t: &matenadar_model::block::Table, cx: &FlowCx, anchor: Option<&str>) {
        let st = t.style.as_deref().map(|n| self.style_var(n)).unwrap_or_else(|| cx.style.clone());
        let hst = t.header_style.as_deref().map(|n| self.style_var(n)).unwrap_or_else(|| st.clone());
        let cols: Vec<String> = t
            .columns
            .iter()
            .map(|c| match c {
                matenadar_model::block::Track::Length(l) => l.typst(),
                matenadar_model::block::Track::Word(w) => w.clone(),
            })
            .collect();
        let stroke = match &t.stroke {
            None => "0.5pt + luma(60%)".to_string(),
            Some(v) => match serde_json::from_value::<matenadar_model::style::Border>(v.clone()) {
                Ok(b) => self.pal.border(&b, false),
                Err(_) => "none".into(),
            },
        };
        let hn = t.header.len();
        let hf = t.header_fill.as_ref().map(|f| self.pal.fill(f)).unwrap_or_else(|| "none".into());
        let zf = t.zebra.as_ref().map(|f| self.pal.fill(f)).unwrap_or_else(|| "none".into());
        let aligns: Vec<&str> = t.align.iter().map(|a| match a {
            Align::Right => "right",
            Align::Center => "center",
            _ => "left",
        }).collect();
        let align = if aligns.is_empty() { "left".to_string() } else { format!("(x, y) => ({},).at(x, default: left)", aligns.join(", ")) };
        let inset = t.inset.as_ref().map(|i| insets(i, false)).unwrap_or_else(|| "(x: 4pt, y: 3pt)".into());
        self.raw(&format!(
            "#block(width: 100%, breakable: {}, above: {}, below: {})[",
            t.breakable.unwrap_or(true),
            t.space_before.map(|l| l.typst()).unwrap_or_else(|| "0.8em".into()),
            t.space_after.map(|l| l.typst()).unwrap_or_else(|| "0.8em".into())
        ));
        self.label(t.id.as_deref());
        if let Some(a) = anchor {
            self.raw(&format!("#metadata(none) <{a}>"));
        }
        self.raw(&format!(
            "#table(columns: ({},), stroke: {stroke}, inset: {inset}, align: {align}, fill: (x, y) => if y < {hn} {{ {hf} }} else if calc.odd(y - {hn}) {{ {zf} }} else {{ none }}",
            cols.join(", ")
        ));
        if hn > 0 {
            self.raw(&format!(", table.header(repeat: {}", t.repeat_header.unwrap_or(true)));
            for row in &t.header {
                for cell in row {
                    self.raw(", ");
                    self.cell(cell, &hst, cx);
                }
            }
            self.raw(")");
        }
        for row in &t.rows {
            for cell in row {
                self.raw(", ");
                self.cell(cell, &st, cx);
            }
        }
        self.raw(")]\n");
    }

    fn cell(&mut self, cell: &Cell, st: &str, cx: &FlowCx) {
        match cell {
            Cell::Text(t) => {
                self.raw(&format!("mt-par({st}, first: true, w: auto)["));
                self.inlines(None, &[Inline::Plain(t.clone())]);
                self.raw("]");
            }
            Cell::Full(c) => {
                let mut args = Vec::new();
                if let Some(n) = c.colspan {
                    args.push(format!("colspan: {n}"));
                }
                if let Some(n) = c.rowspan {
                    args.push(format!("rowspan: {n}"));
                }
                if let Some(f) = &c.fill {
                    args.push(format!("fill: {}", self.pal.fill(f)));
                }
                if let Some(a) = c.align {
                    args.push(format!("align: {}", align_word(Some(a))));
                }
                let cst = c.style.as_deref().map(|n| self.style_var(n)).unwrap_or_else(|| st.to_string());
                self.raw(&format!("table.cell({})[", args.join(", ")));
                if !c.blocks.is_empty() {
                    self.flow(&c.blocks, &cst, cx.scaled);
                } else {
                    let content = content_of(&c.text, &c.content);
                    self.raw(&format!("#mt-par({cst}, first: true, w: auto)["));
                    self.inlines(None, &content);
                    self.raw("]");
                }
                self.raw("]");
            }
        }
    }
}

pub fn align_word(a: Option<Align>) -> &'static str {
    match a {
        Some(Align::Right) => "right",
        Some(Align::Center) => "center",
        _ => "left",
    }
}

#[allow(dead_code)]
fn _label_safe(id: &str) -> bool {
    label_safe(id)
}
