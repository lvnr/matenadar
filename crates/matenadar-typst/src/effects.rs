//! What happens to a laid-out page before it is drawn: elements faded by
//! their `opacity`, and the text of `decorative` elements turned into glyph
//! outlines. The generator wraps such elements in a box labelled
//! `mt-fx-<n>`; here those groups are found and their contents changed.
//!
//! Outlined text is drawn exactly as before, but a PDF reader (or KDP's
//! previewer) no longer sees text there: display type may run into the
//! margins and the bleed without failing a print check.

use typst::model::Document;
use typst::layout::{Abs, Frame, FrameItem, Point};
use typst::text::TextItem;
use typst::visualize::{Curve, FillRule, FixedStroke, Geometry, Paint, Shape};
use typst_layout::PagedDocument;

/// One labelled element's effects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    /// Multiplies the alpha of every paint inside.
    pub opacity: f32,
    /// Text inside becomes outlines.
    pub outline: bool,
}

/// The document with `effects` applied (unchanged when there are none).
pub fn apply(doc: PagedDocument, effects: &[Effect]) -> PagedDocument {
    if effects.is_empty() {
        return doc;
    }
    let mut pages = doc.pages().to_vec();
    let mut changed = false;
    for page in &mut pages {
        changed |= visit(&mut page.frame, effects, Effect { opacity: 1.0, outline: false });
    }
    if !changed {
        return doc;
    }
    PagedDocument::new(pages.into_iter().collect(), doc.info().clone())
}

/// The effect index of a group's label, if it is one of ours.
fn index_of(frame_label: Option<typst::foundations::Label>) -> Option<usize> {
    let label = frame_label?;
    label.resolve().as_str().strip_prefix("mt-fx-")?.parse().ok()
}

fn visit(frame: &mut Frame, effects: &[Effect], active: Effect) -> bool {
    let mut changed = false;
    frame.retain(|item| {
        match item {
            FrameItem::Group(g) => {
                let mut a = active;
                if let Some(e) = index_of(g.label).and_then(|i| effects.get(i)) {
                    a.opacity *= e.opacity;
                    a.outline |= e.outline;
                }
                changed |= visit(&mut g.frame, effects, a);
            }
            FrameItem::Text(text) if active.outline => {
                let span = text.glyphs.first().map(|g| g.span.0).unwrap_or(typst::syntax::Span::detached());
                let mut shape = outlines(text);
                fade_shape(&mut shape, active.opacity);
                *item = FrameItem::Shape(shape, span);
                changed = true;
            }
            FrameItem::Text(text) if active.opacity < 1.0 => {
                text.fill = fade(&text.fill, active.opacity);
                if let Some(s) = &mut text.stroke {
                    s.paint = fade(&s.paint, active.opacity);
                }
                changed = true;
            }
            FrameItem::Shape(shape, _) if active.opacity < 1.0 => {
                fade_shape(shape, active.opacity);
                changed = true;
            }
            _ => {}
        }
        true
    });
    changed
}

fn fade_shape(shape: &mut Shape, opacity: f32) {
    if opacity >= 1.0 {
        return;
    }
    if let Some(f) = &mut shape.fill {
        *f = fade(f, opacity);
    }
    if let Some(s) = &mut shape.stroke {
        s.paint = fade(&s.paint, opacity);
    }
}

/// A solid paint with its alpha multiplied (gradients and tilings are kept).
fn fade(paint: &Paint, opacity: f32) -> Paint {
    match paint {
        Paint::Solid(c) => Paint::Solid(c.clone().with_alpha(c.alpha().unwrap_or(1.0) * opacity)),
        other => other.clone(),
    }
}

/// A run of text as one shape: its glyphs' outlines, in the run's fill and
/// stroke, placed from the run's baseline origin as the text was.
fn outlines(text: &TextItem) -> Shape {
    let scale = text.size.to_pt() / text.font.units_per_em();
    let face = text.font.ttf();
    let mut curve = Curve::new();
    let mut x = Abs::zero();
    for g in &text.glyphs {
        let ox = (x + g.x_offset.at(text.size)).to_pt();
        let oy = -g.y_offset.at(text.size).to_pt();
        let mut pen = Pen { curve: &mut curve, ox, oy, s: scale, last: (0.0, 0.0) };
        face.outline_glyph(ttf_parser::GlyphId(g.id), &mut pen);
        x += g.x_advance.at(text.size);
    }
    Shape {
        geometry: Geometry::Curve(curve),
        fill: Some(text.fill.clone()),
        fill_rule: FillRule::NonZero,
        stroke: text.stroke.clone().map(|s: FixedStroke| s),
    }
}

struct Pen<'a> {
    curve: &'a mut Curve,
    ox: f64,
    oy: f64,
    s: f64,
    /// The last point, in font units, for turning quadratics into cubics.
    last: (f32, f32),
}

impl Pen<'_> {
    fn p(&self, x: f32, y: f32) -> Point {
        Point::new(Abs::pt(self.ox + x as f64 * self.s), Abs::pt(self.oy - y as f64 * self.s))
    }
}

impl ttf_parser::OutlineBuilder for Pen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.p(x, y);
        self.curve.move_(p);
        self.last = (x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.p(x, y);
        self.curve.line(p);
        self.last = (x, y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x0, y0) = self.last;
        let c1 = self.p(x0 + 2.0 / 3.0 * (x1 - x0), y0 + 2.0 / 3.0 * (y1 - y0));
        let c2 = self.p(x + 2.0 / 3.0 * (x1 - x), y + 2.0 / 3.0 * (y1 - y));
        let p = self.p(x, y);
        self.curve.cubic(c1, c2, p);
        self.last = (x, y);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (c1, c2, p) = (self.p(x1, y1), self.p(x2, y2), self.p(x, y));
        self.curve.cubic(c1, c2, p);
        self.last = (x, y);
    }
    fn close(&mut self) {
        self.curve.close();
    }
}
