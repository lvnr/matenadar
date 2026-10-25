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

#[test]
fn a_frame_aligned_to_the_bottom_keeps_its_table_cells_on_top() {
    let s = session(book(
        json!([{"type": "p", "id": "p1", "text": "x"}]),
        json!([{"type": "page", "id": "pg", "master": "body", "elements": [
            {"type": "text", "id": "low", "x": 72, "y": 72, "w": 200, "h": 200, "valign": "bottom", "blocks": [
                {"type": "p", "id": "tail", "text": "At the foot."},
                {"type": "table", "id": "tbl", "columns": [60, 140], "rows": [["top", "one two three four five six seven eight nine ten eleven twelve"]]}
            ]}
        ]}]),
    ));
    let report = inspect::report(&s);
    let page = report.iter().find(|p| p.nodes.iter().any(|n| n.id == "tail")).expect("page");
    let tail = page.nodes.iter().find(|n| n.id == "tail").unwrap();
    let tbl = page.nodes.iter().find(|n| n.id == "tbl").unwrap();
    assert!(tbl.rect.y1 > 262.0 && tbl.rect.y1 <= 273.0, "the content ends at the frame's foot: {tbl:?}");
    assert!(tail.rect.y0 > 150.0, "and starts low in the frame: {tail:?}");
}

fn page(elements: serde_json::Value) -> serde_json::Value {
    json!({
        "matenadar": "1",
        "meta": {"title": "Test", "lang": "en"},
        "design": {"base": {"font": "Libertinus Serif", "size": 10, "line_height": 1.4},
                   "masters": {"m": {"width": "6in", "height": "9in", "bleed": "0.125in",
                                     "margins": {"top": 72, "bottom": 72, "inside": 72, "outside": 72},
                                     "notes": {"width": 54, "gap": 8}}}},
        "content": [{"type": "page", "id": "pg", "master": "m", "elements": elements}]
    })
}

fn rect_of(s: &Session, id: &str) -> inspect::Rect {
    inspect::report(s).iter().flat_map(|p| p.nodes.iter()).find(|n| n.id == id).unwrap_or_else(|| panic!("{id} is laid out")).rect
}

/// Every run of text on every page, with its fill.
fn texts(s: &Session) -> Vec<(String, typst::visualize::Paint)> {
    fn walk(frame: &typst::layout::Frame, out: &mut Vec<(String, typst::visualize::Paint)>) {
        for (_, item) in frame.items() {
            match item {
                typst::layout::FrameItem::Group(g) => walk(&g.frame, out),
                typst::layout::FrameItem::Text(t) => out.push((t.text.to_string(), t.fill.clone())),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for p in s.layout().pages() {
        walk(&p.frame, &mut out);
    }
    out
}

#[test]
fn lists_and_code_compile_in_flows_and_frames() {
    let blocks = json!([
        {"type": "list", "id": "l1", "items": ["one", "two"]},
        {"type": "list", "id": "l2", "ordered": true, "marker_shape": {"w": 12, "fill": "#222"}, "items": ["one", "two"]},
        {"type": "code", "id": "c1", "text": "fn main() {}\n  # not markup"}
    ]);
    let s = session(book(blocks.clone(), json!([{"type": "page", "id": "fr", "master": "body", "elements": [
        {"type": "text", "id": "frame", "x": 72, "y": 72, "w": 288, "blocks": [{"type": "list", "id": "l3", "items": ["a", "b"]}]}
    ]}])));
    for id in ["l1", "l2", "c1", "l3"] {
        assert!(rect_of(&s, id).y1 > rect_of(&s, id).y0, "{id} has height");
    }
}

#[test]
fn a_word_split_into_runs_keeps_its_letter_spacing() {
    let s = session(page(json!([
        {"type": "text", "id": "whole", "x": 72, "y": 72, "w": 200, "set": {"tracking": "2pt"}, "text": "caution."},
        {"type": "text", "id": "split", "x": 72, "y": 100, "w": 200, "set": {"tracking": "2pt"},
         "content": [{"text": "c"}, {"text": "au", "color": "#c00"}, "tion", {"text": "."}]}
    ])));
    let (a, b) = (rect_of(&s, "whole"), rect_of(&s, "split"));
    assert!(((a.x1 - a.x0) - (b.x1 - b.x0)).abs() < 0.05, "{a:?} vs {b:?}");
}

#[test]
fn decorative_text_is_drawn_as_outlines_and_still_reported() {
    let s = session(page(json!([
        {"type": "text", "id": "big", "x": 380, "y": -20, "set": {"size": 160}, "text": "7", "decorative": true},
        {"type": "text", "id": "body", "x": 72, "y": 300, "w": 200, "text": "Body text"}
    ])));
    let runs = texts(&s);
    assert!(runs.iter().any(|(t, _)| t.contains("Body")), "{runs:?}");
    assert!(!runs.iter().any(|(t, _)| t.contains('7')), "the numeral is outlines, not text: {runs:?}");
    assert!(rect_of(&s, "big").y1 > 20.0, "outlines keep their node in the layout report");
    let issues = check(&s, &CheckOptions::default());
    assert!(!issues.iter().any(|i| i.code == "text-in-margin"), "{issues:?}");
}

#[test]
fn opacity_fades_text_and_shapes() {
    let s = session(page(json!([
        {"type": "text", "id": "f", "x": 72, "y": 72, "w": 200, "set": {"color": "#000000"}, "text": "Faded", "opacity": 0.5}
    ])));
    let runs = texts(&s);
    let (_, fill) = runs.iter().find(|(t, _)| t.contains("Faded")).expect("the faded run");
    match fill {
        typst::visualize::Paint::Solid(c) => assert!((c.alpha().unwrap_or(1.0) - 0.5).abs() < 0.01, "{c:?}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_pretty_paragraph_keeps_its_last_two_words_together() {
    let text = "aaa bbb ccc ddd eee fff ggg hhh iii jjj kkk lll mmm nnn ooo ppp qqq rrr sss ttt z";
    let run_with = |s: &Session, word: &str| texts(s).into_iter().map(|t| t.0).find(|t| t.contains(word)).unwrap_or_default();
    for w in 60..160 {
        let doc = |wrap: &str| page(json!([{"type": "text", "id": "t", "x": 72, "y": 72, "w": w, "set": {"wrap": wrap}, "text": text}]));
        let plain = session(doc("normal"));
        if run_with(&plain, " z").is_empty() && run_with(&plain, "z") == "z" {
            // Here plain breaking leaves "z" alone on the last line.
            let pretty = session(doc("pretty"));
            let last = run_with(&pretty, "z");
            assert!(last.contains("ttt") && last.ends_with('z'), "width {w}: {last:?}");
            return;
        }
    }
    panic!("no width left the last word alone");
}

#[test]
fn a_document_can_open_on_a_verso() {
    let mut d = page(json!([{"type": "text", "id": "t", "x": 72, "y": 72, "w": 200, "text": "Left"}]));
    d["content"][0]["start"] = json!("verso");
    let s = session(d);
    assert_eq!(s.page_count(), 2);
    assert_eq!(s.position("t").map(|p| p.0), Some(2));
}

#[test]
fn margin_notes_after_one_block_stack_level_with_it() {
    let s = session(book(
        json!([
            {"type": "p", "id": "p1", "text": "A paragraph with two notes beside it."},
            {"type": "margin", "id": "n1", "text": "First."},
            {"type": "margin", "id": "n2", "text": "Second."},
            {"type": "p", "text": "Text enough to clear the notes above. Text enough to clear the notes above. Text enough to clear the notes above. Text enough to clear the notes above. Text enough to clear the notes above."},
            {"type": "box", "id": "bx", "inset": 12, "fill": "#eee", "blocks": [{"type": "p", "text": "A box."}]},
            {"type": "margin", "id": "n3", "text": "Third."}
        ]),
        json!([]),
    ));
    let (p1, n1, n2, bx, n3) = (rect_of(&s, "p1"), rect_of(&s, "n1"), rect_of(&s, "n2"), rect_of(&s, "bx"), rect_of(&s, "n3"));
    assert!((n1.y0 - p1.y0).abs() < 3.0, "{n1:?} beside {p1:?}");
    assert!(n2.y0 > n1.y1, "{n2:?} under {n1:?}");
    assert!((n3.y0 - bx.y0).abs() < 3.0, "the note meets the box's edge, not its first line: {n3:?} {bx:?}");
}

#[test]
fn table_cells_given_as_blocks_leave_room_for_fr_columns() {
    let s = session(book(
        json!([{"type": "table", "id": "tb", "columns": ["auto", "1fr", "auto"], "rows": [[
            {"blocks": [{"type": "p", "id": "a", "text": "THIS COPY IS ISSUED TO"}]},
            {"blocks": [{"type": "p", "id": "mid", "text": "the middle"}]},
            "end"
        ]]}]),
        json!([]),
    ));
    let (a, mid) = (rect_of(&s, "a"), rect_of(&s, "mid"));
    assert!(mid.x0 > a.x1, "{a:?} {mid:?}");
    assert!(mid.x1 - mid.x0 > 30.0, "{mid:?}");
}
