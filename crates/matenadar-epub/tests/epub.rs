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
