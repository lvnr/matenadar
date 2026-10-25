//! The design's styles as CSS for e-readers. Sizes are relative (`rem` for
//! a style's size, `em` within it), so readers can still scale the text.

use matenadar_model::style::{Align, Case, LineHeight, Style, TextStyle};
use matenadar_model::units::{Length, num};
use matenadar_model::Document;
use matenadar_typst::generate::typ::Palette;

/// A class name for a style or span name.
pub fn class(prefix: &str, name: &str) -> String {
    let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    format!("{prefix}-{safe}")
}

pub struct Css<'a> {
    pub doc: &'a Document,
    pub pal: Palette<'a>,
    /// The base size in points, which readers call 1rem.
    pub base: f64,
}

impl<'a> Css<'a> {
    pub fn new(doc: &'a Document) -> Self {
        let base = doc.design.style("").text.size.and_then(|s| s.pt()).unwrap_or(11.0);
        Css { doc, pal: Palette { names: &doc.design.colors }, base }
    }

    /// A length in a style of `size` points, as CSS.
    fn len(&self, l: Length, size: f64) -> String {
        match l {
            Length::Pt(v) => format!("{}em", num(v / size)),
            Length::Em(v) => format!("{}em", num(v)),
            Length::Pct(v) => format!("{}%", num(v)),
        }
    }

    fn families(&self, t: &TextStyle) -> Option<String> {
        let stack = t.font.as_ref()?;
        let mut out = Vec::new();
        for f in stack.families() {
            match self.doc.design.fonts.get(f) {
                Some(named) => out.extend(named.families().into_iter().map(str::to_string)),
                None => out.push(f.to_string()),
            }
        }
        let generic = if out.iter().any(|f| f.to_lowercase().contains("mono") || f.to_lowercase().contains("courier")) {
            "monospace"
        } else if out.iter().any(|f| f.to_lowercase().contains("sans") || f.to_lowercase().contains("gothic")) {
            "sans-serif"
        } else {
            "serif"
        };
        let mut list: Vec<String> = out.iter().map(|f| format!("\"{}\"", f.replace('"', ""))).collect();
        list.push(generic.into());
        Some(list.join(", "))
    }

    /// Character settings as CSS declarations. `size` is the text's size in
    /// points; `rem` sizes are written against the base.
    pub fn text(&self, t: &TextStyle, size: f64) -> Vec<String> {
        let mut d = Vec::new();
        if let Some(f) = self.families(t) {
            d.push(format!("font-family: {f}"));
        }
        if let Some(s) = t.size {
            d.push(match s {
                Length::Pt(v) => format!("font-size: {}rem", num(v / self.base)),
                Length::Em(v) => format!("font-size: {}em", num(v)),
                Length::Pct(v) => format!("font-size: {}%", num(v)),
            });
        }
        if let Some(w) = &t.weight {
            d.push(format!("font-weight: {}", matenadar_typst::glyphs::weight_number(w)));
        }
        if let Some(i) = t.italic {
            d.push(format!("font-style: {}", if i { "italic" } else { "normal" }));
        }
        if let Some(st) = t.stretch {
            d.push(format!("font-stretch: {}%", num(st)));
        }
        if let Some(tr) = t.tracking {
            d.push(format!("letter-spacing: {}", self.len(tr, size)));
        }
        if let Some(ws) = t.word_spacing {
            d.push(format!("word-spacing: {}", self.len(ws, size)));
        }
        if let Some(c) = &t.color {
            d.push(format!("color: {}", self.color(&c.0)));
        }
        match t.case {
            Some(Case::Upper) => d.push("text-transform: uppercase".into()),
            Some(Case::Lower) => d.push("text-transform: lowercase".into()),
            Some(Case::SmallCaps) => d.push("font-variant-caps: small-caps".into()),
            Some(Case::AllSmallCaps) => d.push("font-variant-caps: all-small-caps".into()),
            _ => {}
        }
        if let Some(f) = &t.features
            && !f.is_empty()
        {
            let feats: Vec<String> = f
                .iter()
                .map(|f| match f.strip_prefix('-') {
                    Some(off) => format!("\"{off}\" 0"),
                    None => match f.split_once('=') {
                        Some((tag, v)) => format!("\"{tag}\" {v}"),
                        None => format!("\"{f}\" 1"),
                    },
                })
                .collect();
            d.push(format!("font-feature-settings: {}", feats.join(", ")));
        }
        let mut deco = Vec::new();
        if t.underline == Some(true) {
            deco.push("underline");
        }
        if t.strike == Some(true) {
            deco.push("line-through");
        }
        if !deco.is_empty() {
            d.push(format!("text-decoration: {}", deco.join(" ")));
        }
        if let Some(h) = t.hyphenate {
            let v = if h { "auto" } else { "manual" };
            d.push(format!("hyphens: {v}"));
            d.push(format!("-webkit-hyphens: {v}"));
            d.push(format!("-epub-hyphens: {v}"));
        }
        if t.ligatures == Some(false) {
            d.push("font-variant-ligatures: none".into());
        }
        if let Some(lh) = t.line_height {
            match lh {
                LineHeight::Factor(f) => d.push(format!("line-height: {}", num(f))),
                LineHeight::Length(l) => d.push(format!("line-height: {}", num(l.resolve(size, size * 100.0) / size))),
            }
        }
        d
    }

    /// Paragraph settings (and its text) as CSS declarations.
    pub fn para(&self, s: &Style) -> Vec<String> {
        let size = s.text.size.map(|l| l.resolve(self.base, 0.0)).unwrap_or(self.base);
        let mut d = self.text(&s.text, size);
        let align = match s.align {
            Some(Align::Justify) => Some("justify"),
            Some(Align::Center) => Some("center"),
            Some(Align::Right) => Some("right"),
            Some(Align::Left | Align::Inside | Align::Outside) => Some("left"),
            None => None,
        };
        if let Some(a) = align {
            d.push(format!("text-align: {a}"));
        }
        match (s.indent, s.hanging) {
            (_, Some(h)) => {
                d.push(format!("padding-left: {}", self.len(h, size)));
                d.push(format!("text-indent: -{}", self.len(h, size)));
            }
            (Some(i), None) => d.push(format!("text-indent: {}", self.len(i, size))),
            _ => {}
        }
        if let Some(m) = s.margin_left {
            d.push(format!("margin-left: {}", self.len(m, size)));
        }
        if let Some(m) = s.margin_right {
            d.push(format!("margin-right: {}", self.len(m, size)));
        }
        if let Some(l) = s.space_before {
            d.push(format!("margin-top: {}", self.len(l, size)));
        }
        if let Some(l) = s.space_after {
            d.push(format!("margin-bottom: {}", self.len(l, size)));
        }
        if s.keep_with_next == Some(true) {
            d.push("page-break-after: avoid".into());
            d.push("break-after: avoid".into());
        }
        if s.keep_together == Some(true) {
            d.push("page-break-inside: avoid".into());
            d.push("break-inside: avoid".into());
        }
        if s.page_break_before == Some(true) {
            d.push("page-break-before: always".into());
            d.push("break-before: page".into());
        }
        for (side, rule) in [("top", &s.rule_above), ("bottom", &s.rule_below)] {
            if let Some(r) = rule {
                let (w, c) = match &r.stroke {
                    Some(matenadar_model::style::Stroke::Full(sp)) => (
                        sp.width.map(|w| w.resolve(size, 0.0)).unwrap_or(0.5),
                        sp.color.as_ref().map(|c| self.color(&c.0)).unwrap_or_else(|| "currentColor".into()),
                    ),
                    Some(matenadar_model::style::Stroke::Color(c)) => (0.5, self.color(&c.0)),
                    None => (0.5, "currentColor".into()),
                };
                d.push(format!("border-{side}: {}pt solid {c}", num(w)));
                d.push(format!("padding-{side}: {}", r.gap.map(|g| self.len(g, size)).unwrap_or_else(|| "0.3em".into())));
            }
        }
        if let Some(matenadar_model::style::Fill::Color(c)) = &s.fill {
            d.push(format!("background-color: {}", self.color(&c.0)));
        }
        if let Some(matenadar_model::style::Insets::All(i)) = &s.inset {
            d.push(format!("padding: {}", self.len(*i, size)));
        }
        if s.balance == Some(true) {
            d.push("text-wrap: balance".into());
        } else if s.wrap.as_deref() == Some("pretty") {
            d.push("text-wrap: pretty".into());
        }
        d.push("orphans: 2".into());
        d.push("widows: 2".into());
        d
    }

    pub fn color(&self, c: &str) -> String {
        self.pal.svg(&matenadar_model::Color(c.to_string()))
    }

    /// The stylesheet: the base on `body`, every style and span as a class.
    pub fn sheet(&self, extra: &str) -> String {
        let d = &self.doc.design;
        let mut out = String::new();
        let base = d.style("");
        let mut body = self.para(&base);
        body.retain(|p| !p.starts_with("font-size") && !p.starts_with("margin"));
        out.push_str(&format!("body {{ {}; margin: 0 }}\n", body.join("; ")));
        for name in d.styles.keys() {
            let st = d.style(name);
            out.push_str(&format!(".{} {{ {} }}\n", class("s", name), self.para(&st).join("; ")));
        }
        for (name, sp) in &d.spans {
            let size = sp.size.map(|l| l.resolve(self.base, 0.0)).unwrap_or(self.base);
            out.push_str(&format!(".{} {{ {} }}\n", class("sp", name), self.text(sp, size).join("; ")));
        }
        out.push_str(BASICS);
        out.push_str(extra);
        out
    }
}

const BASICS: &str = r#"
p, h1, h2, h3, h4, h5, h6 { margin: 0 }
p.first, p.noindent { text-indent: 0 }
.scene-break { text-align: center; margin: 1em 0 }
figure { margin: 1em 0; text-align: center; page-break-inside: avoid; break-inside: avoid }
figure img, .drawing img { max-width: 100%; height: auto }
.drawing { margin: 0.8em 0; text-align: center; page-break-inside: avoid; break-inside: avoid }
.page img { width: 100%; height: auto }
.box { margin: 0.8em 0 }
table { border-collapse: collapse; margin: 0.8em 0; width: 100% }
td, th { vertical-align: top; padding: 0.2em 0.4em; text-align: left }
pre { white-space: pre-wrap; font-family: monospace; margin: 0.8em 0 }
aside.margin-note { margin: 0.5em 0 0.5em 1.5em; font-size: 0.85em }
aside[epub|type~="footnote"] { font-size: 0.85em }
.leader { display: inline-block; min-width: 1em }
.pagebreak { page-break-before: always; break-before: page }
nav ol { list-style: none; padding-left: 1em }
"#;
