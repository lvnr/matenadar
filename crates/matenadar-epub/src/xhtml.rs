//! A section's blocks and runs as XHTML.

use std::collections::HashMap;
use std::sync::Arc;

use matenadar_model::block::{Block, Cell, Inline, ListItem, Run, Track, content_of, plain_text};
use matenadar_model::element::Element;
use matenadar_model::style::{Border, Fill, Insets, Stroke, Style};
use matenadar_model::units::{Length, num};
use matenadar_model::walk::{Node, walk};
use matenadar_model::{Document, Item};
use matenadar_typst::{Fonts, Paths, Session};

use crate::css::{Css, class};
use crate::{Assets, jpeg};

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            // Characters XML does not allow.
            c if (c as u32) < 0x20 && !matches!(c, '\n' | '\t') => {}
            c => out.push(c),
        }
    }
    out
}

/// `::first-letter` rules for styles with a drop cap.
pub fn drop_cap_rules(doc: &Document, css: &Css) -> String {
    let mut out = String::new();
    for name in doc.design.styles.keys() {
        let st = doc.design.style(name);
        if let Some(dc) = &st.drop_cap {
            let lh = match st.text.line_height {
                Some(matenadar_model::style::LineHeight::Factor(f)) => f,
                Some(matenadar_model::style::LineHeight::Length(l)) => {
                    let size = st.text.size.and_then(|s| s.pt()).unwrap_or(css.base);
                    l.resolve(size, size * 100.0) / size
                }
                None => 1.2,
            };
            let mut d = vec![
                "float: left".to_string(),
                format!("font-size: {}em", num(dc.lines as f64 * lh * 0.95)),
                "line-height: 0.85".into(),
                "margin: 0.04em 0.08em 0 0".into(),
            ];
            if let Some(c) = &dc.color {
                d.push(format!("color: {}", css.color(&c.0)));
            }
            if let Some(w) = &dc.weight {
                d.push(format!("font-weight: {}", matenadar_typst::glyphs::weight_number(w)));
            }
            out.push_str(&format!(".{}::first-letter {{ {} }}\n", class("s", name), d.join("; ")));
        }
    }
    out
}

pub struct Writer<'a> {
    doc: &'a Document,
    css: &'a Css<'a>,
    session: &'a Session,
    paths: &'a Paths,
    fonts: Arc<Fonts>,
    dpi: f32,
    /// A node's id to the file it is in.
    files: HashMap<String, String>,
    /// A node's id to its title (headings, sections), for references.
    titles: HashMap<String, String>,
    out: String,
    notes: Vec<String>,
    note_no: usize,
    headings: Vec<(u8, String, String)>,
    heading_no: usize,
    section: Option<&'a matenadar_model::Section>,
    /// The width of the section's text column, in points.
    measure: f64,
}

impl<'a> Writer<'a> {
    pub fn new(doc: &'a Document, css: &'a Css<'a>, session: &'a Session, paths: &'a Paths, fonts: Arc<Fonts>, dpi: f32) -> Self {
        let mut files = HashMap::new();
        let mut titles = HashMap::new();
        let file_of = |path: &str| -> Option<String> {
            let i: usize = path.strip_prefix("content[")?.split(']').next()?.parse().ok()?;
            Some(match doc.content.get(i)? {
                Item::Section(s) => format!("{}.xhtml", super::file_stem("s", &s.id, i)),
                Item::Page(p) => format!("{}.xhtml", super::file_stem("p", &p.id, i)),
            })
        };
        walk(doc, &mut |path, node| {
            let Some(file) = file_of(path) else { return };
            match node {
                Node::Item(item) => {
                    files.insert(item.id().to_string(), file);
                    if let Item::Section(s) = item
                        && let Some(t) = &s.title
                    {
                        titles.insert(s.id.clone(), t.clone());
                    }
                }
                Node::Block(b) => {
                    if let Some(id) = b.id() {
                        files.insert(id.to_string(), file);
                        if let Block::H(h) = b {
                            titles.insert(id.to_string(), h.short.clone().unwrap_or_else(|| plain_text(&content_of(&h.text, &h.content))));
                        }
                    }
                }
                Node::Element(e) => {
                    if let Some(id) = e.place().and_then(|p| p.id.as_deref()) {
                        files.insert(id.to_string(), file);
                    }
                }
                Node::Inline(_) => {}
            }
        });
        Writer {
            doc,
            css,
            session,
            paths,
            fonts,
            dpi,
            files,
            titles,
            out: String::new(),
            notes: Vec::new(),
            note_no: 0,
            headings: Vec::new(),
            heading_no: 0,
            section: None,
            measure: 288.0,
        }
    }

    /// A section as the body of one XHTML file, and the headings in it.
    pub fn section(&mut self, s: &'a matenadar_model::Section, _file: &str, assets: &mut Assets) -> (String, Vec<(u8, String, String)>) {
        self.out.clear();
        self.notes.clear();
        self.headings.clear();
        self.section = Some(s);
        let master = self.doc.design.master(&s.master).unwrap_or_default();
        let mg = master.margins.clone().unwrap_or_default();
        let side = |l: Option<Length>| l.and_then(|l| l.pt()).unwrap_or(36.0);
        let (a, b) = if mg.left.is_some() || mg.right.is_some() { (mg.left, mg.right) } else { (mg.inside, mg.outside) };
        self.measure = master.width.and_then(|w| w.pt()).unwrap_or(432.0) - side(a) - side(b);
        let style = s
            .style
            .clone()
            .or(master.style.clone())
            .or_else(|| self.doc.design.styles.contains_key("body").then(|| "body".to_string()));
        self.out.push_str(&format!("<section id=\"{}\" epub:type=\"chapter\">\n", esc(&s.id)));
        self.blocks(&s.flow, style.as_deref(), assets);
        self.out.push_str("</section>\n");
        if !self.notes.is_empty() {
            self.out.push_str("<section class=\"notes\" epub:type=\"footnotes\">\n");
            for n in std::mem::take(&mut self.notes) {
                self.out.push_str(&n);
            }
            self.out.push_str("</section>\n");
        }
        (std::mem::take(&mut self.out), std::mem::take(&mut self.headings))
    }

    fn blocks(&mut self, blocks: &[Block], style: Option<&str>, assets: &mut Assets) {
        let mut prev_para = false;
        for b in blocks {
            self.block(b, style, !prev_para, assets);
            prev_para = match b {
                Block::P(_) => true,
                Block::Margin(_) | Block::Space(_) => prev_para,
                _ => false,
            };
        }
    }

    fn id_attr(id: Option<&str>) -> String {
        id.map(|i| format!(" id=\"{}\"", esc(i))).unwrap_or_default()
    }

    fn block(&mut self, b: &Block, style: Option<&str>, first: bool, assets: &mut Assets) {
        match b {
            Block::P(p) => {
                let name = p.style.as_deref().or(style);
                let resolved = self.resolved(name, p.set.as_ref());
                let mut classes: Vec<String> = name.map(|n| vec![class("s", n)]).unwrap_or_default();
                if first && resolved.indent.is_some() && resolved.indent_first != Some(true) && resolved.hanging.is_none() {
                    classes.push("first".into());
                }
                let attrs = self.attrs(&classes, p.set.as_ref().map(|s| self.css.para(s)).unwrap_or_default());
                self.out.push_str(&format!("<p{}{attrs}>", Self::id_attr(p.id.as_deref())));
                let content = content_of(&p.text, &p.content);
                self.inlines(&content, assets);
                self.out.push_str("</p>\n");
            }
            Block::H(h) => {
                let level = h.level.clamp(1, 6);
                let name = h.style.clone().or_else(|| {
                    let n = format!("h{}", h.level);
                    self.doc.design.styles.contains_key(&n).then_some(n)
                });
                self.heading_no += 1;
                let anchor = h.id.clone().unwrap_or_else(|| format!("h-{}", self.heading_no));
                let content = content_of(&h.text, &h.content);
                let title = h.short.clone().unwrap_or_else(|| plain_text(&content));
                if h.outline != Some(false) {
                    self.headings.push((level, anchor.clone(), title));
                }
                let classes: Vec<String> = name.iter().map(|n| class("s", n)).collect();
                let attrs = self.attrs(&classes, h.set.as_ref().map(|s| self.css.para(s)).unwrap_or_default());
                self.out.push_str(&format!("<h{level} id=\"{}\"{attrs}>", esc(&anchor)));
                self.inlines(&content, assets);
                self.out.push_str(&format!("</h{level}>\n"));
            }
            Block::Break(x) => {
                let name = x.style.clone().or_else(|| self.doc.design.styles.contains_key("break").then(|| "break".to_string()));
                let mut classes = vec!["scene-break".to_string()];
                classes.extend(name.iter().map(|n| class("s", n)));
                let text = x.text.clone().unwrap_or_else(|| "*  *  *".into());
                self.out.push_str(&format!("<p{}{}>{}</p>\n", Self::id_attr(x.id.as_deref()), self.attrs(&classes, vec![]), esc(&text)));
            }
            Block::Figure(f) => {
                let src = self.picture(&f.src, assets);
                let caption = content_of(&f.caption, &f.caption_content);
                let alt = plain_text(&caption);
                self.out.push_str(&format!("<figure{}>", Self::id_attr(f.id.as_deref())));
                match src {
                    Some(href) => {
                        let width = f.w.and_then(|w| w.pt()).map(|w| format!(" style=\"width: {}%\"", num((w / self.measure * 100.0).min(100.0)))).unwrap_or_default();
                        self.out.push_str(&format!("<img src=\"{href}\" alt=\"{}\"{width}/>", esc(&alt)))
                    }
                    None => self.out.push_str(&format!("<p>[{}]</p>", esc(&f.src))),
                }
                if !caption.is_empty() {
                    let cs = f.caption_style.clone().or_else(|| self.doc.design.styles.contains_key("caption").then(|| "caption".to_string()));
                    let classes: Vec<String> = cs.iter().map(|n| class("s", n)).collect();
                    self.out.push_str(&format!("<figcaption{}>", self.attrs(&classes, vec![])));
                    self.inlines(&caption, assets);
                    self.out.push_str("</figcaption>");
                }
                self.out.push_str("</figure>\n");
            }
            Block::Box(x) => {
                let mut d = Vec::new();
                if let Some(Fill::Color(c)) = &x.fill {
                    d.push(format!("background-color: {}", self.css.color(&c.0)));
                }
                if let Some(s) = &x.stroke {
                    d.extend(self.border(s));
                }
                if let Some(matenadar_model::style::Radius::All(r)) = &x.radius {
                    d.push(format!("border-radius: {}pt", num(r.resolve(self.css.base, 0.0))));
                }
                if let Some(i) = &x.inset {
                    d.push(format!("padding: {}", self.insets(i)));
                }
                if let Some(w) = x.w.and_then(|w| w.pt()) {
                    d.push(format!("width: {}%", num((w / self.measure * 100.0).min(100.0))));
                }
                if x.align == Some(matenadar_model::style::Align::Center) {
                    d.push("margin-left: auto; margin-right: auto".into());
                }
                if x.breakable != Some(true) {
                    d.push("page-break-inside: avoid; break-inside: avoid".into());
                }
                self.out.push_str(&format!("<div{}{}>\n", Self::id_attr(x.id.as_deref()), self.attrs(&["box".to_string()], d)));
                let inner = x.style.clone().or(style.map(str::to_string));
                self.blocks(&x.blocks, inner.as_deref(), assets);
                self.out.push_str("</div>\n");
            }
            Block::Table(t) => self.table(t, style, assets),
            Block::List(l) => {
                let tag = if l.ordered == Some(true) { "ol" } else { "ul" };
                let mut d = Vec::new();
                let marker = l.marker.as_deref();
                if l.ordered == Some(true) {
                    let kind = match marker.map(|m| m.trim_matches(|c: char| !c.is_alphanumeric())) {
                        Some("a") => "lower-alpha",
                        Some("A") => "upper-alpha",
                        Some("i") => "lower-roman",
                        Some("I") => "upper-roman",
                        _ => "decimal",
                    };
                    d.push(format!("list-style-type: {kind}"));
                } else if let Some(m) = marker.filter(|m| *m != "•") {
                    d.push(format!("list-style-type: \"{} \"", m.replace('"', "")));
                }
                let start = l.start.map(|s| format!(" start=\"{s}\"")).unwrap_or_default();
                let name = l.style.as_deref().or(style);
                let classes: Vec<String> = name.iter().map(|n| class("s", n)).collect();
                self.out.push_str(&format!("<{tag}{}{start}{}>\n", Self::id_attr(l.id.as_deref()), self.attrs(&classes, d)));
                for item in &l.items {
                    self.out.push_str("<li>");
                    match item {
                        ListItem::Text(t) => self.inlines(&[Inline::Plain(t.clone())], assets),
                        ListItem::Blocks(bs) => self.blocks(bs, name, assets),
                    }
                    self.out.push_str("</li>\n");
                }
                self.out.push_str(&format!("</{tag}>\n"));
            }
            Block::Code(c) => {
                let name = c.style.clone().or_else(|| self.doc.design.styles.contains_key("code").then(|| "code".to_string()));
                let classes: Vec<String> = name.iter().map(|n| class("s", n)).collect();
                self.out.push_str(&format!("<pre{}{}><code>{}</code></pre>\n", Self::id_attr(c.id.as_deref()), self.attrs(&classes, vec![]), esc(&c.text)));
            }
            Block::Canvas(c) => {
                let w = match c.w {
                    Some(Length::Pt(w)) => w,
                    Some(Length::Pct(p)) => self.measure * p / 100.0,
                    _ => self.measure,
                };
                let h = match &c.h {
                    Some(Track::Length(l)) => l.pt(),
                    _ => None,
                };
                let mut elements = Vec::new();
                if c.fill.is_some() || c.stroke.is_some() {
                    elements.push(serde_json::json!({"type": "rect", "x": 0, "y": 0, "w": w, "h": h.unwrap_or(0.0), "fill": c.fill, "stroke": c.stroke, "radius": c.radius}));
                }
                let elements: Vec<serde_json::Value> = elements.into_iter().chain(c.elements.iter().map(|e| serde_json::to_value(e).unwrap_or_default())).collect();
                self.drawing(c.id.as_deref(), &elements, &c.elements, w, h, assets);
            }
            Block::Use(u) => {
                let comp = self.doc.design.components.get(&u.component).cloned();
                if let Some(Ok(x)) = comp.map(|c| c.expand(&u.component, &u.props)) {
                    if !x.blocks.is_empty() {
                        self.out.push_str(&format!("<div{}>\n", Self::id_attr(u.id.as_deref())));
                        self.blocks(&x.blocks, style, assets);
                        self.out.push_str("</div>\n");
                    } else {
                        let w = x.w.and_then(|w| w.pt()).unwrap_or(self.measure);
                        let elements: Vec<serde_json::Value> = x.elements.iter().map(|e| serde_json::to_value(e).unwrap_or_default()).collect();
                        self.drawing(u.id.as_deref(), &elements, &x.elements, w, x.h.and_then(|h| h.pt()), assets);
                    }
                }
            }
            Block::PageBreak(_) => self.out.push_str("<div class=\"pagebreak\"></div>\n"),
            Block::Space(s) => {
                if let Track::Length(l) = &s.h
                    && let Some(pt) = l.pt()
                    && pt > 0.0
                {
                    self.out.push_str(&format!("<div style=\"height: {}em\"></div>\n", num(pt / self.css.base)));
                }
            }
            Block::Columns(c) => self.blocks(&c.blocks, style, assets),
            Block::ColumnBreak(_) => {}
            Block::Margin(m) => {
                self.out.push_str(&format!("<aside class=\"margin-note\"{}>", Self::id_attr(m.id.as_deref())));
                let name = m.style.clone().or_else(|| self.doc.design.styles.contains_key("note").then(|| "note".to_string()));
                if !m.blocks.is_empty() {
                    self.blocks(&m.blocks, name.as_deref(), assets);
                } else {
                    let classes: Vec<String> = name.iter().map(|n| class("s", n)).collect();
                    self.out.push_str(&format!("<p{}>", self.attrs(&classes, vec![])));
                    self.inlines(&content_of(&m.text, &m.content), assets);
                    self.out.push_str("</p>");
                }
                self.out.push_str("</aside>\n");
            }
            Block::Toc(t) => {
                // The reader's own contents does this job; list the sections.
                self.out.push_str(&format!("<nav class=\"toc\"{}><ol>\n", Self::id_attr(t.id.as_deref())));
                for (i, item) in self.doc.content.iter().enumerate() {
                    if let Item::Section(s) = item
                        && let Some(title) = &s.title
                    {
                        self.out.push_str(&format!("<li><a href=\"{}.xhtml\">{}</a></li>\n", super::file_stem("s", &s.id, i), esc(title)));
                    }
                }
                self.out.push_str("</ol></nav>\n");
            }
            Block::Keep(k) => {
                self.out.push_str(&format!("<div style=\"page-break-inside: avoid; break-inside: avoid\"{}>\n", Self::id_attr(k.id.as_deref())));
                self.blocks(&k.blocks, style, assets);
                self.out.push_str("</div>\n");
            }
        }
    }

    fn resolved(&self, name: Option<&str>, set: Option<&Style>) -> Style {
        let s = self.doc.design.style(name.unwrap_or(""));
        match set {
            Some(over) => s.with(over),
            None => s,
        }
    }

    fn attrs(&self, classes: &[String], decls: Vec<String>) -> String {
        let mut out = String::new();
        if !classes.is_empty() {
            out.push_str(&format!(" class=\"{}\"", esc(&classes.join(" "))));
        }
        if !decls.is_empty() {
            out.push_str(&format!(" style=\"{}\"", esc(&decls.join("; "))));
        }
        out
    }

    fn border(&self, b: &Border) -> Vec<String> {
        let one = |s: &Stroke| -> String {
            match s {
                Stroke::Color(c) => format!("1pt solid {}", self.css.color(&c.0)),
                Stroke::Full(sp) => format!(
                    "{}pt {} {}",
                    num(sp.width.map(|w| w.resolve(self.css.base, 0.0)).unwrap_or(1.0)),
                    if sp.dash.is_some() { "dashed" } else { "solid" },
                    sp.color.as_ref().map(|c| self.css.color(&c.0)).unwrap_or_else(|| "currentColor".into())
                ),
            }
        };
        match b {
            Border::All(s) => vec![format!("border: {}", one(s))],
            Border::Sides(sd) => [("top", &sd.top), ("right", &sd.right), ("bottom", &sd.bottom), ("left", &sd.left)]
                .into_iter()
                .filter_map(|(n, s)| s.as_ref().map(|s| format!("border-{n}: {}", one(s))))
                .collect(),
        }
    }

    fn insets(&self, i: &Insets) -> String {
        let l = |v: Option<Length>| v.map(|v| format!("{}em", num(v.resolve(self.css.base, 0.0) / self.css.base))).unwrap_or_else(|| "0".into());
        match i {
            Insets::All(v) => l(Some(*v)),
            Insets::Sides(s) => format!("{} {} {} {}", l(s.top.or(s.y)), l(s.right.or(s.x)), l(s.bottom.or(s.y)), l(s.left.or(s.x))),
        }
    }

    fn table(&mut self, t: &matenadar_model::block::Table, style: Option<&str>, assets: &mut Assets) {
        let name = t.style.as_deref().or(style).map(str::to_string);
        let hname = t.header_style.clone().or(name.clone());
        let r = t.rules.clone().unwrap_or_default();
        let [top_rule, header_rule, rows_rule, bottom_rule] = [&r.top, &r.header, &r.rows, &r.bottom]
            .map(|s| s.as_ref().map(|s| self.border(&Border::All(s.clone())).remove(0).replacen("border:", "", 1).trim().to_string()));
        let mut tstyle = Vec::new();
        if let Some(top) = &top_rule {
            tstyle.push(format!("border-top: {top}"));
        }
        if let Some(bottom) = &bottom_rule {
            tstyle.push(format!("border-bottom: {bottom}"));
        }
        self.out.push_str(&format!("<table{}{}>\n", Self::id_attr(t.id.as_deref()), self.attrs(&[], tstyle)));
        let align = |i: usize| t.align.get(i).map(|a| match a {
            matenadar_model::style::Align::Right => "right",
            matenadar_model::style::Align::Center => "center",
            _ => "left",
        });
        let rows: Vec<(bool, &Vec<Cell>)> = t.header.iter().map(|r| (true, r)).chain(t.rows.iter().map(|r| (false, r))).collect();
        let hn = t.header.len();
        let mut in_head = false;
        for (ri, (head, row)) in rows.iter().enumerate() {
            if *head && !in_head {
                self.out.push_str("<thead>\n");
                in_head = true;
            }
            if !*head && (ri == hn) {
                if in_head {
                    self.out.push_str("</thead>\n");
                }
                self.out.push_str("<tbody>\n");
            }
            let mut rstyle = Vec::new();
            if ri == hn && hn > 0
                && let Some(h) = &header_rule
            {
                rstyle.push(format!("border-top: {h}"));
            } else if ri > hn
                && let Some(h) = &rows_rule
            {
                rstyle.push(format!("border-top: {h}"));
            }
            self.out.push_str(&format!("<tr{}>", self.attrs(&[], rstyle)));
            let tag = if *head { "th" } else { "td" };
            let st = if *head { hname.clone() } else { name.clone() };
            let mut col = 0;
            for cell in row.iter() {
                let mut d: Vec<String> = align(col).map(|a| vec![format!("text-align: {a}")]).unwrap_or_default();
                match cell {
                    Cell::Text(text) => {
                        self.out.push_str(&format!("<{tag}{}>", self.attrs(&[], d)));
                        let classes: Vec<String> = st.iter().map(|n| class("s", n)).collect();
                        self.out.push_str(&format!("<p{}>", self.attrs(&classes, vec![])));
                        self.inlines(&[Inline::Plain(text.clone())], assets);
                        self.out.push_str("</p>");
                        col += 1;
                    }
                    Cell::Full(c) => {
                        if let Some(Fill::Color(f)) = &c.fill {
                            d.push(format!("background-color: {}", self.css.color(&f.0)));
                        }
                        if let Some(a) = c.align {
                            d.retain(|x| !x.starts_with("text-align"));
                            d.push(format!("text-align: {}", match a {
                                matenadar_model::style::Align::Right => "right",
                                matenadar_model::style::Align::Center => "center",
                                _ => "left",
                            }));
                        }
                        let span = |n: Option<u32>, attr: &str| n.filter(|n| *n > 1).map(|n| format!(" {attr}=\"{n}\"")).unwrap_or_default();
                        self.out.push_str(&format!("<{tag}{}{}{}>", span(c.colspan, "colspan"), span(c.rowspan, "rowspan"), self.attrs(&[], d)));
                        let cname = c.style.clone().or(st.clone());
                        if !c.blocks.is_empty() {
                            self.blocks(&c.blocks, cname.as_deref(), assets);
                        } else {
                            let classes: Vec<String> = cname.iter().map(|n| class("s", n)).collect();
                            let set = c.set.as_ref().map(|s| self.css.para(s)).unwrap_or_default();
                            self.out.push_str(&format!("<p{}>", self.attrs(&classes, set)));
                            self.inlines(&content_of(&c.text, &c.content), assets);
                            self.out.push_str("</p>");
                        }
                        col += c.colspan.unwrap_or(1).max(1) as usize;
                    }
                }
                self.out.push_str(&format!("</{tag}>"));
            }
            self.out.push_str("</tr>\n");
        }
        if in_head && t.rows.is_empty() {
            self.out.push_str("</thead>\n");
        } else if !t.rows.is_empty() {
            self.out.push_str("</tbody>\n");
        }
        self.out.push_str("</table>\n");
    }

    pub fn inlines(&mut self, content: &[Inline], assets: &mut Assets) {
        for x in content {
            match x {
                Inline::Plain(s) => self.text(s),
                Inline::Run(r) => self.run(r),
                Inline::Leader(l) => {
                    if l.leader == "rule" {
                        self.out.push_str("<span class=\"leader\" style=\"border-bottom: 0.5pt solid currentColor; min-width: 3em\"> </span>");
                    } else {
                        self.out.push_str("<span class=\"leader\"> </span>");
                    }
                }
                Inline::Space(sp) => {
                    let em = sp.space.resolve(self.css.base, 0.0) / self.css.base;
                    self.out.push_str(&format!("<span style=\"display: inline-block; width: {}em\"></span>", num(em)));
                }
                Inline::Note(n) => {
                    self.note_no += 1;
                    let k = self.note_no;
                    self.out.push_str(&format!("<a epub:type=\"noteref\" href=\"#fn-{k}\" id=\"fnref-{k}\"><sup>{k}</sup></a>"));
                    let saved = std::mem::take(&mut self.out);
                    self.inlines(&n.note, assets);
                    let note = std::mem::replace(&mut self.out, saved);
                    self.notes.push(format!("<aside epub:type=\"footnote\" id=\"fn-{k}\"><p><a href=\"#fnref-{k}\">{k}</a> {note}</p></aside>\n"));
                }
                Inline::Field(f) => {
                    let value = match f.field.as_str() {
                        "doc.title" => self.doc.meta.title.clone(),
                        "doc.author" => self.doc.meta.author.clone().unwrap_or_default(),
                        "section.title" => self.section.and_then(|s| s.title.clone()).unwrap_or_default(),
                        other => match other.strip_prefix("section.") {
                            Some(var) => self.section.and_then(|s| s.vars.get(var).cloned()).unwrap_or_default(),
                            // Page numbers mean nothing once the text reflows.
                            None => String::new(),
                        },
                    };
                    self.text(&value);
                }
                Inline::Ref(r) => {
                    let text = self.titles.get(&r.target).cloned().unwrap_or_else(|| "↗".into());
                    match self.files.get(&r.target) {
                        Some(file) => self.out.push_str(&format!("<a href=\"{file}#{}\">{}</a>", esc(&r.target), esc(&text))),
                        None => self.text(&text),
                    }
                }
                Inline::Break(_) => self.out.push_str("<br/>"),
                Inline::Image(img) => {
                    let href = if img.image.is_empty() {
                        img.markup.as_ref().map(|m| assets.add(&format!("markup:{m}"), "images/inline", "svg", m.clone().into_bytes()))
                    } else {
                        self.picture(&img.image, assets)
                    };
                    if let Some(href) = href {
                        let h = img.h.map(|h| format!("height: {}em; ", num(h.resolve(self.css.base, 0.0) / self.css.base))).unwrap_or_default();
                        let b = img.baseline.map(|b| format!("vertical-align: -{}em", num(b.resolve(self.css.base, 0.0) / self.css.base))).unwrap_or_default();
                        self.out.push_str(&format!("<img src=\"{href}\" alt=\"\" style=\"{h}{b}\"/>"));
                    }
                }
            }
        }
    }

    fn text(&mut self, s: &str) {
        for (i, line) in s.split('\n').enumerate() {
            if i > 0 {
                self.out.push_str("<br/>");
            }
            self.out.push_str(&esc(line));
        }
    }

    fn run(&mut self, r: &Run) {
        let mut close = Vec::new();
        let mut classes = Vec::new();
        if let Some(sp) = &r.span {
            classes.push(class("sp", sp));
        }
        let size = r.set.size.map(|s| s.resolve(self.css.base, 0.0)).unwrap_or(self.css.base);
        let mut decls = self.css.text(&r.set, size);
        if r.nobreak {
            decls.push("white-space: nowrap".into());
        }
        if let Some(m) = &r.mark {
            let color = m.stroke.as_ref().map(|s| match s {
                Stroke::Color(c) => self.css.color(&c.0),
                Stroke::Full(sp) => sp.color.as_ref().map(|c| self.css.color(&c.0)).unwrap_or_else(|| "currentColor".into()),
            });
            match m.kind.as_str() {
                "underline" => decls.push(format!("text-decoration: underline {}", color.unwrap_or_default())),
                "strike" => decls.push(format!("text-decoration: line-through {}", color.unwrap_or_default())),
                "highlight" => {
                    let fill = match &m.fill {
                        Some(Fill::Color(c)) => self.css.color(&c.0),
                        _ => "#fff3a8".into(),
                    };
                    decls.push(format!("background-color: {fill}"));
                }
                kind => {
                    decls.push(format!("border: 0.8pt solid {}", color.unwrap_or_else(|| "currentColor".into())));
                    if kind == "ring" {
                        decls.push("border-radius: 50%".into());
                    }
                    decls.push("padding: 0 0.15em".into());
                }
            }
        }
        if !classes.is_empty() || !decls.is_empty() {
            self.out.push_str(&format!("<span{}>", self.attrs(&classes, decls)));
            close.push("</span>");
        }
        for (on, open, shut) in [(r.em, "<em>", "</em>"), (r.strong, "<strong>", "</strong>"), (r.sup, "<sup>", "</sup>"), (r.sub, "<sub>", "</sub>")] {
            if on {
                self.out.push_str(open);
                close.push(shut);
            }
        }
        if let Some(url) = &r.link {
            self.out.push_str(&format!("<a href=\"{}\">", esc(url)));
            close.push("</a>");
        }
        self.text(&r.text);
        for c in close.iter().rev() {
            self.out.push_str(c);
        }
    }

    /// A picture from the document's files, added to the book.
    fn picture(&self, src: &str, assets: &mut Assets) -> Option<String> {
        let rel = src.trim_start_matches('/');
        if rel.contains("..") {
            return None;
        }
        let path = self.paths.root.join(rel);
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        let data = std::fs::read(&path).ok()?;
        match ext.as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "svg" | "webp" => Some(assets.add(&format!("file:{}", path.display()), "images/img", &ext, data)),
            // Other formats as PNG.
            _ => {
                let img = image::load_from_memory(&data).ok()?;
                let mut out = Vec::new();
                img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).ok()?;
                Some(assets.add(&format!("file:{}", path.display()), "images/img", "png", out))
            }
        }
    }

    /// Placed elements drawn as one picture: laid out alone, on a page their size.
    fn drawing(&mut self, id: Option<&str>, elements: &[serde_json::Value], originals: &[Element], w: f64, h: Option<f64>, assets: &mut Assets) {
        let h = h.unwrap_or_else(|| {
            originals
                .iter()
                .filter_map(|e| e.place().map(|p| p.y.and_then(|y| y.pt()).unwrap_or(0.0) + p.h.and_then(|h| h.pt()).unwrap_or(0.0)))
                .fold(0.0, f64::max)
        });
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let alt: Vec<String> = originals
            .iter()
            .filter_map(|e| match e {
                Element::Text(t) => Some(t.text.clone().unwrap_or_else(|| plain_text(&t.content))),
                _ => None,
            })
            .filter(|t| !t.trim().is_empty())
            .collect();
        let mut design = serde_json::to_value(&self.doc.design).unwrap_or_default();
        design["masters"]["mt-drawing"] = serde_json::json!({"width": w, "height": h, "margins": {"top": 0, "bottom": 0, "inside": 0, "outside": 0}});
        let mini = serde_json::json!({
            "matenadar": "1",
            "meta": {"title": "drawing"},
            "fonts": self.doc.fonts,
            "design": design,
            "content": [{"type": "page", "id": "mt-drawing", "master": "mt-drawing", "elements": elements}]
        });
        let Ok(doc) = Document::from_json(&mini.to_string()) else { return };
        let Ok(s) = Session::with_fonts(doc, self.paths.clone(), self.fonts.clone()) else { return };
        let Some(png) = s.png(1, self.dpi, false) else { return };
        // Line art stays PNG; anything large becomes JPEG.
        let (data, ext) = if png.len() > 400_000 { (jpeg(&png).map(|j| j.0).unwrap_or(png), "jpg") } else { (png, "png") };
        let key = format!("drawing:{}:{}", id.unwrap_or(""), self.out.len());
        let href = assets.add(&key, "images/drawing", ext, data);
        let width = num((w / self.measure * 100.0).min(100.0));
        self.out.push_str(&format!(
            "<div class=\"drawing\"{}><img src=\"{href}\" alt=\"{}\" style=\"width: {width}%\"/></div>\n",
            Self::id_attr(id),
            esc(&alt.join(" "))
        ));
        let _ = self.session;
    }
}
