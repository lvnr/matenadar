//! Structured vector and layout elements: letter grids, rulings, calendars,
//! repeated dated pages, framed panel grids, and callout/speech balloons.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use matenadar_model::Document;
use matenadar_typst::{Files, Fonts, Paths, Session, inspect};
use serde_json::{Value, json};

fn fonts() -> Arc<Fonts> {
    static FONTS: OnceLock<Arc<Fonts>> = OnceLock::new();
    FONTS.get_or_init(|| Arc::new(Fonts::new(&[], false))).clone()
}

fn open(v: Value, files: &[(&str, Vec<u8>)]) -> Session {
    let doc = Document::from_json(&v.to_string()).expect("document");
    assert_eq!(matenadar_model::validate(&doc), vec![], "the test document is valid");
    let given = Files::memory();
    for (path, data) in files {
        given.give(path, data.clone());
    }
    let paths = Paths { root: PathBuf::new(), font_dirs: vec![], system_fonts: false };
    let s = Session::with_files(doc, paths, fonts(), given).expect("layout");
    assert_eq!(s.problems(), &[] as &[String], "no problems generating the document");
    s
}

/// A 6 × 9 in book whose master draws a folio.
fn book(content: Value) -> Value {
    json!({"matenadar": "1", "meta": {"title": "Test", "lang": "en"}, "design": {
        "base": {"font": "Libertinus Serif", "size": 10, "line_height": 1.4},
        "styles": {"folio": {"size": 9, "align": "center"}, "grid": {"size": 12, "align": "center"}, "small": {"size": 8}},
        "masters": {"body": {
            "width": "6in", "height": "9in",
            "margins": {"top": 54, "bottom": 54, "inside": 54, "outside": 54},
            "foreground": [{"type": "text", "id": "folio", "style": "folio", "x": 54, "y": 610, "w": 324, "content": [{"field": "page"}]}]
        }}
    }, "content": content})
}

fn page(id: &str, elements: Value) -> Value {
    json!({"type": "page", "id": id, "master": "body", "layers": true, "elements": elements})
}

fn pixel_at(s: &Session, page: usize, x: f64, y: f64, dpi: f32) -> [u8; 3] {
    let data = s.png(page, dpi, false).expect("page renders");
    let img = image::load_from_memory(&data).unwrap().to_rgb8();
    let k = dpi as f64 / 72.0;
    img.get_pixel((x * k).round() as u32, (y * k).round() as u32).0
}

fn pixel(s: &Session, page: usize, x: f64, y: f64) -> [u8; 3] {
    pixel_at(s, page, x, y, 72.0)
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (*x as i32 - y as i32).abs() < 40)
}

fn dark(a: [u8; 3]) -> bool {
    a.iter().all(|v| *v < 110)
}

const WHITE: [u8; 3] = [255, 255, 255];

fn texts(s: &Session, page: usize) -> Vec<(String, inspect::Rect)> {
    inspect::items(s)
        .into_iter()
        .filter(|i| i.page == page)
        .filter_map(|i| match i.kind {
            inspect::Kind::Text { text, .. } => Some((text, i.rect)),
            _ => None,
        })
        .collect()
}

fn page_text(s: &Session, page: usize) -> String {
    texts(s, page).into_iter().map(|(t, _)| t).collect::<Vec<_>>().join(" ")
}

fn dark_runs_down(s: &Session, page: usize, x: f64, y0: f64, y1: f64) -> usize {
    let data = s.png(page, 144.0, false).expect("page renders");
    let img = image::load_from_memory(&data).unwrap().to_rgb8();
    let (mut runs, mut inside) = (0, false);
    for y in (y0 * 2.0) as u32..(y1 * 2.0) as u32 {
        let d = img.get_pixel((x * 2.0) as u32, y).0.iter().all(|v| *v < 200);
        if d && !inside {
            runs += 1;
        }
        inside = d;
    }
    runs
}

fn line_width_across(s: &Session, page: usize, x: f64, y: f64) -> u32 {
    let data = s.png(page, 288.0, false).expect("page renders");
    let img = image::load_from_memory(&data).unwrap().to_rgb8();
    let (cx, cy) = ((x * 4.0) as i64, (y * 4.0) as i64);
    let is_dark = |px: i64| img.get_pixel(px as u32, cy as u32).0.iter().all(|v| *v < 128);
    let start = (cx - 12..=cx + 12).find(|p| is_dark(*p)).expect("a line near the point");
    (start..start + 40).take_while(|p| is_dark(*p)).count() as u32
}

#[test]
fn a_letter_grid_hides_cells_keeps_givens_and_marks_regions() {
    let sol = "534678912672195348198342567859761423426853791713924856961537284287419635345286179";
    let given = "53..7....6..195....98....6.8...6...34..8.3..17...2...6.6....28....419..5....8..79";
    let rows: Vec<Value> = (0..9)
        .map(|r| {
            Value::Array(
                (0..9)
                    .map(|c| {
                        let d = &sol[r * 9 + c..r * 9 + c + 1];
                        if &given[r * 9 + c..r * 9 + c + 1] != "." { json!({"text": d, "given": true}) } else { json!(d) }
                    })
                    .collect(),
            )
        })
        .collect();
    let grid = |id: &str, blank: bool| {
        json!({"type": "letter-grid", "id": id, "x": 54, "y": 72, "cell": 30, "rows": rows, "blank": blank, "regions": [3, 3],
            "lines": {"width": 0.5, "color": "#000000"}, "style": "grid"})
    };
    let crossword = json!({"type": "letter-grid", "id": "cw", "x": 54, "y": 400, "cell": 30, "blank": true, "style": "grid",
        "lines": {"width": 1.5, "color": "#000000"},
        "rows": [[{"text": "C", "number": "1"}, "A", "T"], [{"block": true}, {"text": "O", "circle": true}, {"block": true}]]});
    let s = open(book(json!([page("p1", json!([grid("puzzle", true), crossword])), page("p2", json!([grid("answers", false)]))])), &[]);
    let digits = |p: usize| page_text(&s, p).chars().filter(|c| c.is_ascii_digit()).count();
    assert_eq!(digits(1), 30 + 1 + 1);
    assert_eq!(digits(2), 81 + 1);
    let text = page_text(&s, 1);
    assert!(!text.contains('C') && !text.contains('O'), "blank grid hides ungiven letters: {text}");
    let thin = line_width_across(&s, 1, 54.0 + 30.0, 72.0 + 15.0);
    let thick = line_width_across(&s, 1, 54.0 + 90.0, 72.0 + 15.0);
    assert!(thick >= thin * 2, "a region line ({thick} px) is heavier than a cell line ({thin} px)");
    assert!(dark(pixel_at(&s, 1, 54.0 + 45.0 - 12.0, 400.0 + 45.0, 144.0)), "the ring");
    assert!(!dark(pixel_at(&s, 1, 54.0 + 45.0 - 6.0, 400.0 + 45.0, 144.0)), "inside the ring");
}

#[test]
fn rulings_lay_lines_dots_and_grids_at_their_pitch() {
    let s = open(
        book(json!([page("p1", json!([
            {"type": "ruling", "x": 54, "y": 72, "w": 300, "h": 220, "pitch": 20, "stroke": {"width": 1, "color": "#000000"}},
            {"type": "ruling", "x": 54, "y": 320, "w": 300, "h": 200, "pattern": "graph", "pitch": 10, "major": 5,
                "stroke": {"width": 0.5, "color": "#000000"}, "major_stroke": {"width": 2, "color": "#000000"}}
        ]))])),
        &[],
    );
    assert_eq!(dark_runs_down(&s, 1, 200.0, 72.0, 72.0 + 220.0 + 0.4), 11);
    let (major, minor) = (line_width_across(&s, 1, 54.0, 335.0), line_width_across(&s, 1, 64.0, 335.0));
    assert!(major > minor * 2, "major {major} px, minor {minor} px");
}

#[test]
fn a_calendar_sets_a_month_from_its_first_weekday() {
    let cal = |id: &str, y: f64, start: &str| {
        json!({"type": "calendar", "id": id, "x": 54, "y": y, "w": 280, "h": 200, "year": 2027, "month": 3, "week_start": start, "style": "small"})
    };
    let s = open(book(json!([page("p1", json!([cal("mon", 60.0, "monday"), cal("sun", 330.0, "sunday")]))])), &[]);
    let t = texts(&s, 1);
    let one = |y0: f64| t.iter().find(|(w, r)| w == "1" && r.y0 > y0 && r.y0 < y0 + 200.0).map(|(_, r)| r.x0).expect("day 1");
    let thirty_one = t.iter().filter(|(w, _)| w == "31").count();
    assert_eq!(thirty_one, 2, "March has 31 days");
    assert!(one(60.0) < 54.0 + 40.0, "first column: {}", one(60.0));
    assert!(one(330.0) > 54.0 + 40.0 && one(330.0) < 54.0 + 80.0, "second column: {}", one(330.0));
    assert!(page_text(&s, 1).contains("Mon") && page_text(&s, 1).contains("Sun"));
}

#[test]
fn repeated_pages_carry_their_own_dates_and_words() {
    let p = json!({"type": "page", "id": "day", "master": "body", "layers": true,
        "repeat": {"dates": {"from": "2027-02-27", "step": 1}, "vars": [{"prompt": "One"}, {"prompt": "Two"}, {"prompt": "Three"}]},
        "elements": [
            {"type": "text", "id": "date", "x": 54, "y": 54, "w": 324, "content": [{"field": "section.date", "format": "[weekday] [day padding:none] [month repr:long]"}]},
            {"type": "text", "id": "prompt", "x": 54, "y": 90, "w": 324, "content": [{"field": "section.prompt"}, " (", {"field": "section.n"}, ")"]}
        ]});
    let s = open(book(json!([page("first", json!([])), p])), &[]);
    assert_eq!(s.page_count(), 4, "three times");
    let t2 = page_text(&s, 2).replace(' ', "");
    assert!(t2.contains("Saturday27February") && t2.contains("One(1)"), "{t2}");
    let t4 = page_text(&s, 4).replace(' ', "");
    assert!(t4.contains("Monday1March") && t4.contains("Three(3)"), "{t4}");
    assert!(t4.contains('4'), "the folio counts on");
    assert_eq!(s.position("day").map(|p| p.0), Some(2), "the page's id points at its first time");
}

#[test]
fn panels_tile_a_page_and_balloons_join_their_tails() {
    let s = open(
        book(json!([page("p1", json!([{"type": "panels", "id": "pn", "x": 36, "y": 36, "w": 360, "h": 500, "gutter": 10,
            "rows": [
                {"h": "1fr", "panels": [{"id": "a", "fill": "#ff0000"}, {"id": "b", "w": "2fr", "fill": "#0000ff"}]},
                {"h": "200pt", "panels": [{"id": "c", "fill": "#00ff00", "elements": [
                    {"type": "balloon", "id": "say", "x": 60, "y": 30, "w": 140, "text": "Where are my keys?", "tail": [120, 160]}
                ]}]}
            ]}]))])),
        &[],
    );
    assert!(near(pixel(&s, 1, 36.0 + 50.0, 36.0 + 100.0), [255, 0, 0]));
    assert!(near(pixel(&s, 1, 36.0 + 300.0, 36.0 + 100.0), [0, 0, 255]), "the 2fr panel is twice as wide");
    assert!(near(pixel(&s, 1, 36.0 + 116.7 + 5.0, 36.0 + 100.0), WHITE), "the gutter");
    assert!(near(pixel(&s, 1, 36.0 + 100.0, 36.0 + 290.0 + 10.0 + 180.0), [0, 255, 0]), "the second row, 200 pt");
    assert!(page_text(&s, 1).contains("keys"), "the balloon's words are live text");
}

#[test]
fn a_balloon_and_its_tail_are_one_outline() {
    let balloon = |tail: bool| {
        let mut b = json!({"type": "balloon", "id": if tail { "with" } else { "without" }, "x": 216, "y": 300, "anchor": "center", "w": 140, "text": "Where are my keys?"});
        if tail {
            b["tail"] = json!([366, 300]);
        }
        b
    };
    let s = open(book(json!([page("p1", json!([balloon(true)])), page("p2", json!([balloon(false)]))])), &[]);
    let words = texts(&s, 1).into_iter().map(|(_, r)| r).filter(|r| r.y0 < 400.0).fold(None::<inspect::Rect>, |a, r| match a {
        None => Some(r),
        Some(a) => Some(inspect::Rect { x0: a.x0.min(r.x0), y0: a.y0.min(r.y0), x1: a.x1.max(r.x1), y1: a.y1.max(r.y1) }),
    });
    let words = words.expect("the words");
    let png = |p: usize| image::load_from_memory(&s.png(p, 144.0, false).unwrap()).unwrap().to_rgb8();
    let (with, without) = (png(1), png(2));
    let is_dark = |img: &image::RgbImage, x: u32, y: u32| img.get_pixel(x, y).0.iter().all(|v| *v < 128);
    let joined = (-8..=-2).any(|dy| {
        let y = ((300 + dy) * 2) as u32;
        let from = ((words.x1 + 2.0) * 2.0) as u32;
        (from..from + 140).find(|x| is_dark(&without, *x, y)).is_some_and(|edge| !is_dark(&with, edge, y))
    });
    assert!(joined, "the tail opens the balloon's outline where they meet");
    let y = (290.0 - (words.y1 - words.y0)) as u32 * 2;
    let x = (216.0 * 2.0) as u32;
    let edge = (y.saturating_sub(80)..y).rev().find(|yy| is_dark(&without, x, *yy));
    assert!(edge.is_none() || is_dark(&with, x, edge.unwrap()));
}
