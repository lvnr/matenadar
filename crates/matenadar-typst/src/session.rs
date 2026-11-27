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
    /// Speaker notes from the owning page, section, or spread (omitted from renders).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// An in-memory file attachment passed programmatically to [`Session::pdf_with_options`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PdfAttachmentData {
    /// Filename stored inside the PDF (e.g. `"factur-x.xml"`).
    pub name: String,
    /// Raw file bytes.
    pub data: Vec<u8>,
    /// Optional MIME type (e.g. `"text/xml"`, `"application/xml"`).
    pub mime: Option<String>,
    pub description: Option<String>,
    /// `"data"`, `"source"`, `"alternative"`, `"supplement"`, or `"unspecified"`.
    pub relationship: Option<String>,
}

/// Options for [`Session::pdf_with_options`]. Unset fields inherit from the
/// document's `pdf` and `print` configuration.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PdfOptions {
    /// PDF standards to enforce (e.g. `["a-3b"]`, `["a-2b"]`, `["ua-1"]`, `["1.7"]`).
    pub standards: Vec<String>,
    /// Emit tagged PDF structure (`None` inherits `doc.pdf.tagged`, default `true`).
    pub tagged: Option<bool>,
    /// Extra programmatic in-memory attachments merged with `doc.pdf.attachments`.
    pub attachments: Vec<PdfAttachmentData>,
    /// Optional target maximum PPI for deterministic Lanczos-3 downsampling of
    /// oversized raster images (`None` inherits `doc.print.max_ppi`).
    pub max_ppi: Option<f32>,
    /// Optional ISO-8601 UTC timestamp (`"YYYY-MM-DD"` or `"YYYY-MM-DDTHH:MM:SSZ"`).
    /// When omitted and PDF/A or embedded attachments require a creation date,
    /// a deterministic UTC timestamp is used.
    pub timestamp: Option<String>,
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
    /// Each page's print, for telling which pages a change touched.
    hashes: Vec<u128>,
    /// The prints of the frames of `layout` (see `page_prints`).
    prints: Prints,
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
        let (hashes, prints) = page_prints(&layout, &Prints::new());
        Ok(Self { doc, paths, files, fonts, virt, generated, world, layout, hashes, prints, warnings, compile_time })
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
        let (doc, mut inverses) = self.doc.applied(ops).map_err(|e| EngineError(e.to_string()))?;
        inverses.reverse();
        let generated = generate(&doc, &self.fonts, &self.virt, &self.files);
        self.world.set_source(&generated.source);
        let start = Stopwatch::start();
        match compile(&self.world) {
            Ok((layout, warnings)) => {
                let layout = crate::effects::apply(layout, &generated.effects);
                let compile = start.elapsed();
                let (hashes, prints) = page_prints(&layout, &self.prints);
                let changed_pages = (0..hashes.len()).filter(|&i| self.hashes.get(i) != Some(&hashes[i])).map(|i| i + 1).collect();
                self.doc = doc;
                self.generated = generated;
                self.layout = layout;
                self.hashes = hashes;
                self.prints = prints;
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
                let item_ref = item.as_deref().and_then(|id| self.doc.content.iter().find(|c| c.id() == id));
                let master = item_ref.and_then(|c| c.master().map(str::to_string));
                let notes = item_ref.and_then(|c| c.notes()).map(matenadar_model::Notes::plain_text);
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
                    notes,
                }
            })
            .collect()
    }

    /// Speaker notes for 1-based `page`, if the owning page, section, or spread has any.
    pub fn notes(&self, page: usize) -> Option<String> {
        self.pages().into_iter().find(|p| p.page == page).and_then(|p| p.notes)
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
        self.layout.pages().iter().map(|p| format!("{:032x}", print_of(&p.frame))).collect()
    }

    pub fn pdf(&self) -> Result<Vec<u8>, EngineError> {
        self.pdf_with_options(&PdfOptions::default())
    }

    /// Exports the document as PDF with explicit standards, tagging,
    /// programmatic embedded attachments, and optional deterministic image
    /// downsampling (`max_ppi`).
    pub fn pdf_with_options(&self, opts: &PdfOptions) -> Result<Vec<u8>, EngineError> {
        let std_names: Vec<String> = if !opts.standards.is_empty() {
            opts.standards.clone()
        } else {
            self.doc.pdf.as_ref().map(|p| p.resolved_standards()).unwrap_or_default()
        };
        let standards = parse_pdf_standards(&std_names)?;
        let tagged = opts
            .tagged
            .or_else(|| self.doc.pdf.as_ref().and_then(|p| p.tagged))
            .unwrap_or(true);
        let max_ppi = opts
            .max_ppi
            .or_else(|| self.doc.print.as_ref().and_then(|p| p.max_ppi).map(|v| v as f32));

        let mut export_layout = if opts.attachments.is_empty() {
            None
        } else {
            let virt = self.virt.snapshot();
            let mut extra = String::new();
            for (i, att) in opts.attachments.iter().enumerate() {
                if att.name.trim().is_empty() {
                    return Err(EngineError("PDF attachment name cannot be empty".into()));
                }
                let vpath = virt.insert(&format!("pdf-att-{i}.bin"), att.data.clone());
                extra.push_str(&crate::generate::pdf_attach_stmt(
                    &att.name,
                    &vpath,
                    att.mime.as_deref(),
                    att.description.as_deref(),
                    att.relationship.as_deref(),
                ));
            }
            let source = format!("{}\n{extra}", self.generated.source);
            let world = MtWorld::new(source, self.files.clone(), self.fonts.clone(), virt);
            let (layout, _) = compile(&world)?;
            Some(crate::effects::apply(layout, &self.generated.effects))
        };

        if let Some(ppi) = max_ppi {
            let base = export_layout.take().unwrap_or_else(|| self.layout.clone());
            export_layout = Some(crate::resample::downsample_document(base, ppi));
        }

        let has_attachments = !opts.attachments.is_empty()
            || self.doc.pdf.as_ref().is_some_and(|p| !p.attachments.is_empty());
        let needs_date = has_attachments || std_names.iter().any(|s| s.trim().to_ascii_lowercase().starts_with("a-"));
        let timestamp = match opts.timestamp.as_deref() {
            Some(ts) => Some(parse_pdf_timestamp(ts)?),
            None if needs_date => Some(typst_pdf::Timestamp::new_utc(
                typst::foundations::Datetime::from_ymd_hms(2026, 1, 1, 0, 0, 0).expect("valid fallback date"),
            )),
            None => None,
        };

        let target = export_layout.as_ref().unwrap_or(&self.layout);
        let typst_opts = typst_pdf::PdfOptions {
            ident: typst::foundations::Smart::Custom(self.doc.meta.title.clone()),
            timestamp,
            standards,
            tagged,
            ..Default::default()
        };
        typst_pdf::pdf(target, &typst_opts).map_err(diagnostics)
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

    /// A page as RGB pixels on white paper at `dpi`, drawn once (no
    /// supersampling), for pictures that are compressed next (an ebook's
    /// pages); without the text runs `drop` picks, when given.
    pub fn rgb(&self, page: usize, dpi: f32, drop: Option<&dyn Fn(&typst::text::TextItem) -> bool>) -> Option<(u32, u32, Vec<u8>)> {
        let p = self.layout.pages().get(page.checked_sub(1)?)?;
        let stripped;
        let p = match drop {
            Some(drop) => {
                let mut q = p.clone();
                q.frame = without_text(&q.frame, drop);
                stripped = q;
                &stripped
            }
            None => p,
        };
        let opts = typst_render::RenderOptions { pixel_per_pt: typst::utils::Scalar::new((dpi / 72.0).into()), render_bleed: false };
        let pixmap = typst_render::render(p, &opts);
        let (w, h) = (pixmap.width(), pixmap.height());
        let mut rgb = Vec::with_capacity(w as usize * h as usize * 3);
        for px in pixmap.data().as_chunks::<4>().0 {
            // Premultiplied: over white, each channel gains what the alpha leaves.
            let paper = 255 - px[3];
            rgb.extend_from_slice(&[px[0].saturating_add(paper), px[1].saturating_add(paper), px[2].saturating_add(paper)]);
        }
        Some((w, h, rgb))
    }
}

/// A frame with the text runs `drop` picks left out, groups and all else kept.
fn without_text(frame: &typst::layout::Frame, drop: &dyn Fn(&typst::text::TextItem) -> bool) -> typst::layout::Frame {
    use typst::layout::FrameItem;
    let mut out = typst::layout::Frame::new(frame.size(), frame.kind());
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Text(t) if drop(t) => {}
            FrameItem::Group(g) => {
                let mut g = g.clone();
                g.frame = without_text(&g.frame, drop);
                out.push(*pos, FrameItem::Group(g));
            }
            other => out.push(*pos, other.clone()),
        }
    }
    out
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
    // Typst memoizes the layout and keeps every result until told to forget:
    // without this, each edit's layouts stay for good (half a megabyte a
    // keystroke on a short book, and WebAssembly memory never shrinks).
    // What the last few layouts used stays, as in Typst's own watch mode.
    comemo::evict(10);
    let warnings = warned
        .warnings
        .iter()
        .map(|w| w.message.to_string())
        .filter(|m| !m.contains("layout did not converge") || true)
        .collect();
    warned.output.map(|d| (d, warnings)).map_err(diagnostics)
}

fn parse_pdf_standards(names: &[String]) -> Result<typst_pdf::PdfStandards, EngineError> {
    use typst_pdf::PdfStandard;
    if names.is_empty() {
        return Ok(typst_pdf::PdfStandards::default());
    }
    let mut list = Vec::with_capacity(names.len());
    for raw in names {
        let s = match raw.trim().to_ascii_lowercase().as_str() {
            "1.4" => PdfStandard::V_1_4,
            "1.5" => PdfStandard::V_1_5,
            "1.6" => PdfStandard::V_1_6,
            "1.7" => PdfStandard::V_1_7,
            "2.0" => PdfStandard::V_2_0,
            "a-1b" | "pdf/a-1b" => PdfStandard::A_1b,
            "a-1a" | "pdf/a-1a" => PdfStandard::A_1a,
            "a-2b" | "pdf/a-2b" => PdfStandard::A_2b,
            "a-2u" | "pdf/a-2u" => PdfStandard::A_2u,
            "a-2a" | "pdf/a-2a" => PdfStandard::A_2a,
            "a-3b" | "pdf/a-3b" => PdfStandard::A_3b,
            "a-3u" | "pdf/a-3u" => PdfStandard::A_3u,
            "a-3a" | "pdf/a-3a" => PdfStandard::A_3a,
            "a-4" | "pdf/a-4" => PdfStandard::A_4,
            "a-4f" | "pdf/a-4f" => PdfStandard::A_4f,
            "a-4e" | "pdf/a-4e" => PdfStandard::A_4e,
            "ua-1" | "pdf/ua-1" => PdfStandard::Ua_1,
            other => {
                return Err(EngineError(format!(
                    "unknown PDF standard \"{other}\" (use {})",
                    matenadar_model::PDF_STANDARDS.join(", ")
                )));
            }
        };
        list.push(s);
    }
    typst_pdf::PdfStandards::new(&list).map_err(|e| EngineError(e.message().to_string()))
}

fn parse_pdf_timestamp(raw: &str) -> Result<typst_pdf::Timestamp, EngineError> {
    let s = raw.trim().trim_end_matches('Z');
    let (date_part, time_part) = match s.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };
    let d: Vec<&str> = date_part.split('-').collect();
    if d.len() != 3 {
        return Err(EngineError(format!("invalid PDF timestamp \"{raw}\" (use YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ)")));
    }
    let y: i32 = d[0].parse().map_err(|_| EngineError(format!("invalid year in timestamp \"{raw}\"")))?;
    let m: u8 = d[1].parse().map_err(|_| EngineError(format!("invalid month in timestamp \"{raw}\"")))?;
    let day: u8 = d[2].parse().map_err(|_| EngineError(format!("invalid day in timestamp \"{raw}\"")))?;
    let (hh, mm, ss) = if let Some(tp) = time_part {
        let t: Vec<&str> = tp.split(':').collect();
        if t.len() < 2 || t.len() > 3 {
            return Err(EngineError(format!("invalid time in timestamp \"{raw}\"")));
        }
        let h: u8 = t[0].parse().map_err(|_| EngineError(format!("invalid hour in timestamp \"{raw}\"")))?;
        let min: u8 = t[1].parse().map_err(|_| EngineError(format!("invalid minute in timestamp \"{raw}\"")))?;
        let sec: u8 = if t.len() == 3 {
            t[2].split('.').next().unwrap_or("0").parse().map_err(|_| EngineError(format!("invalid second in timestamp \"{raw}\"")))?
        } else {
            0
        };
        (h, min, sec)
    } else {
        (0, 0, 0)
    };
    let dt = typst::foundations::Datetime::from_ymd_hms(y, m, day, hh, mm, ss)
        .ok_or_else(|| EngineError(format!("out-of-range PDF timestamp \"{raw}\"")))?;
    Ok(typst_pdf::Timestamp::new_utc(dt))
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

/// Prints of frames (what they draw, as in `frame_print`), by where a
/// frame's items are in memory and its size. Those of the layout in hand: that
/// layout keeps their items alive, so a frame of the next layout whose items
/// are at the same place is the very same frame, kept by Typst's memory of
/// layouts, and its print is known without looking at it again.
type Prints = HashMap<(usize, usize, u64, u64), u128>;

/// Each page's print, for telling which pages a change touched, and the
/// prints of every frame met (for the next change). After an edit, only the
/// frames laid out anew are looked at: a long book costs what the edit does.
fn page_prints(doc: &PagedDocument, known: &Prints) -> (Vec<u128>, Prints) {
    let mut next = Prints::with_capacity(known.len());
    let pages = doc.pages().iter().map(|p| frame_print(&p.frame, known, &mut next)).collect();
    (pages, next)
}

/// What a frame draws, leaving out where in the generated program each
/// piece came from (those positions move when text before them changes).
fn frame_print(frame: &typst::layout::Frame, known: &Prints, next: &mut Prints) -> u128 {
    use typst::layout::FrameItem;
    use typst::utils::hash128;
    let items = frame.items().as_slice();
    let size = frame.size();
    let key = (items.as_ptr() as usize, items.len(), size.x.to_raw().to_bits(), size.y.to_raw().to_bits());
    if let Some(&h) = known.get(&key).or_else(|| next.get(&key)) {
        next.insert(key, h);
        return h;
    }
    let mut parts = Vec::with_capacity(items.len() + 1);
    parts.push(hash128(&size));
    for (pos, item) in items {
        match item {
            FrameItem::Group(g) => parts.push(hash128(&(pos, g.transform, &g.clip, frame_print(&g.frame, known, next)))),
            FrameItem::Text(t) => {
                let glyphs: Vec<_> = t.glyphs.iter().map(|g| (g.id, g.x_advance, g.x_offset, g.y_advance, g.y_offset)).collect();
                parts.push(hash128(&(pos, &t.font, t.size, &t.fill, &t.stroke, t.lang, t.region, &t.text, glyphs)));
            }
            FrameItem::Shape(shape, _) => parts.push(hash128(&(pos, shape))),
            FrameItem::Image(image, size, _) => parts.push(hash128(&(pos, image, size))),
            FrameItem::Link(_, size) => parts.push(hash128(&(pos, size))),
            FrameItem::Tag(_) => {}
        }
    }
    let h = hash128(&parts);
    next.insert(key, h);
    h
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
