//! EPUB 3 from a Matenadar document.
//!
//! - **Reflowable** (the default): each section becomes a chapter of XHTML,
//!   its blocks as HTML and the design's styles as CSS, so readers can
//!   reflow and resize the text. Hand-laid pages and drawings (canvases,
//!   components made of elements) become images, rendered by the engine.
//! - **Fixed layout**: every page as laid out for print, one image per page,
//!   for picture books and other page-led books.

mod css;
mod xhtml;
mod zip;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use matenadar_model::{Document, Item};
use matenadar_typst::{Fonts, Paths, Session};

pub use css::class;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Reflowable,
    Fixed,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub layout: Layout,
    /// Resolution of page and drawing images.
    pub dpi: f32,
    /// Put the fonts in the book (only for fonts licensed for embedding).
    pub embed_fonts: bool,
    /// A cover image; otherwise the first hand-laid page, if the book opens with one.
    pub cover: Option<PathBuf>,
    /// The book's identifier (an ISBN as `urn:isbn:…`); otherwise one made from the title.
    pub identifier: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Options { layout: Layout::Reflowable, dpi: 200.0, embed_fonts: false, cover: None, identifier: None }
    }
}

/// A file in the book.
pub(crate) struct Asset {
    pub href: String,
    pub media: &'static str,
    pub data: Vec<u8>,
    pub properties: Option<&'static str>,
}

/// Files gathered while writing, by href.
#[derive(Default)]
pub(crate) struct Assets {
    pub list: Vec<Asset>,
    /// Source path (or key) to href, so a file is added once.
    pub seen: HashMap<String, String>,
}

impl Assets {
    pub fn add(&mut self, key: &str, stem: &str, ext: &str, data: Vec<u8>) -> String {
        if let Some(h) = self.seen.get(key) {
            return h.clone();
        }
        let href = format!("{stem}{}.{ext}", self.list.len() + 1);
        self.list.push(Asset { href: href.clone(), media: media_type(ext), data, properties: None });
        self.seen.insert(key.to_string(), href.clone());
        href
    }
}

pub(crate) fn media_type(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "css" => "text/css",
        _ => "application/octet-stream",
    }
}

/// One XHTML file of the book.
pub(crate) struct Chapter {
    pub file: String,
    pub title: String,
    pub body: String,
    /// Headings inside: (level, anchor, text).
    pub headings: Vec<(u8, String, String)>,
    /// For fixed layout: the page's size in pixels.
    pub viewport: Option<(u32, u32)>,
    pub spread: Option<&'static str>,
}

/// The EPUB file for `doc`.
pub fn build(doc: &Document, paths: &Paths, opts: &Options) -> Result<Vec<u8>, String> {
    let fonts = Arc::new(paths.fonts_for(doc));
    let session = Session::with_fonts(doc.clone(), paths.clone(), fonts.clone()).map_err(|e| e.to_string())?;
    let mut assets = Assets::default();
    let css = css::Css::new(doc);
    let mut chapters = Vec::new();
    let mut cover: Option<String> = None;
    if let Some(c) = &opts.cover {
        let data = std::fs::read(c).map_err(|e| format!("cover {}: {e}", c.display()))?;
        let ext = c.extension().and_then(|e| e.to_str()).unwrap_or("jpg").to_string();
        cover = Some(assets.add("cover", "images/cover", &ext, data));
    }

    match opts.layout {
        Layout::Fixed => {
            for page in 1..=session.page_count() {
                let png = session.png(page, opts.dpi, false).ok_or("a page could not be drawn")?;
                let (data, (w, h)) = jpeg(&png)?;
                let href = assets.add(&format!("page:{page}"), "images/page", "jpg", data);
                if cover.is_none() {
                    cover = Some(href.clone());
                }
                let text = page_text(&session, page);
                chapters.push(Chapter {
                    file: format!("page-{page:03}.xhtml"),
                    title: format!("Page {page}"),
                    body: format!(
                        "<div class=\"page\"><img src=\"{href}\" alt=\"{}\" style=\"width: {w}px; height: {h}px\"/></div>",
                        xhtml::esc(&text)
                    ),
                    headings: Vec::new(),
                    viewport: Some((w, h)),
                    spread: Some(if page % 2 == 1 { "page-spread-right" } else { "page-spread-left" }),
                });
            }
        }
        Layout::Reflowable => {
            let mut w = xhtml::Writer::new(doc, &css, &session, paths, fonts.clone(), opts.dpi);
            for (i, item) in doc.content.iter().enumerate() {
                match item {
                    Item::Section(s) => {
                        let file = format!("{}.xhtml", file_stem("s", &s.id, i));
                        let (body, headings) = w.section(s, &file, &mut assets);
                        let title = s.title.clone().or_else(|| headings.first().map(|h| h.2.clone())).unwrap_or_else(|| format!("Section {}", i + 1));
                        chapters.push(Chapter { file, title, body, headings, viewport: None, spread: None });
                    }
                    Item::Spread(sp) => {
                        // Both pages as one wide picture, as the reader sees them open.
                        let Some((page, _, _)) = session.position(&sp.id) else { continue };
                        let (Some(left), Some(right)) = (session.png(page, opts.dpi, false), session.png(page + 1, opts.dpi, false)) else { continue };
                        let data = side_by_side(&left, &right)?;
                        let href = assets.add(&format!("spread:{page}"), "images/spread", "jpg", data);
                        let alt = sp.alt.clone().unwrap_or_else(|| format!("{} {}", page_text(&session, page), page_text(&session, page + 1)).trim().to_string());
                        chapters.push(Chapter {
                            file: format!("{}.xhtml", file_stem("p", &sp.id, i)),
                            title: sp.title.clone().unwrap_or_default(),
                            body: format!("<div class=\"page spread\"><img src=\"{href}\" alt=\"{}\"/></div>", xhtml::esc(&alt)),
                            headings: Vec::new(),
                            viewport: None,
                            spread: None,
                        });
                    }
                    Item::Page(p) => {
                        let Some((first, _, _)) = session.position(&p.id) else { continue };
                        // A repeated page is laid out once per time, one after another.
                        let times = p.repeat.as_ref().map_or(1, |r| r.times());
                        for k in 0..times {
                            let page = first + k;
                            let png = session.png(page, opts.dpi, false).ok_or("a page could not be drawn")?;
                            let (data, _) = jpeg(&png)?;
                            let href = assets.add(&format!("page:{page}"), "images/page", "jpg", data);
                            if i == 0 && k == 0 && cover.is_none() {
                                cover = Some(href.clone());
                            }
                            let text = page_text(&session, page);
                            let stem = file_stem("p", &p.id, i);
                            chapters.push(Chapter {
                                file: if k == 0 { format!("{stem}.xhtml") } else { format!("{stem}-{}.xhtml", k + 1) },
                                title: p.title.clone().unwrap_or_default(),
                                body: format!("<div class=\"page\"><img src=\"{href}\" alt=\"{}\"/></div>", xhtml::esc(&text)),
                                headings: Vec::new(),
                                viewport: None,
                                spread: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Fonts, when asked for.
    let mut font_css = String::new();
    if opts.embed_fonts {
        font_css = embed_fonts(doc, &fonts, &mut assets);
    }
    let stylesheet = format!(
        "@charset \"utf-8\";\n@namespace epub \"http://www.idpf.org/2007/ops\";\n{font_css}{}",
        css.sheet(&xhtml::drop_cap_rules(doc, &css))
    );
    if let Some(c) = &cover
        && let Some(a) = assets.list.iter_mut().find(|a| &a.href == c)
    {
        a.properties = Some("cover-image");
    }
    Ok(package(doc, opts, &chapters, &assets, &stylesheet, cover.as_deref()))
}

/// A page's text, for an image's description.
fn page_text(s: &Session, page: usize) -> String {
    let mut text = String::new();
    for item in matenadar_typst::inspect::items(s) {
        if item.page == page
            && let matenadar_typst::inspect::Kind::Text { text: t, .. } = &item.kind
        {
            if !text.is_empty() && !text.ends_with(' ') {
                text.push(' ');
            }
            text.push_str(t.trim());
        }
    }
    let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut short: String = text.chars().take(600).collect();
    if short.len() < text.len() {
        short.push('…');
    }
    short
}

/// Two page pictures (PNG) side by side, as one JPEG.
fn side_by_side(left: &[u8], right: &[u8]) -> Result<Vec<u8>, String> {
    let l = image::load_from_memory(left).map_err(|e| e.to_string())?.to_rgb8();
    let r = image::load_from_memory(right).map_err(|e| e.to_string())?.to_rgb8();
    let mut both = image::RgbImage::from_pixel(l.width() + r.width(), l.height().max(r.height()), image::Rgb([255, 255, 255]));
    image::imageops::replace(&mut both, &l, 0, 0);
    image::imageops::replace(&mut both, &r, l.width() as i64, 0);
    let mut out = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
    both.write_with_encoder(enc).map_err(|e| e.to_string())?;
    Ok(out)
}

/// A PNG as JPEG, and its size in pixels.
pub(crate) fn jpeg(png: &[u8]) -> Result<(Vec<u8>, (u32, u32)), String> {
    let img = image::load_from_memory(png).map_err(|e| e.to_string())?.to_rgb8();
    let size = (img.width(), img.height());
    let mut out = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
    img.write_with_encoder(enc).map_err(|e| e.to_string())?;
    Ok((out, size))
}

/// A file name from a prefix, an id and a position (ids may hold any letters).
fn file_stem(prefix: &str, id: &str, i: usize) -> String {
    let safe: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(40).collect();
    format!("{prefix}{:03}-{safe}", i + 1)
}

fn embed_fonts(doc: &Document, fonts: &Fonts, assets: &mut Assets) -> String {
    let d = &doc.design;
    // Each family with the weights and styles the design asks for, plus
    // regular, bold and italic for emphasis in runs.
    let mut wanted: Vec<(String, u16, bool)> = Vec::new();
    let mut want = |t: &matenadar_model::TextStyle, base: &matenadar_model::TextStyle| {
        let stack = t.font.as_ref().or(base.font.as_ref());
        let weight = t.weight.as_ref().or(base.weight.as_ref()).map(matenadar_typst::glyphs::weight_number).unwrap_or(400);
        let italic = t.italic.or(base.italic).unwrap_or(false);
        for f in stack.iter().flat_map(|s| s.families()) {
            let named: Vec<String> = match d.fonts.get(f) {
                Some(n) => n.families().into_iter().map(str::to_string).collect(),
                None => vec![f.to_string()],
            };
            for n in named {
                for combo in [(n.clone(), weight, italic), (n.clone(), 400, false), (n.clone(), 700, false), (n.clone(), 400, true)] {
                    if !wanted.contains(&combo) {
                        wanted.push(combo);
                    }
                }
            }
        }
    };
    let base = d.style("").text;
    want(&base, &base);
    for name in d.styles.keys() {
        want(&d.style(name).text, &base);
    }
    for s in d.spans.values() {
        want(s, &base);
    }
    let mut css = String::new();
    // One rule per file and style, with the weights it serves.
    let mut rules: BTreeMap<(String, usize, bool), (Vec<u16>, String)> = BTreeMap::new();
    for (family, weight, italic) in wanted {
        let Some(font) = fonts.select(&family, weight, italic) else { continue };
        if font.index() != 0 || !font.info().family.eq_ignore_ascii_case(&family) {
            continue;
        }
        let data = font.data();
        let key = data.as_slice().as_ptr() as usize;
        let ext = if data.as_slice().starts_with(b"OTTO") { "otf" } else { "ttf" };
        let href = assets.add(&format!("font:{key}"), "fonts/font", ext, data.as_slice().to_vec());
        // A static font serves its own weight; a variable one, every weight asked of it.
        let own = font.info().variant.weight.to_number();
        let rule = rules.entry((family.clone(), key, italic)).or_insert((Vec::new(), href));
        rule.0.push(if font.info().flags.contains(typst::text::FontFlags::VARIABLE) { weight } else { own });
    }
    for ((family, _, italic), (weights, href)) in rules {
        let (lo, hi) = (weights.iter().min().copied().unwrap_or(400), weights.iter().max().copied().unwrap_or(400));
        let weight = if lo == hi { lo.to_string() } else { format!("{lo} {hi}") };
        css.push_str(&format!(
            "@font-face {{ font-family: \"{family}\"; src: url(\"{href}\"); font-weight: {weight}; font-style: {}; }}\n",
            if italic { "italic" } else { "normal" }
        ));
    }
    css
}

fn package(doc: &Document, opts: &Options, chapters: &[Chapter], assets: &Assets, stylesheet: &str, cover: Option<&str>) -> Vec<u8> {
    let lang = doc.meta.lang.clone().unwrap_or_else(|| "en".into());
    let title = if doc.meta.title.trim().is_empty() { "Untitled".to_string() } else { doc.meta.title.clone() };
    let fixed = opts.layout == Layout::Fixed;
    let mut zip = zip::Zip::new();
    zip.stored("mimetype", b"application/epub+zip");
    zip.deflated(
        "META-INF/container.xml",
        br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/package.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>
"#,
    );
    zip.deflated("OEBPS/style.css", stylesheet.as_bytes());

    for c in chapters {
        let head = match c.viewport {
            Some((w, h)) => format!("<meta name=\"viewport\" content=\"width={w}, height={h}\"/>\n<style>body {{ margin: 0 }}</style>\n"),
            None => String::new(),
        };
        let page = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"{lang}\" lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\"/>\n<title>{}</title>\n<link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/>\n{head}</head>\n<body>\n{}\n</body>\n</html>\n",
            xhtml::esc(if c.title.is_empty() { &title } else { &c.title }),
            c.body
        );
        zip.deflated(&format!("OEBPS/{}", c.file), page.as_bytes());
    }

    // The contents.
    let mut nav = String::new();
    for c in chapters.iter().filter(|c| !c.title.is_empty()) {
        nav.push_str(&format!("<li><a href=\"{}\">{}</a>", c.file, xhtml::esc(&c.title)));
        let subs: Vec<_> = c.headings.iter().filter(|h| h.0 == 2).collect();
        if !subs.is_empty() {
            nav.push_str("<ol>");
            for (_, anchor, text) in subs {
                nav.push_str(&format!("<li><a href=\"{}#{}\">{}</a></li>", c.file, xhtml::esc(anchor), xhtml::esc(text)));
            }
            nav.push_str("</ol>");
        }
        nav.push_str("</li>\n");
    }
    if nav.is_empty() {
        nav = format!("<li><a href=\"{}\">{}</a></li>\n", chapters.first().map(|c| c.file.as_str()).unwrap_or(""), xhtml::esc(&title));
    }
    let nav_doc = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"{lang}\" lang=\"{lang}\">\n<head><meta charset=\"utf-8\"/><title>{t}</title><link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/></head>\n<body>\n<nav epub:type=\"toc\" id=\"toc\"><h1>{t}</h1>\n<ol>\n{nav}</ol></nav>\n</body>\n</html>\n",
        t = xhtml::esc(&title)
    );
    zip.deflated("OEBPS/nav.xhtml", nav_doc.as_bytes());

    for a in &assets.list {
        if a.media.starts_with("image/jpeg") || a.media.starts_with("image/png") || a.media.starts_with("font/") {
            zip.stored(&format!("OEBPS/{}", a.href), &a.data);
        } else {
            zip.deflated(&format!("OEBPS/{}", a.href), &a.data);
        }
    }

    let mut manifest = String::from("<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n<item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>\n");
    let mut spine = String::new();
    for (i, c) in chapters.iter().enumerate() {
        let svg = c.body.contains("<svg");
        manifest.push_str(&format!(
            "<item id=\"c{}\" href=\"{}\" media-type=\"application/xhtml+xml\"{}/>\n",
            i + 1,
            c.file,
            if svg { " properties=\"svg\"" } else { "" }
        ));
        spine.push_str(&format!("<itemref idref=\"c{}\"{}/>\n", i + 1, c.spread.map(|p| format!(" properties=\"{p}\"")).unwrap_or_default()));
    }
    for (i, a) in assets.list.iter().enumerate() {
        manifest.push_str(&format!(
            "<item id=\"a{}\" href=\"{}\" media-type=\"{}\"{}/>\n",
            i + 1,
            a.href,
            a.media,
            a.properties.map(|p| format!(" properties=\"{p}\"")).unwrap_or_default()
        ));
    }
    let identifier = opts.identifier.clone().unwrap_or_else(|| uuid_from(&format!("{}\u{0}{}", title, doc.meta.author.clone().unwrap_or_default())));
    let cover_meta = cover
        .and_then(|c| assets.list.iter().position(|a| a.href == c))
        .map(|i| format!("<meta name=\"cover\" content=\"a{}\"/>\n", i + 1))
        .unwrap_or_default();
    let fixed_meta = if fixed {
        "<meta property=\"rendition:layout\">pre-paginated</meta>\n<meta property=\"rendition:spread\">auto</meta>\n<meta property=\"rendition:orientation\">auto</meta>\n"
    } else {
        ""
    };
    let author = doc.meta.author.as_ref().map(|a| format!("<dc:creator>{}</dc:creator>\n", xhtml::esc(a))).unwrap_or_default();
    let subject = doc.meta.subject.as_ref().map(|s| format!("<dc:subject>{}</dc:subject>\n", xhtml::esc(s))).unwrap_or_default();
    let opf = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"bookid\" xml:lang=\"{lang}\"{prefix}>\n<metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n<dc:identifier id=\"bookid\">{id}</dc:identifier>\n<dc:title>{title}</dc:title>\n<dc:language>{lang}</dc:language>\n{author}{subject}<meta property=\"dcterms:modified\">{modified}</meta>\n{fixed_meta}{cover_meta}</metadata>\n<manifest>\n{manifest}</manifest>\n<spine>\n{spine}</spine>\n</package>\n",
        prefix = if fixed { " prefix=\"rendition: http://www.idpf.org/vocab/rendition/#\"" } else { "" },
        id = xhtml::esc(&identifier),
        title = xhtml::esc(&title),
        modified = now_utc(),
    );
    zip.deflated("OEBPS/package.opf", opf.as_bytes());
    zip.finish()
}

/// A name-based identifier: the same title and author give the same one.
fn uuid_from(text: &str) -> String {
    let mut h: [u64; 2] = [0xcbf29ce484222325, 0x84222325cbf29ce4];
    for (i, b) in text.bytes().enumerate() {
        let k = i % 2;
        h[k] ^= b as u64;
        h[k] = h[k].wrapping_mul(0x100000001b3);
    }
    let hex = format!("{:016x}{:016x}", h[0], h[1]);
    format!("urn:uuid:{}-{}-5{}-a{}-{}", &hex[0..8], &hex[8..12], &hex[13..16], &hex[17..20], &hex[20..32])
}

/// The time now as `CCYY-MM-DDThh:mm:ssZ`.
fn now_utc() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}
