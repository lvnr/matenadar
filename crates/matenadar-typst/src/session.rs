use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use matenadar_model::{Document, Op};
use serde::Serialize;
use typst::foundations::Label;
use typst::introspection::{Introspector, PagedPosition};
use typst::layout::{Abs, Point};
use typst::utils::PicoStr;
use typst_ide::Jump;
use typst_layout::{PagedDocument, PagedIntrospector};

use crate::generate::{Generated, generate};
use crate::world::{Fonts, MtWorld, Virtual};

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
pub struct Session {
    doc: Document,
    paths: Paths,
    fonts: Arc<Fonts>,
    virt: Virtual,
    generated: Generated,
    world: MtWorld,
    layout: PagedDocument,
    hashes: Vec<u128>,
    warnings: Vec<String>,
    compile_time: Duration,
}

impl Session {
    pub fn new(doc: Document, paths: Paths) -> Result<Self, EngineError> {
        let fonts = Arc::new(paths.fonts_for(&doc));
        Self::with_fonts(doc, paths, fonts)
    }

    /// With a font set already scanned (scanning is the slow part of starting).
    pub fn with_fonts(doc: Document, paths: Paths, fonts: Arc<Fonts>) -> Result<Self, EngineError> {
        let virt = Virtual::default();
        let generated = generate(&doc, &fonts, &virt, &paths.root);
        let world = MtWorld::new(generated.source.clone(), paths.root.clone(), fonts.clone(), virt.clone());
        let start = Instant::now();
        let (layout, warnings) = compile(&world)?;
        let compile_time = start.elapsed();
        let hashes = page_hashes(&layout);
        Ok(Self { doc, paths, fonts, virt, generated, world, layout, hashes, warnings, compile_time })
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

    /// Applies `op`, regenerates the program and lays it out again
    /// incrementally. On failure the session is unchanged.
    pub fn apply(&mut self, op: &Op) -> Result<Change, EngineError> {
        let mut doc = self.doc.clone();
        let inverse = doc.apply(op).map_err(|e| EngineError(e.to_string()))?;
        let generated = generate(&doc, &self.fonts, &self.virt, &self.paths.root);
        self.world.set_source(&generated.source);
        let start = Instant::now();
        match compile(&self.world) {
            Ok((layout, warnings)) => {
                let compile = start.elapsed();
                let hashes = page_hashes(&layout);
                let changed_pages = (0..hashes.len()).filter(|&i| self.hashes.get(i) != Some(&hashes[i])).map(|i| i + 1).collect();
                self.doc = doc;
                self.generated = generated;
                self.layout = layout;
                self.hashes = hashes;
                self.warnings = warnings;
                self.compile_time = compile;
                Ok(Change { inverse, changed_pages, page_count: self.layout.pages().len(), compile })
            }
            Err(e) => {
                let old = generate(&self.doc, &self.fonts, &self.virt, &self.paths.root);
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

    pub fn pdf(&self) -> Result<Vec<u8>, EngineError> {
        let opts = typst_pdf::PdfOptions { ident: typst::foundations::Smart::Custom(self.doc.meta.title.clone()), ..Default::default() };
        typst_pdf::pdf(&self.layout, &opts).map_err(diagnostics)
    }

    pub fn svg(&self, page: usize, bleed: bool) -> Option<String> {
        let p = self.layout.pages().get(page.checked_sub(1)?)?;
        Some(typst_svg::svg(p, &typst_svg::SvgOptions { render_bleed: bleed, ..Default::default() }))
    }

    /// A page as PNG at `dpi` dots per inch.
    pub fn png(&self, page: usize, dpi: f32, bleed: bool) -> Option<Vec<u8>> {
        let p = self.layout.pages().get(page.checked_sub(1)?)?;
        let opts = typst_render::RenderOptions { pixel_per_pt: typst::utils::Scalar::new((dpi / 72.0).into()), render_bleed: bleed };
        typst_render::render(p, &opts).encode_png().ok()
    }
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
    doc.pages().iter().map(|p| typst::utils::hash128(&p.frame)).collect()
}
