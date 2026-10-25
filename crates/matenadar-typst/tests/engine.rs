//! End-to-end checks of the engine on small documents.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use matenadar_model::{Document, Op};
use matenadar_model::ops::Target;
use matenadar_typst::{CheckOptions, Fonts, Paths, Session, check, inspect};
use serde_json::json;

fn fonts() -> Arc<Fonts> {
    static FONTS: OnceLock<Arc<Fonts>> = OnceLock::new();
    FONTS.get_or_init(|| Arc::new(Fonts::new(&[], false))).clone()
}

fn session(v: serde_json::Value) -> Session {
    let doc = Document::from_json(&v.to_string()).expect("document");
    assert_eq!(matenadar_model::validate(&doc), vec![], "the test document is valid");
    let paths = Paths { root: PathBuf::from(env!("CARGO_MANIFEST_DIR")), font_dirs: vec![], system_fonts: false };
    Session::with_fonts(doc, paths, fonts()).expect("layout")
}

fn book(flow: serde_json::Value, extra: serde_json::Value) -> serde_json::Value {
    let mut content = vec![json!({"type": "page", "id": "title", "master": "body", "elements": [
        {"type": "text", "id": "t", "x": 72, "y": 200, "w": 288, "text": "Title"}
    ]})];
    content.push(json!({"type": "section", "id": "ch1", "master": "body", "start": "recto", "title": "One", "flow": flow}));
    if let serde_json::Value::Array(more) = extra {
        content.extend(more);
    }
    json!({
        "matenadar": "1",
        "meta": {"title": "Test", "lang": "en"},
        "design": {
            "base": {"font": "Libertinus Serif", "size": 10, "line_height": 1.4},
            "styles": {"body": {"align": "justify", "indent": "1em"}, "note": {"size": 7}},
            "masters": {"body": {
                "width": "6in", "height": "9in", "bleed": "0.125in",
                "margins": {"top": 72, "bottom": 72, "inside": 72, "outside": 108},
                "notes": {"width": 72, "gap": 12, "style": "note"},
                "foreground": [{"type": "text", "id": "folio", "x": 72, "y": 600, "w": 288, "content": [{"field": "page"}], "when": "not-first not-blank"}]
            }}
        },
        "content": content
    })
}

#[test]
fn a_recto_section_after_a_page_gets_a_blank_verso() {
    let s = session(book(json!([{"type": "p", "id": "p1", "text": "Hello."}]), json!([])));
    assert_eq!(s.page_count(), 3);
    assert_eq!(s.position("p1").map(|p| p.0), Some(3), "the section opens on the right-hand page 3");
    let report = inspect::report(&s);
    assert!(report[1].nodes.is_empty(), "the blank verso carries nothing, not even the folio: {:?}", report[1].nodes);
}

#[test]
fn margin_notes_sit_in_the_outer_column_level_with_their_block() {
    let s = session(book(
        json!([
            {"type": "p", "id": "p1", "text": "A paragraph that has a note beside it."},
            {"type": "margin", "id": "n1", "text": "Note."}
        ]),
        json!([]),
    ));
    let report = inspect::report(&s);
    let page = &report[2];
    let p1 = page.nodes.iter().find(|n| n.id == "p1").expect("p1 placed");
    let n1 = page.nodes.iter().find(|n| n.id == "n1").expect("n1 placed");
    assert!(n1.rect.x0 > p1.rect.x1, "the note is right of the text on a recto: {n1:?} vs {p1:?}");
    assert!((n1.rect.y0 - p1.rect.y0).abs() < 4.0, "level with the paragraph: {n1:?} vs {p1:?}");
}

#[test]
fn page_references_resolve_after_layout() {
    let s = session(book(
        json!([
            {"type": "p", "id": "p1", "content": ["See page ", {"ref": "target"}, "."]},
            {"type": "page-break"},
            {"type": "p", "id": "target", "text": "Here."}
        ]),
        json!([]),
    ));
    let report = inspect::report(&s);
    assert_eq!(s.position("target").map(|p| p.0), Some(4));
    assert!(!report.is_empty());
}

#[test]
fn frames_report_overflow_and_shrink_to_fit() {
    let long = "word ".repeat(200);
    let s = session(book(
        json!([{"type": "p", "id": "p1", "text": "x"}]),
        json!([{"type": "page", "id": "pg", "master": "body", "elements": [
            {"type": "text", "id": "over", "x": 72, "y": 72, "w": 144, "h": 40, "text": long},
            {"type": "text", "id": "fits", "x": 72, "y": 200, "w": 144, "h": 300, "fit": "shrink", "min_size": 2, "text": long}
        ]}]),
    ));
    let issues = check(&s, &CheckOptions::default());
    let over: Vec<_> = issues.iter().filter(|i| i.code == "overflow").collect();
    assert_eq!(over.len(), 1, "{issues:?}");
    assert_eq!(over[0].node.as_deref(), Some("over"));
}

#[test]
fn text_near_the_trim_is_reported_unless_decorative() {
    let s = session(book(
        json!([{"type": "p", "id": "p1", "text": "x"}]),
        json!([{"type": "page", "id": "pg", "master": "body", "elements": [
            {"type": "text", "id": "edge", "x": 2, "y": 300, "w": 100, "text": "Too close"},
            {"type": "text", "id": "art", "x": 2, "y": 400, "w": 100, "text": "Art", "decorative": true}
        ]}]),
    ));
    let issues = check(&s, &CheckOptions::default());
    let hits: Vec<_> = issues.iter().filter(|i| i.code == "text-in-margin").filter_map(|i| i.node.clone()).collect();
    assert!(hits.contains(&"edge".to_string()), "{issues:?}");
    assert!(!hits.contains(&"art".to_string()), "{issues:?}");
}

#[test]
fn an_edit_relays_only_what_changed_and_undoes() {
    let paras: Vec<_> = (0..60)
        .map(|i| json!({"type": "p", "id": format!("p{i}"), "text": "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ".repeat(3)}))
        .collect();
    let mut s = session(book(json!(paras), json!([])));
    let original = s.document().clone();
    let pages = s.page_count();
    assert!(pages > 6, "{pages} pages");
    let change = s.apply(&Op::InsertText { node: Target::id("p58"), offset: 0, text: "Yes. ".into() }).unwrap();
    assert!(change.changed_pages.iter().all(|p| *p >= pages - 2), "only the last pages change: {:?}", change.changed_pages);
    let undo = s.apply(&change.inverse).unwrap();
    assert!(matches!(undo.inverse, Op::InsertText { .. }));
    assert_eq!(&original, s.document(), "undo restores the document");
    let located = s.locate(s.position("p10").unwrap().0, s.position("p10").unwrap().1 + 20.0, s.position("p10").unwrap().2 + 6.0);
    assert_eq!(located.map(|l| l.0), Some("p10".to_string()));
}

#[test]
fn bleed_puts_the_trim_box_inside_the_media_box() {
    let s = session(book(json!([{"type": "p", "id": "p1", "text": "x"}]), json!([])));
    let pdf = s.pdf().unwrap();
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/TrimBox"), "the PDF marks the trim");
}
