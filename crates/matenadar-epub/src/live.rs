//! Fixed-layout pages with live text. The page is drawn without its typeset
//! words, as a picture, and the words are set over it as SVG text in the
//! book's own faces, run by run and word by word where the engine set them:
//! readers can select, search and read them aloud, and they stay sharp at
//! any zoom. Words drawn as outlines (lettering, decorative type) and runs
//! in faces the book can't carry stay in the picture.

use std::collections::HashMap;

use matenadar_typst::Session;
use typst::layout::{Frame, FrameItem, Transform};
use typst::text::TextItem;
use typst::visualize::Paint;

use crate::Assets;
use crate::xhtml::esc;

/// The faces live text uses, each embedded once under a name of its own.
#[derive(Default)]
pub(crate) struct Faces {
    /// Font data address to its CSS name.
    names: HashMap<usize, String>,
    pub css: String,
}

/// Whether a run can be live: its face is a whole font file (not one of a
/// collection, not a variable instance) that allows embedding.
pub(crate) fn live(t: &TextItem) -> bool {
    let font = t.font.font();
    font.index() == 0
        && t.font.variations().0.is_empty()
        && !font.info().flags.contains(typst::text::FontFlags::VARIABLE)
        && !matches!(t.font.ttf().permissions(), Some(ttf_parser::Permissions::Restricted))
        && t.glyphs.iter().all(|g| g.id != 0)
}

impl Faces {
    fn name(&mut self, t: &TextItem, assets: &mut Assets) -> String {
        let font = t.font.font();
        let data = font.data();
        let key = data.as_slice().as_ptr() as usize;
        if let Some(n) = self.names.get(&key) {
            return n.clone();
        }
        let name = format!("mt-face-{}", self.names.len() + 1);
        let ext = if data.as_slice().starts_with(b"OTTO") { "otf" } else { "ttf" };
        let href = assets.add(&format!("font:{key}"), "fonts/font", ext, data.as_slice().to_vec());
        self.css.push_str(&format!("@font-face {{ font-family: \"{name}\"; src: url(\"{href}\"); }}\n"));
        self.names.insert(key, name.clone());
        name
    }
}

/// The page's body: its picture without the live words, and the words over
/// it. `background` is the picture's href and its size in pixels.
pub(crate) fn page(session: &Session, page: usize, background: &str, px: (u32, u32), alt: &str, faces: &mut Faces, assets: &mut Assets) -> String {
    let Some(p) = session.layout().pages().get(page - 1) else { return String::new() };
    let size = p.frame.size();
    let (w, h) = (size.x.to_pt(), size.y.to_pt());
    let mut text = String::new();
    walk(&p.frame, Transform::identity(), &mut |t, ts| run(t, ts, faces, assets, &mut text));
    let title = if alt.is_empty() { String::new() } else { format!("<title>{}</title>", esc(alt)) };
    format!(
        "<div class=\"page\"><svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" version=\"1.1\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"><image width=\"{}\" height=\"{}\" xlink:href=\"{background}\">{title}</image>{text}</svg></div>",
        px.0,
        px.1,
        num(w),
        num(h),
        num(w),
        num(h)
    )
}

fn walk(frame: &Frame, ts: Transform, f: &mut dyn FnMut(&TextItem, Transform)) {
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Group(g) => walk(&g.frame, ts.pre_concat(Transform::translate(pos.x, pos.y)).pre_concat(g.transform), f),
            FrameItem::Text(t) if live(t) => f(t, ts.pre_concat(Transform::translate(pos.x, pos.y))),
            _ => {}
        }
    }
}

/// One run as an SVG text element: a span per word at its own place, held
/// to the width the engine gave it (tracking and justification included).
fn run(t: &TextItem, ts: Transform, faces: &mut Faces, assets: &mut Assets, out: &mut String) {
    let size = t.size.to_pt();
    let (fill, opacity) = paint(&t.fill);
    let mut words: Vec<(f64, f64, f64, String)> = Vec::new();
    let mut x = 0.0;
    let mut word: Option<(f64, f64, f64, usize, usize)> = None;
    let flush = |w: &mut Option<(f64, f64, f64, usize, usize)>, words: &mut Vec<(f64, f64, f64, String)>| {
        if let Some((x0, y0, width, a, b)) = w.take()
            && let Some(s) = t.text.get(a..b)
            && !s.trim().is_empty()
        {
            words.push((x0, y0, width, s.to_string()));
        }
    };
    for g in &t.glyphs {
        let (a, b) = (g.range.start as usize, g.range.end as usize);
        let advance = g.x_advance.at(t.size).to_pt();
        if t.text.get(a..b).is_some_and(|s| s.chars().all(char::is_whitespace)) {
            flush(&mut word, &mut words);
        } else {
            let w = word.get_or_insert((x + g.x_offset.at(t.size).to_pt(), -g.y_offset.at(t.size).to_pt(), 0.0, a, b));
            w.3 = w.3.min(a);
            w.4 = w.4.max(b);
            w.2 = x + advance - w.0;
        }
        x += advance;
    }
    flush(&mut word, &mut words);
    if words.is_empty() {
        return;
    }
    let face = faces.name(t, assets);
    out.push_str(&format!(
        "<text transform=\"matrix({} {} {} {} {} {})\" font-family=\"{face}\" font-size=\"{}\" fill=\"{fill}\"{}>",
        num(ts.sx.get()),
        num(ts.ky.get()),
        num(ts.kx.get()),
        num(ts.sy.get()),
        num(ts.tx.to_pt()),
        num(ts.ty.to_pt()),
        num(size),
        if opacity < 1.0 { format!(" fill-opacity=\"{}\"", num(opacity)) } else { String::new() }
    ));
    for (x0, y0, width, s) in words {
        let fit = if s.chars().count() > 1 { format!(" textLength=\"{}\" lengthAdjust=\"spacing\"", num(width)) } else { String::new() };
        out.push_str(&format!("<tspan x=\"{}\" y=\"{}\"{fit}>{}</tspan>", num(x0), num(y0), esc(&s)));
    }
    out.push_str("</text>");
}

/// A paint as an SVG colour and opacity; gradients and tilings give their
/// first colour.
fn paint(p: &Paint) -> (String, f64) {
    let c = match p {
        Paint::Solid(c) => c.clone(),
        Paint::Gradient(g) => g.stops_ref().first().map(|s| s.0.clone()).unwrap_or(typst::visualize::Color::BLACK),
        Paint::Tiling(_) => typst::visualize::Color::BLACK,
    };
    let [r, g, b, a] = c.to_vec4_u8();
    (format!("#{r:02x}{g:02x}{b:02x}"), a as f64 / 255.0)
}

fn num(v: f64) -> String {
    matenadar_model::units::num(v)
}
