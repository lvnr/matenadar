use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use matenadar_model::{Document, Op};
use serde::Serialize;
use typst::foundations::Label;
use typst::introspection::{Introspector, PagedPosition};
use typst::layout::{Abs, Point};
use typst::utils::PicoStr;
use typst_ide::Jump;
use typst_layout::{PagedDocument, PagedIntrospector};

use crate::generate::{Generated, generate};
use crate::world::{Files, Fonts, MtWorld, Virtual};

#[derive(Debug, Clone, PartialEq)]
pub struct EngineError(pub String);

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EngineError {}

/// What an operation changed.
#[derive(Debug, Serialize)]
pub struct Change {
    pub inverse: Op,
    /// 1-based numbers of pages whose layout differs, including new pages.
    pub changed_pages: Vec<usize>,
    pub page_count: usize,
    #[serde(serialize_with = "ms")]
    pub compile: Duration,
}

/// What a list of operations, applied together, changed.
#[derive(Debug, Serialize)]
pub struct Changes {
    /// The operations that undo them, in the order to apply them.
    pub inverses: Vec<Op>,
    pub changed_pages: Vec<usize>,
    pub page_count: usize,
    #[serde(serialize_with = "ms")]
    pub compile: Duration,
}

/// A page's geometry, in points from the trim's top-left corner.
#[derive(Clone, Debug, Serialize)]
pub struct PageInfo {
    pub page: usize,
    /// The page number as printed.
    pub label: String,
    /// `recto` (odd pages, on the right) or `verso`.
    pub side: &'static str,
    /// The section or page it belongs to, and that item's master.
    pub item: Option<String>,
    pub master: Option<String>,
    pub trim: Size,
    pub bleed: Sides,
    /// The master's margins (the text area's distance from the trim).
    pub margins: Option<Margins>,
}

/// A font face the layout drew with.
#[derive(Clone, Debug, Serialize)]
pub struct UsedFont {
    pub family: String,
    pub weight: u16,
    pub italic: bool,
    pub stretch: f64,
    /// The file it was read from (and its index in a collection); none for
    /// a font given as bytes or bundled with Typst.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    pub pages: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Sides {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Margins {
    pub top: f64,
    pub bottom: f64,
    pub inside: f64,
    pub outside: f64,
}

fn ms<S: serde::Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_f64(d.as_secs_f64() * 1000.0)
}

/// Where a document's files are.
#[derive(Clone, Debug)]
pub struct Paths {
    /// Images, drawings and the document's font folders are under here.
    pub root: PathBuf,
    /// More font folders.
    pub font_dirs: Vec<PathBuf>,
    /// Also use the system's fonts.
    pub system_fonts: bool,
}

impl Paths {
    pub fn fonts_for(&self, doc: &Document) -> Fonts {
        let mut dirs: Vec<PathBuf> = doc.fonts.iter().map(|d| self.root.join(d)).collect();
        dirs.extend(self.font_dirs.iter().cloned());
        Fonts::new(&dirs, self.system_fonts)
    }
}

/// A document laid out by Typst, kept up to date as operations arrive.
/// The generated program, with where each node is in it.
#[derive(Debug, Serialize)]
pub struct SourceExport {
    pub text: String,
    pub prelude: usize,
    pub nodes: Vec<SourceNode>,
}

#[derive(Debug, Serialize)]
pub struct SourceNode {
    pub id: String,
    pub kind: &'static str,
    pub start: usize,
    pub end: usize,
}

pub struct Session {
    doc: Document,
    paths: Paths,
    files: Files,
    fonts: Arc<Fonts>,
    virt: Virtual,
    generated: Generated,
    world: MtWorld,
    layout: PagedDocument,
    hashes: Vec<u128>,
    warnings: Vec<String>,
    compile_time: Duration,
}

/// The Typst program generated for `doc`, without laying it out (for
/// debugging the generator when layout fails).
pub fn source_of(doc: &Document, paths: &Paths) -> String {
    let fonts = paths.fonts_for(doc);
    generate(doc, &fonts, &Virtual::default(), &Files::at(&paths.root)).source
}

/// Times layout. WebAssembly in the browser has no clock to read, so there
/// it measures nothing.
struct Stopwatch(#[cfg(not(target_arch = "wasm32"))] std::time::Instant);

impl Stopwatch {
    fn start() -> Self {
        Self(
            #[cfg(not(target_arch = "wasm32"))]
            std::time::Instant::now(),
        )
    }

    fn elapsed(&self) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        return self.0.elapsed();
        #[cfg(target_arch = "wasm32")]
        Duration::ZERO
    }
}

impl Session {
    pub fn new(doc: Document, paths: Paths) -> Result<Self, EngineError> {
        let fonts = Arc::new(paths.fonts_for(&doc));
        Self::with_fonts(doc, paths, fonts)
    }

    /// With a font set already scanned (scanning is the slow part of starting).
    pub fn with_fonts(doc: Document, paths: Paths, fonts: Arc<Fonts>) -> Result<Self, EngineError> {
        let files = Files::at(&paths.root);
        Self::with_files(doc, paths, fonts, files)
    }

    /// With the document's files given (in memory, or under a root).
    pub fn with_files(doc: Document, paths: Paths, fonts: Arc<Fonts>, files: Files) -> Result<Self, EngineError> {
        let virt = Virtual::default();
        let generated = generate(&doc, &fonts, &virt, &files);
        let world = MtWorld::new(generated.source.clone(), files.clone(), fonts.clone(), virt.clone());
        let start = Stopwatch::start();
        let (layout, warnings) = compile(&world)?;
        let layout = crate::effects::apply(layout, &generated.effects);
        let compile_time = start.elapsed();
        let hashes = page_hashes(&layout);
        Ok(Self { doc, paths, files, fonts, virt, generated, world, layout, hashes, warnings, compile_time })
    }

    pub fn document(&self) -> &Document {
        &self.doc
    }

    pub fn layout(&self) -> &PagedDocument {
        &self.layout
    }

    pub fn world(&self) -> &MtWorld {
        &self.world
    }

    pub fn generated(&self) -> &Generated {
        &self.generated
    }

    /// The generated program, for debugging the engine itself.
    pub fn source(&self) -> &str {
        &self.generated.source
    }

    pub fn page_count(&self) -> usize {
        self.layout.pages().len()
    }

    pub fn compile_time(&self) -> Duration {
        self.compile_time
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn problems(&self) -> &[String] {
        &self.generated.problems
    }

    pub fn fonts(&self) -> &Arc<Fonts> {
        &self.fonts
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn files(&self) -> &Files {
        &self.files
    }

    /// Applies `op`, regenerates the program and lays it out again
    /// incrementally. On failure the session is unchanged.
    pub fn apply(&mut self, op: &Op) -> Result<Change, EngineError> {
        let c = self.apply_all(std::slice::from_ref(op))?;
        Ok(Change { inverse: c.inverses.into_iter().next().expect("one operation"), changed_pages: c.changed_pages, page_count: c.page_count, compile: c.compile })
    }

    /// Applies operations in order and lays the result out once. On
    /// failure the session is unchanged.
    pub fn apply_all(&mut self, ops: &[Op]) -> Result<Changes, EngineError> {
        let mut doc = self.doc.clone();
        let mut inverses = Vec::with_capacity(ops.len());
        for (i, op) in ops.iter().enumerate() {
            let inverse = doc.apply(op).map_err(|e| EngineError(if ops.len() > 1 { format!("operation {}: {e}", i + 1) } else { e.to_string() }))?;
            inverses.push(inverse);
        }
        inverses.reverse();
        let generated = generate(&doc, &self.fonts, &self.virt, &self.files);
        self.world.set_source(&generated.source);
        let start = Stopwatch::start();
        match compile(&self.world) {
            Ok((layout, warnings)) => {
                let layout = crate::effects::apply(layout, &generated.effects);
                let compile = start.elapsed();
                let hashes = page_hashes(&layout);
                let changed_pages = (0..hashes.len()).filter(|&i| self.hashes.get(i) != Some(&hashes[i])).map(|i| i + 1).collect();
                self.doc = doc;
                self.generated = generated;
                self.layout = layout;
                self.hashes = hashes;
                self.warnings = warnings;
                self.compile_time = compile;
                Ok(Changes { inverses, changed_pages, page_count: self.layout.pages().len(), compile })
            }
            Err(e) => {
                let old = generate(&self.doc, &self.fonts, &self.virt, &self.files);
                self.world.set_source(&old.source);
                Err(e)
            }
        }
    }

    /// The node and plain-text offset under a point on a page (1-based
    /// page, points from the trim's top-left corner).
    pub fn locate(&self, page: usize, x_pt: f64, y_pt: f64) -> Option<(String, usize)> {
        let pos = PagedPosition { page: NonZeroUsize::new(page)?, point: Point::new(Abs::pt(x_pt), Abs::pt(y_pt)) };
        match typst_ide::jump_from_click(&self.world, &self.layout, &pos)? {
            Jump::File(id, offset) if id == self.world.main_source().id() => {
                if let Some((n, o)) = self.generated.map.locate(offset) {
                    return Some((n.to_string(), o));
                }
                self.generated.map.node_at(offset).map(|n| (n.id.clone(), 0))
            }
            _ => None,
        }
    }

    /// Where a node starts: 1-based page and point.
    pub fn position(&self, id: &str) -> Option<(usize, f64, f64)> {
        let label = Label::new(PicoStr::intern(id))?;
        let intro = self.layout.introspector();
        let loc = intro.query_label(label).ok()?.location()?;
        let pos = PagedIntrospector::position(intro, loc)?;
        Some((pos.page.get(), pos.point.x.to_pt(), pos.point.y.to_pt()))
    }

        /// The program the engine generated, and where each node is in it, in
    /// UTF-16 code units (as JavaScript counts); `prelude` is the length of
    /// Matenadar's own definitions at its start.
    pub fn source_export(&self) -> SourceExport {
        let text = &self.generated.source;
        let nodes = self.generated.map.nodes();
        let mut wanted: Vec<usize> = nodes.iter().flat_map(|n| [n.src.start, n.src.end]).collect();
        wanted.push(crate::generate::PRELUDE.len().min(text.len()));
        wanted.sort_unstable();
        wanted.dedup();
        // One pass over the text turns byte offsets into UTF-16 ones.
        let mut utf16 = HashMap::with_capacity(wanted.len());
        let (mut byte, mut unit) = (0, 0);
        let mut chars = text.chars();
        for &w in &wanted {
            while byte < w {
                let Some(c) = chars.next() else { break };
                byte += c.len_utf8();
                unit += c.len_utf16();
            }
            utf16.insert(w, unit);
        }
        SourceExport {
            text: text.clone(),
            prelude: utf16[&crate::generate::PRELUDE.len().min(text.len())],
            nodes: nodes.iter().map(|n| SourceNode { id: n.id.clone(), kind: n.kind, start: utf16[&n.src.start], end: utf16[&n.src.end] }).collect(),
        }
    }

/// Every page's geometry: trim, bleed, side, master and margins.
    pub fn pages(&self) -> Vec<PageInfo> {
        // Sections and pages mark their first page (`<mt-sec>`).
        let intro = self.layout.introspector();
        let mut starts: Vec<(usize, String)> = Vec::new();
        if let Some(label) = Label::new(PicoStr::intern("mt-sec")) {
            for m in intro.query(&typst::foundations::Selector::Label(label)).iter() {
                let page = m.location().and_then(|l| PagedIntrospector::page(intro, l)).map(|p| p.get());
                let id = match m.get_by_name("value").ok() {
                    Some(typst::foundations::Value::Dict(d)) => d.get("id").ok().and_then(|v| v.clone().cast::<typst::foundations::Str>().ok()).map(|s| s.to_string()),
                    _ => None,
                };
                if let (Some(p), Some(id)) = (page, id) {
                    starts.push((p, id));
                }
            }
        }
        starts.sort_by_key(|s| s.0);
        let pt = |l: Option<matenadar_model::Length>, d: f64| l.and_then(|l| l.pt()).unwrap_or(d);
        self.layout
            .pages()
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let page = i + 1;
                let item = starts.iter().rev().find(|s| s.0 <= page).map(|s| s.1.clone());
                let master = item.as_deref().and_then(|id| self.doc.content.iter().find(|c| c.id() == id)).and_then(|c| c.master().map(str::to_string));
                let margins = master.as_deref().and_then(|m| self.doc.design.master(m)).map(|m| {
                    let mg = m.margins.unwrap_or_default();
                    let recto = page % 2 == 1;
                    let (inside, outside) = match (mg.left, mg.right) {
                        (None, None) => (pt(mg.inside, 36.0), pt(mg.outside, 36.0)),
                        (l, r) if recto => (pt(l, 36.0), pt(r, 36.0)),
                        (l, r) => (pt(r, 36.0), pt(l, 36.0)),
                    };
                    Margins { top: pt(mg.top, 36.0), bottom: pt(mg.bottom, 36.0), inside, outside }
                });
                PageInfo {
                    page,
                    label: p.number.to_string(),
                    side: if page % 2 == 1 { "recto" } else { "verso" },
                    item,
                    master,
                    trim: Size { w: p.frame.width().to_pt(), h: p.frame.height().to_pt() },
                    bleed: Sides { top: p.bleed.top.to_pt(), right: p.bleed.right.to_pt(), bottom: p.bleed.bottom.to_pt(), left: p.bleed.left.to_pt() },
                    margins,
                }
            })
            .collect()
    }

    /// The font faces the layout drew with, and where each came from: what
    /// another engine (the browser's) needs to lay the document out the same.
    pub fn fonts_used(&self) -> Vec<UsedFont> {
        fn visit(frame: &typst::layout::Frame, page: usize, out: &mut Vec<(typst::text::Font, usize)>) {
            for (_, item) in frame.items() {
                match item {
                    typst::layout::FrameItem::Group(g) => visit(&g.frame, page, out),
                    typst::layout::FrameItem::Text(t) => {
                        let font = t.font.font();
                        if !out.iter().any(|(f, p)| f == font && *p == page) {
                            out.push((font.clone(), page));
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut seen = Vec::new();
        for (i, p) in self.layout.pages().iter().enumerate() {
            visit(&p.frame, i + 1, &mut seen);
        }
        let mut out: Vec<(typst::text::Font, UsedFont)> = Vec::new();
        for (font, page) in seen {
            if let Some((_, used)) = out.iter_mut().find(|(f, _)| *f == font) {
                used.pages.push(page);
                continue;
            }
            let info = font.info();
            let file = self.fonts.file_of(&font);
            out.push((
                font.clone(),
                UsedFont {
                    family: info.family.clone(),
                    weight: info.variant.weight.to_number(),
                    italic: info.variant.style != typst::text::FontStyle::Normal,
                    stretch: info.variant.stretch.to_ratio().get() * 100.0,
                    file: file.as_ref().map(|f| f.0.display().to_string()),
                    index: file.map(|f| f.1),
                    pages: vec![page],
                },
            ));
        }
        out.into_iter().map(|(_, u)| u).collect()
    }

    /// One hash per page, equal wherever the engine runs when the pages are
    /// the same (the browser's build and the command line's).
    pub fn fingerprints(&self) -> Vec<String> {
        self.hashes.iter().map(|h| format!("{h:032x}")).collect()
    }

    pub fn pdf(&self) -> Result<Vec<u8>, EngineError> {
        let opts = typst_pdf::PdfOptions { ident: typst::foundations::Smart::Custom(self.doc.meta.title.clone()), ..Default::default() };
        typst_pdf::pdf(&self.layout, &opts).map_err(diagnostics)
    }

    pub fn svg(&self, page: usize, bleed: bool) -> Option<String> {
        let p = self.layout.pages().get(page.checked_sub(1)?)?;
        Some(typst_svg::svg(p, &typst_svg::SvgOptions { render_bleed: bleed, ..Default::default() }))
    }

    /// A page as PNG at `dpi` dots per inch. Below 200 dpi the page is drawn
    /// at twice the size and scaled down, for smooth edges: small type, hairlines
    /// and QR codes stay legible on screen.
    pub fn png(&self, page: usize, dpi: f32, bleed: bool) -> Option<Vec<u8>> {
        let p = self.layout.pages().get(page.checked_sub(1)?)?;
        let factor = if dpi < 200.0 { 2.0 } else { 1.0 };
        let opts = typst_render::RenderOptions { pixel_per_pt: typst::utils::Scalar::new((dpi * factor / 72.0).into()), render_bleed: bleed };
        let pixmap = typst_render::render(p, &opts);
        if factor == 1.0 { pixmap.encode_png().ok() } else { halve(&pixmap)?.encode_png().ok() }
    }
}

impl Session {
    /// A page as straight (not premultiplied) RGBA pixels, `scale` pixels
    /// per point, for a canvas: (width, height, pixels).
    pub fn rgba(&self, page: usize, scale: f32, bleed: bool) -> Option<(u32, u32, Vec<u8>)> {
        let p = self.layout.pages().get(page.checked_sub(1)?)?;
        let opts = typst_render::RenderOptions { pixel_per_pt: typst::utils::Scalar::new(scale.into()), render_bleed: bleed };
        let pixmap = typst_render::render(p, &opts);
        let (w, h) = (pixmap.width(), pixmap.height());
        let mut data = pixmap.take();
        for px in data.as_chunks_mut::<4>().0 {
            let a = px[3] as u32;
            if a != 0 && a != 255 {
                for c in &mut px[..3] {
                    *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        Some((w, h, data))
    }
}

/// Halves a pixmap, each pixel the mean of four (premultiplied, so edges blend right).
fn halve(src: &tiny_skia::Pixmap) -> Option<tiny_skia::Pixmap> {
    let (w, h) = (src.width() / 2, src.height() / 2);
    let mut out = tiny_skia::Pixmap::new(w.max(1), h.max(1))?;
    let sw = src.width() as usize;
    let s = src.data();
    let d = out.data_mut();
    for y in 0..h as usize {
        for x in 0..w as usize {
            for c in 0..4 {
                let at = |xx: usize, yy: usize| s[(yy * sw + xx) * 4 + c] as u32;
                let sum = at(2 * x, 2 * y) + at(2 * x + 1, 2 * y) + at(2 * x, 2 * y + 1) + at(2 * x + 1, 2 * y + 1);
                d[(y * w as usize + x) * 4 + c] = ((sum + 2) / 4) as u8;
            }
        }
    }
    Some(out)
}

pub(crate) fn compile(world: &MtWorld) -> Result<(PagedDocument, Vec<String>), EngineError> {
    let warned = typst::compile::<PagedDocument>(world);
    let warnings = warned
        .warnings
        .iter()
        .map(|w| w.message.to_string())
        .filter(|m| !m.contains("layout did not converge") || true)
        .collect();
    warned.output.map(|d| (d, warnings)).map_err(diagnostics)
}

fn diagnostics(errors: typst::ecow::EcoVec<typst::diag::SourceDiagnostic>) -> EngineError {
    EngineError(
        errors
            .iter()
            .map(|d| {
                let mut m = d.message.to_string();
                for h in &d.hints {
                    m.push_str(&format!(" (hint: {})", h.v));
                }
                m
            })
            .collect::<Vec<_>>()
            .join("; "),
    )
}

fn page_hashes(doc: &PagedDocument) -> Vec<u128> {
    doc.pages().iter().map(|p| print_of(&p.frame)).collect()
}

/// A hash of what a frame draws, leaving out where in the generated
/// program each piece came from: those positions differ between a document
/// edited in place and the same document laid out afresh.
fn print_of(frame: &typst::layout::Frame) -> u128 {
    fn visit(frame: &typst::layout::Frame, out: &mut Vec<u128>) {
        use typst::layout::FrameItem;
        use typst::utils::hash128;
        out.push(hash128(&frame.size()));
        for (pos, item) in frame.items() {
            match item {
                FrameItem::Group(g) => {
                    out.push(hash128(&(pos, g.transform, &g.clip)));
                    visit(&g.frame, out);
                    out.push(0);
                }
                FrameItem::Text(t) => {
                    let glyphs: Vec<_> = t.glyphs.iter().map(|g| (g.id, g.x_advance, g.x_offset, g.y_advance, g.y_offset)).collect();
                    out.push(hash128(&(pos, &t.font, t.size, &t.fill, &t.stroke, t.lang, t.region, &t.text, glyphs)));
                }
                FrameItem::Shape(shape, _) => out.push(hash128(&(pos, shape))),
                FrameItem::Image(image, size, _) => out.push(hash128(&(pos, image, size))),
                FrameItem::Link(_, size) => out.push(hash128(&(pos, size))),
                FrameItem::Tag(_) => {}
            }
        }
    }
    let mut parts = Vec::new();
    visit(frame, &mut parts);
    typst::utils::hash128(&parts)
}
