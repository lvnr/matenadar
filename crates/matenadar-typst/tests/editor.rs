//! The page editor: finding, changing, undoing, and a patch that replays
//! to the same pages.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use matenadar_model::{Document, Op};
use matenadar_typst::check::{CheckOptions, check};
use matenadar_typst::inspect::items_on;
use matenadar_typst::{Editor, Files, Fonts, Paths, Session};
use serde_json::{Value, json};

fn fonts() -> Arc<Fonts> {
    static FONTS: OnceLock<Arc<Fonts>> = OnceLock::new();
    FONTS.get_or_init(|| Arc::new(Fonts::new(&[], false))).clone()
}

fn files() -> Files {
    let files = Files::at(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    // Left half black, right half white.
    let mut img = image::RgbImage::from_pixel(40, 20, image::Rgb([255, 255, 255]));
    for y in 0..20 {
        for x in 0..20 {
            img.put_pixel(x, y, image::Rgb([0, 0, 0]));
        }
    }
    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    files.give("images/halves.png", png);
    files
}

fn session_of(doc: &Document) -> Session {
    let paths = Paths { root: PathBuf::from(env!("CARGO_MANIFEST_DIR")), font_dirs: vec![], system_fonts: false };
    Session::with_files(doc.clone(), paths, fonts(), files()).expect("layout")
}

const WORDS: [&str; 24] = [
    "the", "night", "shift", "kept", "its", "own", "hours", "and", "the", "river", "below", "ran", "black", "under", "lamps", "while", "somebody",
    "counted", "crates", "by", "hand", "slowly", "again", "tonight",
];

fn words(seed: usize, n: usize) -> String {
    let mut out: Vec<&str> = (0..n).map(|i| WORDS[(seed * 7 + i * 5 + i * i) % WORDS.len()]).collect();
    out.push("end.");
    out.join(" ")
}

fn doc() -> Document {
    let flow: Vec<Value> = (1..=14).map(|i| json!({"type": "p", "id": format!("p{i}"), "text": words(i, 70 + i * 3)})).collect();
    let v = json!({
        "matenadar": "1",
        "meta": {"title": "Edit"},
        "design": {
            "base": {"font": "Libertinus Serif", "size": 10, "line_height": "14pt"},
            "styles": {"body": {"align": "justify", "indent": "1em"}},
            "masters": {"body": {
                "width": "5in", "height": "8in", "bleed": "0.125in",
                "margins": {"top": 54, "bottom": 54, "inside": 54, "outside": 45},
                "foreground": [{"type": "text", "id": "folio", "x": 170, "y": 540, "w": 20, "content": [{"field": "page"}]}]
            }}
        },
        "content": [
            {"type": "section", "id": "s1", "master": "body", "flow": flow},
            {"type": "page", "id": "plate", "master": "body", "elements": [
                {"type": "image", "id": "ph", "src": "images/halves.png", "x": 72, "y": 72, "w": 144, "h": 72, "fit": "stretch"}
            ]}
        ]
    });
    Document::from_json(&v.to_string()).expect("document")
}

fn editor() -> Editor {
    let d = doc();
    assert_eq!(matenadar_model::validate(&d), vec![]);
    Editor::new(session_of(&d), CheckOptions::default())
}

fn fields(v: Value) -> serde_json::Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn hits_find_words_and_pictures() {
    let e = editor();
    let (page, x, y) = e.session().position("p2").unwrap();
    let hit = e.hit(page, x + 20.0, y + 6.0).expect("something is there");
    assert_eq!(hit.id, "p2");
    assert_eq!(hit.kind, "p");
    assert!(hit.offset.is_some(), "a point on words gives an offset");
    assert!(hit.rect.contains(x + 20.0, y + 6.0, 0.0));
    let plate = e.session().page_count();
    let pic = e.hit(plate, 144.0, 108.0).expect("the picture");
    assert_eq!((pic.id.as_str(), pic.kind), ("ph", "image"));
    assert!(e.hit(plate, 300.0, 400.0).is_none(), "empty paper is nothing");
    // A frame's text is the frame's: the paragraphs the engine makes of it are not nodes.
    let mut e = e;
    e.edit(vec![serde_json::from_value(json!({"op": "insert", "id": "plate", "field": "elements", "value": {"type": "text", "id": "cap", "x": 72, "y": 300, "w": 200, "text": "A caption."}})).unwrap()]).unwrap();
    let cap = e.hit(plate, 80.0, 306.0).expect("the caption");
    assert_eq!(cap.id, "cap");
    assert!(cap.ancestors.iter().all(|a| e.node(&a.id).is_some()));
    let rects = e.rects("p2");
    assert!(!rects.is_empty() && rects[0].page == page);
}

#[test]
fn edits_undo_and_redo() {
    let mut e = editor();
    let original = e.document().clone();
    let prints = e.session().fingerprints();
    let edited = e.set_fields("p2", &fields(json!({"space_before": "24pt", "align": "center"})), true).unwrap();
    assert!(!edited.changed_pages.is_empty());
    assert!(edited.can_undo && !edited.can_redo);
    let node = e.node("p2").unwrap();
    assert_eq!(node.pointer("/set/align"), Some(&json!("center")));
    let after = e.session().fingerprints();
    e.undo().unwrap();
    assert_eq!(e.document(), &original);
    assert_eq!(e.session().fingerprints(), prints);
    let redone = e.redo().unwrap();
    assert!(redone.can_undo && !redone.can_redo);
    assert_eq!(e.session().fingerprints(), after);
    assert!(e.redo().is_err(), "nothing left to redo");
}

#[test]
fn breaks_go_in_before_a_block_and_come_out() {
    let mut e = editor();
    let (page, _, _) = e.session().position("p5").unwrap();
    e.break_before("p5", Some("page")).unwrap();
    let (moved, _, y) = e.session().position("p5").unwrap();
    assert_eq!(moved, page + 1, "p5 starts the next page");
    assert!(y < 70.0, "at the top of its page");
    e.break_before("p5", None).unwrap();
    assert_eq!(e.session().position("p5").unwrap().0, page);
    // A second forced break where the paragraph already opens a page adds no blank page.
    e.break_before("p5", Some("page")).unwrap();
    let pages = e.session().page_count();
    e.break_before("p6", Some("page")).unwrap();
    e.break_before("p5", Some("recto")).unwrap();
    assert!(e.session().page_count() >= pages);
    assert_eq!(e.session().position("p5").unwrap().0 % 2, 1, "recto");
}

#[test]
fn a_page_can_leave_out_its_folio() {
    let mut e = editor();
    let (page, _, _) = e.session().position("p6").unwrap();
    // Master elements are not nodes; the folio is the text at its place.
    let folio_on = |e: &Editor, p: usize| items_on(e.session(), [p]).iter().any(|i| i.rect.y0 > 530.0);
    assert!(folio_on(&e, page) && folio_on(&e, page + 1));
    e.hide_on_page("p6", &["folio".to_string()]).unwrap();
    assert!(!folio_on(&e, page), "not on p6's page");
    assert!(folio_on(&e, page + 1), "still on the next");
    e.hide_on_page("p6", &[]).unwrap();
    assert!(folio_on(&e, page));
    // By the style of a text element, or all of them.
    e.edit(vec![serde_json::from_value(json!({"op": "set", "at": "/design/masters/body/foreground/0", "path": "/style", "value": "folio"})).unwrap()]).unwrap();
    e.hide_on_page("p6", &["folio".to_string()]).unwrap();
    assert!(!folio_on(&e, page));
    e.hide_on_page("p6", &["all".to_string()]).unwrap();
    assert!(!folio_on(&e, page) && folio_on(&e, page + 1));
    // `all` reaches elements with neither an id nor a style.
    e.undo().unwrap();
    e.edit(vec![serde_json::from_value(json!({"op": "unset", "at": "/design/masters/body/foreground/0", "path": "/id"})).unwrap()]).unwrap();
    e.edit(vec![serde_json::from_value(json!({"op": "unset", "at": "/design/masters/body/foreground/0", "path": "/style"})).unwrap()]).unwrap();
    e.hide_on_page("p6", &["all".to_string()]).unwrap();
    assert!(!folio_on(&e, page) && folio_on(&e, page + 1));
}

#[test]
fn nudge_takes_a_line_back_or_leaves_the_paragraph_alone() {
    let mut e = editor();
    let mut done = 0;
    for i in 1..=14 {
        let id = format!("p{i}");
        let before = e.lines(&id);
        let prints = e.session().fingerprints();
        match e.nudge(&id, -1) {
            Ok(n) => {
                assert_eq!((n.lines_before, n.lines_after), (before, before - 1));
                assert_eq!(e.lines(&id), before - 1);
                assert!(n.tracking < 0.0 && n.tracking >= -0.02);
                done += 1;
            }
            Err(_) => assert_eq!(e.session().fingerprints(), prints, "a failed nudge changes nothing"),
        }
    }
    assert!(done > 0, "some paragraph could lose a line");
    let patch = e.patch();
    assert_eq!(patch.layout.len(), done);
    assert!(patch.words.is_empty());
}

#[test]
fn crop_shows_only_part_of_a_picture() {
    let mut e = editor();
    let plate = e.session().page_count();
    let at = |e: &Editor| {
        let png = e.session().png(plate, 72.0, false).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgb8();
        (img.get_pixel(110, 108).0[0], img.get_pixel(180, 108).0[0])
    };
    assert!(at(&e).0 < 30 && at(&e).1 > 220, "the whole picture: black then white");
    e.set_fields("ph", &fields(json!({"crop": {"x": 0.5, "w": 0.5}})), false).unwrap();
    assert!(at(&e).0 > 220 && at(&e).1 > 220, "only the white half");
    e.set_fields("ph", &fields(json!({"crop": {"w": 0.5}, "fit": "cover"})), false).unwrap();
    assert!(at(&e).0 < 30 && at(&e).1 < 30, "only the black half");
}

#[test]
fn diagnostics_after_edits_match_a_full_check() {
    let mut e = editor();
    e.set_fields("p3", &fields(json!({"space_before": "60pt"})), true).unwrap();
    e.break_before("p8", Some("page")).unwrap();
    e.set_words("p9", json!(["Short."])).unwrap();
    let key = |i: &matenadar_typst::Issue| (i.code, i.page, i.node.clone());
    let mine: BTreeSet<_> = e.diagnostics().iter().filter(|i| i.code != "fallback-font").map(key).collect();
    let full: BTreeSet<_> = check(e.session(), &CheckOptions::default()).iter().filter(|i| i.code != "fallback-font").map(key).collect();
    assert_eq!(mine, full);
}

#[test]
fn the_fonts_a_layout_used_are_listed() {
    let e = editor();
    let used = e.session().fonts_used();
    let body = used.iter().find(|f| f.family == "Libertinus Serif" && f.weight == 400 && !f.italic).expect("the body face");
    assert!(body.pages.contains(&1));
    assert!(body.file.is_none(), "Typst's own faces have no file");
}

#[test]
fn pages_report_their_geometry() {
    let e = editor();
    let pages = e.session().pages();
    let p1 = &pages[0];
    assert_eq!((p1.trim.w, p1.trim.h), (360.0, 576.0));
    assert_eq!(p1.side, "recto");
    assert_eq!((p1.bleed.top, p1.bleed.right, p1.bleed.left), (9.0, 9.0, 0.0), "the outside of a recto is its right");
    assert_eq!((pages[1].bleed.left, pages[1].bleed.right), (9.0, 0.0));
    assert_eq!(p1.master.as_deref(), Some("body"));
    assert_eq!(p1.item.as_deref(), Some("s1"));
    let m = p1.margins.unwrap();
    assert_eq!((m.inside, m.outside, m.top), (54.0, 45.0, 54.0));
    assert_eq!(pages.last().unwrap().item.as_deref(), Some("plate"));
}

#[test]
fn the_patch_replays_to_the_same_pages() {
    let mut e = editor();
    e.set_words("p3", json!(["A new ", {"text": "opening", "em": true}, " for the third paragraph."])).unwrap();
    e.set_fields("p4", &fields(json!({"align": "center"})), true).unwrap();
    e.set_fields("p4", &fields(json!({"align": "right"})), true).unwrap();
    e.set_fields("p4", &fields(json!({"space_after": "12pt"})), true).unwrap();
    e.set_fields("p4", &fields(json!({"space_after": null})), true).unwrap();
    e.set_fields("ph", &fields(json!({"crop": {"x": 0.25, "w": 0.5}, "x": 90, "y": 72})), false).unwrap();
    e.break_before("p6", Some("recto")).unwrap();
    e.hide_on_page("p6", &["folio".to_string()]).unwrap();
    e.edit(vec![serde_json::from_value(json!({"op": "insert", "after": "p7", "value": {"type": "p", "id": "p7b", "text": "Added."}})).unwrap()]).unwrap();
    e.undo().unwrap();
    e.edit(vec![serde_json::from_value(json!({"op": "insert-text", "id": "p8", "offset": 0, "text": "Then "})).unwrap()]).unwrap();

    let patch = e.patch();
    assert_eq!(patch.touched, vec!["p3".to_string(), "p8".to_string()]);
    let layout = serde_json::to_value(&patch.layout).unwrap();
    let paths: Vec<String> = patch
        .layout
        .iter()
        .map(|op| serde_json::to_value(op).unwrap())
        .map(|v| format!("{} {}", v["op"].as_str().unwrap(), v["path"].as_str().unwrap_or("")))
        .collect();
    assert_eq!(paths, vec!["set /set/align", "set /crop", "set /x", "insert ", "set /on_page/p6"], "{layout}");

    // As Aldus replays it: the words through the manuscript, then the layout edits.
    let mut replayed = doc();
    for op in patch.words.iter().chain(&patch.layout) {
        replayed.apply(op).unwrap();
    }
    let ops_json = serde_json::to_string(&patch.layout).unwrap();
    let back: Vec<Op> = serde_json::from_str(&ops_json).unwrap();
    assert_eq!(back.len(), patch.layout.len(), "the patch round-trips through JSON");
    assert_eq!(&replayed, e.document());
    assert_eq!(session_of(&replayed).fingerprints(), e.session().fingerprints());
}
