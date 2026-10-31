//! Preflight: what a printer or KDP's previewer would refuse, and what a
//! careful typesetter would fix.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use typst::foundations::Label;
use typst::introspection::Introspector;
use typst::utils::PicoStr;

use crate::inspect::{Item, Kind, items, items_on};
use crate::session::Session;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug, Serialize)]
pub struct Issue {
    pub severity: Severity,
    /// A stable code: `overflow`, `text-in-margin`, `low-resolution`,
    /// `color-in-black-ink`, `missing-glyph`, `fallback-font`, `widow`,
    /// `orphan`, `missing-file`, `compiler`.
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct CheckOptions {
    /// Live text must stay this far inside the trim (KDP: 0.25 in).
    pub safe_pt: f64,
    /// The spine-side margin; `None` takes KDP's minimum for the page count.
    pub gutter_pt: Option<f64>,
    /// Raster images must reach this resolution at their printed size.
    pub min_ppi: f64,
    /// Hold the pages to a black-ink interior even when the document doesn't
    /// say it is one (`print.ink`).
    pub black_ink: bool,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self { safe_pt: 18.0, gutter_pt: None, min_ppi: 300.0, black_ink: false }
    }
}

/// KDP's minimum inside margin for a page count, in points.
pub fn kdp_gutter(pages: usize) -> f64 {
    let inches = match pages {
        0..=150 => 0.375,
        151..=300 => 0.5,
        301..=500 => 0.625,
        501..=700 => 0.75,
        _ => 0.875,
    };
    inches * 72.0
}

pub fn check(s: &Session, opts: &CheckOptions) -> Vec<Issue> {
    let mut out = document_issues(s);
    out.extend(page_issues(s, opts, &items(s), None));
    out
}

/// What the document as a whole reports: problems generating it and the
/// compiler's warnings.
pub fn document_issues(s: &Session) -> Vec<Issue> {
    let mut out = Vec::new();
    for p in s.problems() {
        let code = if p.starts_with("missing file") { "missing-file" } else { "compiler" };
        out.push(Issue { severity: Severity::Error, code, page: None, node: None, message: p.clone() });
    }
    for w in s.warnings() {
        out.push(Issue { severity: Severity::Warning, code: "compiler", page: None, node: None, message: w.clone() });
    }
    out
}

/// The issues on some pages only, for an editor that checks again just
/// the pages an edit changed. Their neighbours are looked at too, since a
/// widow is known only from the page before.
pub fn check_pages(s: &Session, opts: &CheckOptions, pages: &BTreeSet<usize>) -> Vec<Issue> {
    let around: BTreeSet<usize> = pages.iter().flat_map(|&p| [p.saturating_sub(1), p, p + 1]).filter(|&p| p > 0).collect();
    page_issues(s, opts, &items_on(s, around), Some(pages))
}

/// Issues found in `all` (items of some or every page), kept only for
/// `keep` when given.
fn page_issues(s: &Session, opts: &CheckOptions, all: &[Item], keep: Option<&BTreeSet<usize>>) -> Vec<Issue> {
    let mut out = Vec::new();

    // Frames whose text doesn't fit.
    let intro = s.layout().introspector();
    if let Some(label) = Label::new(PicoStr::intern("mt-overflow")) {
        for elem in intro.query(&typst::foundations::Selector::Label(label)).iter() {
            let value = elem.get_by_name("value").ok();
            let (id, over) = match value {
                Some(typst::foundations::Value::Dict(d)) => (
                    d.get("id").ok().map(|v| v.clone().cast::<typst::foundations::Str>().map(|s| s.to_string()).unwrap_or_default()),
                    d.get("over").ok().and_then(|v| v.clone().cast::<f64>().ok()).unwrap_or(0.0),
                ),
                _ => (None, 0.0),
            };
            let page = elem.location().and_then(|l| typst_layout::PagedIntrospector::page(intro, l)).map(|p| p.get());
            out.push(Issue {
                severity: Severity::Error,
                code: "overflow",
                page,
                node: id.clone(),
                message: format!("the text of {} runs {:.1} pt past its frame", id.as_deref().unwrap_or("a frame"), over),
            });
        }
    }

    let pages = s.layout().pages();
    let gutter = opts.gutter_pt.unwrap_or_else(|| kdp_gutter(pages.len()));
    let mut design_families: BTreeSet<String> = design_families(s);
    let resolved: Vec<String> = design_families.iter().filter_map(|f| s.fonts().resolve(f)).map(|f| f.to_lowercase()).collect();
    design_families.extend(resolved);
    let mut fallback: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut margin_hits: BTreeMap<(usize, String), usize> = BTreeMap::new();
    // Lines per paragraph per page, for widows and orphans.
    let mut lines: BTreeMap<String, BTreeMap<usize, BTreeSet<i64>>> = BTreeMap::new();

    for item in all {
        let decorative = matches!(item.element, Some((_, "decorative")));
        match &item.kind {
            Kind::Text { text, family, missing, baseline, .. } => {
                if *missing {
                    out.push(Issue {
                        severity: Severity::Error,
                        code: "missing-glyph",
                        page: Some(item.page),
                        node: item.node.as_ref().map(|n| n.0.clone()),
                        message: format!("no font has a glyph in \"{}\" ({family})", text.trim()),
                    });
                }
                if !design_families.contains(&family.to_lowercase()) {
                    fallback.entry(family.clone()).or_default().insert(item.page);
                }
                if !decorative && !text.trim().is_empty() {
                    let page = &pages[item.page - 1];
                    let (w, h) = (page.frame.width().to_pt(), page.frame.height().to_pt());
                    let recto = item.page % 2 == 1;
                    let (left_min, right_min) = if recto { (gutter, opts.safe_pt) } else { (opts.safe_pt, gutter) };
                    let r = item.rect;
                    if r.x0 < left_min - 0.5 || r.x1 > w - right_min + 0.5 || r.y0 < opts.safe_pt - 0.5 || r.y1 > h - opts.safe_pt + 0.5 {
                        // The element to move, or else the block that ran into the margin.
                        let id = item.element.as_ref().or(item.node.as_ref()).map(|n| n.0.clone()).unwrap_or_default();
                        *margin_hits.entry((item.page, id)).or_default() += 1;
                    }
                }
                if let Some((id, "p")) = &item.node {
                    lines.entry(id.clone()).or_default().entry(item.page).or_default().insert((baseline * 10.0).round() as i64);
                }
            }
            Kind::Image { px: Some((pw, ph)), w, h, .. } => {
                let ppi = (*pw as f64 / (w / 72.0)).min(*ph as f64 / (h / 72.0));
                if ppi + 0.5 < opts.min_ppi && *w > 1.0 && *h > 1.0 {
                    out.push(Issue {
                        severity: Severity::Error,
                        code: "low-resolution",
                        page: Some(item.page),
                        node: item.node.as_ref().map(|n| n.0.clone()),
                        message: format!("an image prints at {ppi:.0} ppi ({pw}×{ph} px over {:.2}×{:.2} in); {} needed", w / 72.0, h / 72.0, opts.min_ppi),
                    });
                }
            }
            _ => {}
        }
    }
    for ((page, id), n) in margin_hits {
        out.push(Issue {
            severity: Severity::Error,
            code: "text-in-margin",
            page: Some(page),
            node: (!id.is_empty()).then(|| id.clone()),
            message: format!(
                "{n} runs of live text sit closer than KDP allows to the trim ({:.3} in outside, {:.3} in at the spine); mark art as decorative or move it",
                opts.safe_pt / 72.0,
                gutter / 72.0
            ),
        });
    }
    for (family, pages) in fallback {
        out.push(Issue {
            severity: Severity::Warning,
            code: "fallback-font",
            page: pages.iter().next().copied(),
            node: None,
            message: format!("\"{family}\" is used but no style names it (a fallback for missing glyphs?) on pages {}", list(&pages)),
        });
    }
    for (id, per_page) in lines {
        let pages: Vec<(&usize, &BTreeSet<i64>)> = per_page.iter().collect();
        for pair in pages.windows(2) {
            let ((p0, l0), (p1, l1)) = (pair[0], pair[1]);
            if *p1 != p0 + 1 {
                continue;
            }
            if l0.len() == 1 {
                out.push(Issue { severity: Severity::Warning, code: "orphan", page: Some(*p0), node: Some(id.clone()), message: format!("{id} leaves one line at the foot of page {p0}") });
            }
            if l1.len() == 1 {
                out.push(Issue { severity: Severity::Warning, code: "widow", page: Some(*p1), node: Some(id.clone()), message: format!("{id} carries one line to the top of page {p1}") });
            }
        }
    }
    if opts.black_ink || s.document().black_ink() {
        out.extend(colour_issues(all));
    }
    if let Some(keep) = keep {
        out.retain(|i| i.page.is_some_and(|p| keep.contains(&p)));
    }
    out
}

/// On a black-ink interior, whatever is drawn in colour: a printer turns it
/// to grey with tones nobody chose. One issue per element (or node) and page.
fn colour_issues(all: &[Item]) -> Vec<Issue> {
    let mut hits: BTreeMap<(usize, String), String> = BTreeMap::new();
    for item in all {
        let what = match &item.kind {
            Kind::Text { fill, .. } => colour_of(fill).map(|c| format!("text in {c}")),
            Kind::Shape { paints } => paints.iter().find_map(colour_of).map(|c| format!("a shape in {c}")),
            Kind::Image { image, .. } => picture_colour(image)
                .filter(crate::tone::Colour::is_colour)
                .map(|c| format!("a colour picture ({:.0}% of it clearly in colour)", c.share * 100.0)),
        };
        if let Some(what) = what {
            let id = item.element.as_ref().or(item.node.as_ref()).map(|n| n.0.clone()).unwrap_or_default();
            hits.entry((item.page, id)).or_insert(what);
        }
    }
    hits.into_iter()
        .map(|((page, id), what)| Issue {
            severity: Severity::Error,
            code: "color-in-black-ink",
            page: Some(page),
            node: (!id.is_empty()).then(|| id.clone()),
            message: format!("{what} on a black-ink page; give pictures a grey filter (\"filter\": {{\"gray\": true}}) and set type and shapes in greys"),
        })
        .collect()
}

/// A paint's colour as `#rrggbb` when it is not a grey (a gradient's first
/// coloured stop).
fn colour_of(p: &typst::visualize::Paint) -> Option<String> {
    use typst::visualize::Paint;
    let hex = |c: &typst::visualize::Color| {
        let rgb = c.to_rgb();
        crate::tone::is_chromatic(rgb.red, rgb.green, rgb.blue)
            .then(|| format!("#{:02x}{:02x}{:02x}", (rgb.red * 255.0).round() as u8, (rgb.green * 255.0).round() as u8, (rgb.blue * 255.0).round() as u8))
    };
    match p {
        Paint::Solid(c) => hex(c),
        Paint::Gradient(g) => g.stops_ref().iter().find_map(|(c, _)| hex(c)),
        Paint::Tiling(_) => None,
    }
}

/// How much colour a picture holds, measured once per picture: a raster's
/// pixels, or a drawing rendered small.
fn picture_colour(image: &typst::visualize::Image) -> Option<crate::tone::Colour> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<HashMap<u128, crate::tone::Colour>>> = OnceLock::new();
    let key = typst::utils::hash128(image);
    let seen = SEEN.get_or_init(Default::default);
    if let Some(c) = seen.lock().ok()?.get(&key) {
        return Some(*c);
    }
    let colour = match image.kind() {
        typst::visualize::ImageKind::Raster(r) => crate::tone::colour(r.dynamic()),
        _ => {
            // A drawing: rendered with its longest side about 256 pixels.
            use typst::layout::{Abs, Frame, FrameItem, Point, Size};
            let longest = image.width().max(image.height()).max(1.0);
            let k = 256.0 / longest;
            let size = Size::new(Abs::pt(image.width() * k), Abs::pt(image.height() * k));
            let mut frame = Frame::hard(size);
            frame.push(Point::zero(), FrameItem::Image(image.clone(), size, typst::syntax::Span::detached()));
            let page = typst_layout::Page {
                frame,
                bleed: Default::default(),
                fill: typst::foundations::Smart::Custom(None),
                numbering: None,
                supplement: Default::default(),
                number: 1,
            };
            let px = typst_render::render(&page, &typst_render::RenderOptions { pixel_per_pt: typst::utils::Scalar::new(1.0), render_bleed: false });
            let img = image::RgbaImage::from_raw(px.width(), px.height(), px.data().to_vec())?;
            crate::tone::colour(&image::DynamicImage::ImageRgba8(img))
        }
    };
    if let Ok(mut s) = seen.lock() {
        if s.len() > 4096 {
            s.clear();
        }
        s.insert(key, colour);
    }
    Some(colour)
}

fn list(pages: &BTreeSet<usize>) -> String {
    let v: Vec<String> = pages.iter().take(12).map(|p| p.to_string()).collect();
    if pages.len() > 12 { format!("{}…", v.join(", ")) } else { v.join(", ") }
}

fn design_families(s: &Session) -> BTreeSet<String> {
    let d = &s.document().design;
    let mut out = BTreeSet::new();
    let mut add = |t: &matenadar_model::TextStyle| {
        if let Some(f) = &t.font {
            for fam in f.families() {
                match d.fonts.get(fam) {
                    Some(named) => out.extend(named.families().into_iter().map(|x| x.to_lowercase())),
                    None => {
                        out.insert(fam.to_lowercase());
                    }
                }
            }
        }
    };
    add(&d.base.text);
    for st in d.styles.values() {
        add(&st.text);
    }
    for sp in d.spans.values() {
        add(sp);
    }
    for stack in d.fonts.values() {
        for f in stack.families() {
            out.insert(f.to_lowercase());
        }
    }
    // Inline overrides anywhere in the document.
    let text = serde_json::to_string(s.document()).unwrap_or_default();
    for (i, _) in text.match_indices("\"font\":") {
        let rest = &text[i + 7..];
        if let Some(v) = rest.strip_prefix('"').and_then(|r| r.split('"').next()) {
            out.insert(v.to_lowercase());
        } else if let Some(arr) = rest.strip_prefix('[').and_then(|r| r.split(']').next()) {
            for v in arr.split(',') {
                out.insert(v.trim().trim_matches('"').to_lowercase());
            }
        }
    }
    out
}
