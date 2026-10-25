//! What landed where: every glyph, image and shape on every page, traced
//! back to the document node it came from.

use serde::Serialize;
use typst::layout::{Frame, FrameItem, Point, Size, Transform};
use typst::syntax::Span;

use crate::session::Session;

/// A rectangle in points from the trim's top-left corner.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    fn union(self, o: Rect) -> Rect {
        Rect { x0: self.x0.min(o.x0), y0: self.y0.min(o.y0), x1: self.x1.max(o.x1), y1: self.y1.max(o.y1) }
    }

    fn of(points: &[Point]) -> Rect {
        let xs = points.iter().map(|p| p.x.to_pt());
        let ys = points.iter().map(|p| p.y.to_pt());
        Rect {
            x0: xs.clone().fold(f64::INFINITY, f64::min),
            x1: xs.fold(f64::NEG_INFINITY, f64::max),
            y0: ys.clone().fold(f64::INFINITY, f64::min),
            y1: ys.fold(f64::NEG_INFINITY, f64::max),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Kind {
    /// A glyph run: its text, font family and size, and whether a glyph is
    /// missing from the font (`.notdef`).
    Text { text: String, family: String, size: f64, missing: bool, baseline: f64 },
    /// An image: pixel size (none for vector images) and placed size in points.
    Image { px: Option<(u32, u32)>, w: f64, h: f64 },
    Shape,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub page: usize,
    pub rect: Rect,
    pub kind: Kind,
    pub span: Span,
    /// The innermost document node that made it, and that node's kind.
    pub node: Option<(String, &'static str)>,
    /// The innermost placed element it belongs to, and that element's kind.
    pub element: Option<(String, &'static str)>,
    /// Every node it belongs to, innermost first.
    pub ancestors: Vec<(String, &'static str)>,
}

/// Every item on every page.
pub fn items(s: &Session) -> Vec<Item> {
    let mut out = Vec::new();
    for (i, page) in s.layout().pages().iter().enumerate() {
        walk(s, &page.frame, Transform::identity(), i + 1, &mut out);
    }
    out
}

fn node_of(s: &Session, span: Span) -> Option<(String, &'static str)> {
    let source = s.world().main_source();
    let node = source.find(span)?;
    s.generated().map.node_at(node.offset()).map(|n| (n.id.clone(), n.kind))
}

fn element_of(s: &Session, span: Span) -> Option<(String, &'static str)> {
    let source = s.world().main_source();
    let node = source.find(span)?;
    s.generated().map.element_at(node.offset()).map(|n| (n.id.clone(), n.kind))
}

fn ancestors_of(s: &Session, span: Span) -> Vec<(String, &'static str)> {
    let source = s.world().main_source();
    match source.find(span) {
        Some(node) => s.generated().map.nodes_at(node.offset()).map(|n| (n.id.clone(), n.kind)).collect(),
        None => Vec::new(),
    }
}

fn corners(at: Point, size: Size, ts: Transform) -> Rect {
    let pts = [
        at,
        Point::new(at.x + size.x, at.y),
        Point::new(at.x, at.y + size.y),
        Point::new(at.x + size.x, at.y + size.y),
    ];
    Rect::of(&pts.map(|p| p.transform(ts)))
}

fn walk(s: &Session, frame: &Frame, ts: Transform, page: usize, out: &mut Vec<Item>) {
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Group(group) => {
                let ts = ts.pre_concat(Transform::translate(pos.x, pos.y)).pre_concat(group.transform);
                walk(s, &group.frame, ts, page, out);
            }
            FrameItem::Text(text) => {
                let width = text.glyphs.iter().map(|g| g.x_advance.at(text.size)).sum::<typst::layout::Abs>();
                let size = text.size;
                let metrics = text.font.metrics();
                let top = Point::new(pos.x, pos.y - metrics.ascender.at(size));
                let rect = corners(top, Size::new(width, metrics.ascender.at(size) - metrics.descender.at(size)), ts);
                let span = text.glyphs.first().map(|g| g.span.0).unwrap_or(Span::detached());
                let missing = text.glyphs.iter().any(|g| g.id == 0);
                out.push(Item {
                    page,
                    rect,
                    kind: Kind::Text {
                        text: text.text.to_string(),
                        family: text.font.info().family.clone(),
                        size: size.to_pt(),
                        missing,
                        baseline: Point::new(pos.x, pos.y).transform(ts).y.to_pt(),
                    },
                    span,
                    node: node_of(s, span),
                    element: element_of(s, span),
                    ancestors: ancestors_of(s, span),
                });
            }
            FrameItem::Image(image, size, span) => {
                let px = match image.kind() {
                    typst::visualize::ImageKind::Raster(r) => Some((r.width(), r.height())),
                    _ => None,
                };
                let rect = corners(*pos, *size, ts);
                // The placed size, after any scaling of the groups above it.
                let (w, h) = (rect.x1 - rect.x0, rect.y1 - rect.y0);
                let (w, h) = if is_rotated(ts) { (size.x.to_pt() * scale_of(ts), size.y.to_pt() * scale_of(ts)) } else { (w, h) };
                out.push(Item { page, rect, kind: Kind::Image { px, w, h }, span: *span, node: node_of(s, *span), element: element_of(s, *span), ancestors: ancestors_of(s, *span) });
            }
            FrameItem::Shape(shape, span) => {
                let b = shape.bbox(true);
                let rect = corners(*pos + b.min, Size::new(b.max.x - b.min.x, b.max.y - b.min.y), ts);
                out.push(Item { page, rect, kind: Kind::Shape, span: *span, node: node_of(s, *span), element: element_of(s, *span), ancestors: ancestors_of(s, *span) });
            }
            _ => {}
        }
    }
}

fn is_rotated(ts: Transform) -> bool {
    ts.ky.get().abs() > 1e-9 || ts.kx.get().abs() > 1e-9
}

fn scale_of(ts: Transform) -> f64 {
    (ts.sx.get() * ts.sx.get() + ts.ky.get() * ts.ky.get()).sqrt()
}

/// One node's extent on one page.
#[derive(Clone, Debug, Serialize)]
pub struct Placed {
    pub id: String,
    pub kind: &'static str,
    pub rect: Rect,
}

#[derive(Clone, Debug, Serialize)]
pub struct PageReport {
    pub page: usize,
    /// The page number as printed (`"iv"`, `"12"`).
    pub label: String,
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<Placed>,
}

/// Every page and the nodes on it, with their extents: what an agent reads
/// to know what is where.
pub fn report(s: &Session) -> Vec<PageReport> {
    let all = items(s);
    let mut pages: Vec<PageReport> = s
        .layout()
        .pages()
        .iter()
        .enumerate()
        .map(|(i, p)| PageReport {
            page: i + 1,
            label: p.number.to_string(),
            width: p.frame.width().to_pt(),
            height: p.frame.height().to_pt(),
            nodes: Vec::new(),
        })
        .collect();
    for item in all {
        let page = &mut pages[item.page - 1];
        // Outermost first, so frames come before what is in them.
        for (id, kind) in item.ancestors.iter().rev() {
            match page.nodes.iter_mut().find(|n| &n.id == id) {
                Some(n) => n.rect = n.rect.union(item.rect),
                None => page.nodes.push(Placed { id: id.clone(), kind, rect: item.rect }),
            }
        }
    }
    for p in &mut pages {
        for n in &mut p.nodes {
            n.rect = Rect { x0: r2(n.rect.x0), y0: r2(n.rect.y0), x1: r2(n.rect.x1), y1: r2(n.rect.y1) };
        }
    }
    pages
}

fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
