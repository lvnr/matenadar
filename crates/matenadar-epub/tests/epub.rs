//! A small book through both layouts; epubcheck judges it when installed.

use std::path::PathBuf;

use matenadar_epub::{Layout, Options, build};
use matenadar_model::Document;
use matenadar_typst::Paths;
use serde_json::json;

fn doc() -> Document {
    let v = json!({
        "matenadar": "1",
        "meta": {"title": "A Small Book", "author": "Test", "lang": "en"},
        "design": {
            "base": {"font": "Libertinus Serif", "size": 10, "line_height": 1.4},
            "styles": {"body": {"align": "justify", "indent": "1em"}, "h1": {"size": 18, "space_after": 12}},
            "spans": {"key": {"weight": 700, "tracking": "0.05em"}},
            "masters": {"m": {"width": "5in", "height": "7in", "margins": {"top": 36, "bottom": 36, "inside": 36, "outside": 36}, "style": "body"}}
        },
        "content": [
            {"type": "page", "id": "cover", "master": "m", "elements": [{"type": "text", "x": 36, "y": 200, "w": 288, "text": "A Small Book"}]},
            {"type": "section", "id": "one", "master": "m", "title": "One", "flow": [
                {"type": "h", "level": 1, "id": "h1", "text": "Chapter One"},
                {"type": "p", "content": ["A paragraph with a ", {"text": "key", "span": "key"}, " word and a note", {"note": ["The note."]}, "."]},
                {"type": "p", "text": "A second paragraph & more <text>."},
                {"type": "list", "ordered": true, "items": ["first", "second"]},
                {"type": "table", "columns": ["auto", "1fr"], "header": [["A", "B"]], "rows": [["1", "one"], ["2", "two"]]},
                {"type": "canvas", "h": 40, "elements": [{"type": "rect", "x": 0, "y": 0, "w": 100, "h": 40, "fill": "#cc3333"}]},
                {"type": "p", "content": ["See ", {"ref": "h1", "show": "title"}, "."]}
            ]}
        ]
    });
    Document::from_json(&v.to_string()).expect("document")
}

fn paths() -> Paths {
    Paths { root: PathBuf::from(env!("CARGO_MANIFEST_DIR")), font_dirs: vec![], system_fonts: false }
}

fn check_with_epubcheck(bytes: &[u8], name: &str) {
    let Ok(status) = std::process::Command::new("epubcheck").arg("--version").output() else { return };
    if !status.status.success() {
        return;
    }
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, bytes).unwrap();
    let out = std::process::Command::new("epubcheck").arg(&path).output().expect("epubcheck runs");
    let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{text}");
}

#[test]
fn a_reflowable_book_is_a_valid_epub() {
    let bytes = build(&doc(), &paths(), &Options::default()).expect("epub");
    assert_eq!(&bytes[0..4], b"PK\x03\x04");
    // The mimetype comes first, stored, so readers can sniff it.
    assert_eq!(&bytes[30..38], b"mimetype");
    assert_eq!(&bytes[38..58], b"application/epub+zip");
    check_with_epubcheck(&bytes, "matenadar-reflowable.epub");
}

#[test]
fn a_fixed_layout_book_is_a_valid_epub() {
    let opts = Options { layout: Layout::Fixed, dpi: 72.0, ..Options::default() };
    let bytes = build(&doc(), &paths(), &opts).expect("epub");
    check_with_epubcheck(&bytes, "matenadar-fixed.epub");
}

/// The text of every XHTML file in an EPUB (needs `unzip`; empty without it).
fn xhtml_text(bytes: &[u8], name: &str) -> String {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, bytes).unwrap();
    std::process::Command::new("unzip")
        .args(["-p", path.to_str().unwrap(), "OEBPS/*.xhtml"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

#[test]
fn numbered_figures_their_list_and_a_spread_reach_the_ebook() {
    let dir = std::env::temp_dir().join("matenadar-epub-figures");
    std::fs::create_dir_all(dir.join("art")).unwrap();
    let mut png = Vec::new();
    let img = image::RgbImage::from_fn(60, 40, |x, _| if x < 30 { image::Rgb([200, 40, 40]) } else { image::Rgb([40, 60, 200]) });
    image::DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    std::fs::write(dir.join("art/pic.png"), &png).unwrap();
    let v = json!({
        "matenadar": "1",
        "meta": {"title": "Figures", "author": "Test", "lang": "en"},
        "design": {
            "base": {"font": "Libertinus Serif", "size": 10, "line_height": 1.4},
            "figures": {"label": "Fig.", "per": "chapter"},
            "masters": {"m": {"width": "5in", "height": "7in", "bleed": "0.125in", "margins": {"top": 36, "bottom": 36, "inside": 36, "outside": 36}}}
        },
        "content": [
            {"type": "section", "id": "front", "master": "m", "flow": [{"type": "lof"}]},
            {"type": "spread", "id": "sp", "master": "m", "src": "art/pic.png", "alt": "Red and blue across the fold."},
            {"type": "section", "id": "two", "master": "m", "title": "Two", "vars": {"chapter": "2"}, "flow": [
                {"type": "figure", "id": "f1", "src": "art/pic.png", "caption": "A picture.", "alt": "Red beside blue."},
                {"type": "p", "content": ["As ", {"ref": "f1", "show": "label"}, " shows."]}
            ]}
        ]
    });
    let doc = Document::from_json(&v.to_string()).expect("document");
    let paths = Paths { root: dir, font_dirs: vec![], system_fonts: false };
    let bytes = build(&doc, &paths, &Options { dpi: 72.0, ..Options::default() }).expect("epub");
    check_with_epubcheck(&bytes, "matenadar-figures.epub");
    let text = xhtml_text(&bytes, "matenadar-figures.epub");
    if !text.is_empty() {
        assert!(text.contains("Fig.\u{a0}2.1"), "the caption carries the number: {text}");
        assert!(text.contains("2.1 A picture"), "the list of figures: {text}");
        assert!(text.contains("As <a") && text.contains("Fig.\u{a0}2.1</a> shows"), "the reference: {text}");
        assert!(text.contains("Red and blue across the fold."), "the spread, with its words for readers who can't see it: {text}");
    }
}

/// A file of an EPUB (needs `unzip`; none without it).
fn file_in(bytes: &[u8], name: &str, inner: &str) -> Option<Vec<u8>> {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, bytes).unwrap();
    let out = std::process::Command::new("unzip").args(["-p", path.to_str().unwrap(), inner]).output().ok()?;
    (out.status.success() && !out.stdout.is_empty()).then_some(out.stdout)
}

#[test]
fn a_fixed_layout_page_sets_its_words_as_live_text_over_its_picture() {
    let opts = Options { layout: Layout::Fixed, dpi: 72.0, modified: Some("2026-10-31T09:00:00Z".into()), ..Options::default() };
    let bytes = build(&doc(), &paths(), &opts).expect("epub");
    check_with_epubcheck(&bytes, "matenadar-fixed-live.epub");
    let Some(page) = file_in(&bytes, "matenadar-fixed-live.epub", "OEBPS/page-001.xhtml") else { return };
    let page = String::from_utf8(page).unwrap();
    // The cover's words, as SVG text in an embedded face, over the page's picture.
    assert!(page.contains("<svg") && page.contains("<image"), "{page}");
    assert!(page.contains(">Small</tspan>"), "each word a span: {page}");
    let face = page.split("font-family=\"").nth(1).and_then(|s| s.split('"').next()).expect("a face");
    let css = String::from_utf8(file_in(&bytes, "matenadar-fixed-live.epub", "OEBPS/style.css").unwrap()).unwrap();
    assert!(css.contains(&format!("font-family: \"{face}\"")), "the face is in the book: {css}");
    let opf = String::from_utf8(file_in(&bytes, "matenadar-fixed-live.epub", "OEBPS/package.opf").unwrap()).unwrap();
    assert!(opf.contains("2026-10-31T09:00:00Z") && opf.contains("properties=\"svg\""));
    // The picture leaves the words out: where the cover's title sits, it is paper.
    let href = page.split("xlink:href=\"").nth(1).and_then(|s| s.split('"').next()).expect("the picture");
    let jpg = file_in(&bytes, "matenadar-fixed-live.epub", &format!("OEBPS/{href}")).expect("the picture's file");
    let img = image::load_from_memory(&jpg).unwrap().to_luma8();
    let darkest = (36..120).flat_map(|x| (200..214).map(move |y| (x, y))).map(|(x, y)| img.get_pixel(x, y).0[0]).min().unwrap();
    assert!(darkest > 200, "no words drawn in the picture (darkest {darkest})");

    // Without live text the page is one picture, its words in it.
    let opts = Options { live_text: false, ..opts };
    let bytes = build(&doc(), &paths(), &opts).expect("epub");
    let page = String::from_utf8(file_in(&bytes, "matenadar-fixed-pictures.epub", "OEBPS/page-001.xhtml").unwrap()).unwrap();
    assert!(page.contains("<img") && !page.contains("<tspan"));
}
