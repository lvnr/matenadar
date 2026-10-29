//! Where a node's words are on its pages, glyph by glyph: for a caret, for
//! the place in the words nearest a point, and for the rectangles of a
//! selection. A glyph is traced to the node through its span (the text node
//! in the generated program, and the glyph's offset in it) and the source
//! map, which turns that into a byte offset of the node's plain text.

use std::collections::HashMap;

use serde::Serialize;
use typst::layout::{Abs, Frame, FrameItem, Point, Transform};
use typst::syntax::{Source, Span, SyntaxKind};

use crate::editor::PageRect;
use crate::generate::SourceMap;
use crate::inspect::Rect;
use crate::session::Session;

/// A caret: the left edge of a place in the words, as tall as its line.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct Caret {
    pub page: usize,
    pub x: f64,
    /// The top of the line, in points from the trim's top-left corner.
    pub y: f64,
    pub h: f64,
}

/// One glyph of a node: where it is, and the bytes of the node's plain text
/// it draws (`start..end`; a ligature draws several).
#[derive(Clone, Copy, Debug)]
struct GlyphBox {
    page: usize,
    x0: f64,
    x1: f64,
    top: f64,
    bottom: f64,
    baseline: f64,
    start: usize,
    end: usize,
}

/// Where a span's text sits in the program: its start, and whether it is
/// plain text (a glyph's span offset counts within it) or something else,
/// such as an escape, that draws as a whole.
type SpanAt = Option<(usize, usize, bool)>;

struct Tracer<'a> {
    source: &'a Source,
    map: &'a SourceMap,
    id: &'a str,
    spans: HashMap<Span, SpanAt>,
}

impl Tracer<'_> {
    fn span_at(&mut self, span: Span) -> SpanAt {
        if let Some(at) = self.spans.get(&span) {
            return *at;
        }
        let at = (span.id() == Some(self.source.id()))
            .then(|| self.source.find(span))
            .flatten()
            .map(|node| (node.offset(), node.len(), node.kind() == SyntaxKind::Text));
        self.spans.insert(span, at);
        at
    }

    /// The bytes of the node's plain text a glyph draws, if it is the node's.
    fn range(&mut self, span: Span, offset: u16, len: usize) -> Option<(usize, usize)> {
        let (node_start, node_len, text) = self.span_at(span)?;
        let (src, src_len) = if text { (node_start + offset as usize, len) } else { (node_start, node_len) };
        let (node, start) = self.map.locate(src)?;
        if node != self.id {
            return None;
        }
        let end = match self.map.locate(src + src_len) {
            Some((n, e)) if n == self.id && e >= start => e,
            _ => start + len,
        };
        Some((start, end))
    }

    fn walk(&mut self, frame: &Frame, ts: Transform, page: usize, out: &mut Vec<GlyphBox>) {
        for (pos, item) in frame.items() {
            match item {
                FrameItem::Group(group) => {
                    let ts = ts.pre_concat(Transform::translate(pos.x, pos.y)).pre_concat(group.transform);
                    self.walk(&group.frame, ts, page, out);
                }
                FrameItem::Text(text) => {
                    let metrics = text.font.metrics();
                    let (asc, desc) = (metrics.ascender.at(text.size), metrics.descender.at(text.size));
                    let mut x = Abs::zero();
                    for glyph in &text.glyphs {
                        let advance = glyph.x_advance.at(text.size);
                        if let Some((start, end)) = self.range(glyph.span.0, glyph.span.1, glyph.range().len()) {
                            let at = |dx: Abs, dy: Abs| Point::new(pos.x + dx, pos.y + dy).transform(ts);
                            let (p0, p1) = (at(x, Abs::zero()), at(x + advance, Abs::zero()));
                            out.push(GlyphBox {
                                page,
                                x0: p0.x.to_pt().min(p1.x.to_pt()),
                                x1: p0.x.to_pt().max(p1.x.to_pt()),
                                top: at(x, -asc).y.to_pt(),
                                bottom: at(x, -desc).y.to_pt(),
                                baseline: p0.y.to_pt(),
                                start,
                                end,
                            });
                        }
                        x += advance;
                    }
                }
                _ => {}
            }
        }
    }
}

/// The node's glyphs on the pages it reaches (or on one page), in reading order.
fn glyphs(s: &Session, id: &str, only: Option<usize>) -> Vec<GlyphBox> {
    let mut tracer = Tracer { source: s.world().main_source(), map: &s.generated().map, id, spans: HashMap::new() };
    let pages = s.layout().pages();
    let mut out = Vec::new();
    match only {
        Some(page) => {
            if let Some(p) = page.checked_sub(1).and_then(|i| pages.get(i)) {
                tracer.walk(&p.frame, Transform::identity(), page, &mut out);
            }
        }
        None => {
            let start = s.position(id).map(|p| p.0).unwrap_or(1);
            for page in start..=pages.len() {
                let before = out.len();
                tracer.walk(&pages[page - 1].frame, Transform::identity(), page, &mut out);
                if out.len() == before && before > 0 {
                    break;
                }
            }
        }
    }
    out
}

/// The glyphs grouped in lines: by page, then baseline.
fn lines(mut glyphs: Vec<GlyphBox>) -> Vec<Vec<GlyphBox>> {
    let key = |g: &GlyphBox| (g.page, (g.baseline * 10.0).round() as i64);
    glyphs.sort_by(|a, b| key(a).cmp(&key(b)).then(a.x0.total_cmp(&b.x0)));
    let mut out: Vec<Vec<GlyphBox>> = Vec::new();
    for g in glyphs {
        match out.last_mut() {
            Some(line) if key(&line[0]) == key(&g) => line.push(g),
            _ => out.push(vec![g]),
        }
    }
    out
}

fn caret_at(g: &GlyphBox, x: f64) -> Caret {
    Caret { page: g.page, x, y: g.top, h: g.bottom - g.top }
}

/// The caret before the byte `offset` of the node's plain text. Where a line
/// breaks inside a word, the same place ends one line and starts the next:
/// `upstream` puts the caret at the end of the line before (for End, or a
/// click past a line's end).
pub fn caret(s: &Session, id: &str, offset: usize, upstream: bool) -> Option<Caret> {
    let glyphs = glyphs(s, id, None);
    if glyphs.is_empty() {
        // Nothing drawn (an empty paragraph): where the node starts, a line high.
        let (page, x, y) = s.position(id)?;
        return Some(Caret { page, x, y, h: 12.0 });
    }
    if upstream && let Some(g) = glyphs.iter().find(|g| g.end == offset && g.end > g.start) {
        return Some(caret_at(g, g.x1));
    }
    // A glyph that draws the character there (not a hyphen added at a break, which draws none).
    if let Some(g) = glyphs.iter().find(|g| g.start == offset && g.end > g.start) {
        return Some(caret_at(g, g.x0));
    }
    if let Some(g) = glyphs.iter().find(|g| g.start < offset && offset < g.end) {
        let t = (offset - g.start) as f64 / (g.end - g.start) as f64;
        return Some(caret_at(g, g.x0 + (g.x1 - g.x0) * t));
    }
    // Past the last glyph before it: the end of a line (a line-break space), or of the words.
    let before = glyphs.iter().filter(|g| g.end <= offset).max_by_key(|g| (g.end, g.page, (g.baseline * 10.0) as i64));
    match before {
        Some(g) => Some(caret_at(g, g.x1)),
        None => glyphs.iter().min_by_key(|g| g.start).map(|g| caret_at(g, g.x0)),
    }
}

/// The place in the node's words nearest a point on a page: the line the
/// point is on (or the nearest), then the nearest edge of a glyph.
pub fn offset_at(s: &Session, id: &str, page: usize, x: f64, y: f64) -> Option<usize> {
    let lines = lines(glyphs(s, id, Some(page)));
    let line = lines.iter().min_by(|a, b| distance(a, y).total_cmp(&distance(b, y)))?;
    // The nearest glyph edge: a glyph's left edge is its start, its right edge its end.
    line.iter()
        .flat_map(|g| [(g.x0, g.start), (g.x1, g.end)])
        .min_by(|a, b| (a.0 - x).abs().total_cmp(&(b.0 - x).abs()))
        .map(|(_, offset)| offset)
}

/// How far a point is, up or down, from a line (zero inside it).
fn distance(line: &[GlyphBox], y: f64) -> f64 {
    let top = line.iter().map(|g| g.top).fold(f64::INFINITY, f64::min);
    let bottom = line.iter().map(|g| g.bottom).fold(f64::NEG_INFINITY, f64::max);
    if y < top {
        top - y
    } else if y > bottom {
        y - bottom
    } else {
        0.0
    }
}

/// One line of a node's words: where it is and the bytes it sets.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct TextLine {
    pub page: usize,
    pub rect: Rect,
    pub start: usize,
    pub end: usize,
}

/// The lines a node's words set in, in reading order.
pub fn text_lines(s: &Session, id: &str) -> Vec<TextLine> {
    lines(glyphs(s, id, None))
        .into_iter()
        .map(|line| TextLine {
            page: line[0].page,
            rect: bounds(line.iter()),
            start: line.iter().map(|g| g.start).min().unwrap_or(0),
            end: line.iter().map(|g| g.end).max().unwrap_or(0),
        })
        .collect()
}

fn bounds<'a>(glyphs: impl Iterator<Item = &'a GlyphBox> + Clone) -> Rect {
    Rect {
        x0: glyphs.clone().map(|g| g.x0).fold(f64::INFINITY, f64::min),
        x1: glyphs.clone().map(|g| g.x1).fold(f64::NEG_INFINITY, f64::max),
        y0: glyphs.clone().map(|g| g.top).fold(f64::INFINITY, f64::min),
        y1: glyphs.map(|g| g.bottom).fold(f64::NEG_INFINITY, f64::max),
    }
}

/// A rectangle per line over the bytes `start..end` of the node's words.
pub fn selection_rects(s: &Session, id: &str, start: usize, end: usize) -> Vec<PageRect> {
    let (start, end) = (start.min(end), start.max(end));
    lines(glyphs(s, id, None))
        .into_iter()
        .filter_map(|line| {
            let hit: Vec<&GlyphBox> = line.iter().filter(|g| g.start < end && g.end > start).collect();
            let first = hit.first()?;
            Some(PageRect { page: first.page, rect: bounds(hit.iter().copied()) })
        })
        .collect()
}
