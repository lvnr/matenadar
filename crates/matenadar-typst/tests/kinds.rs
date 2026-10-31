//! Book kinds drawn from their data: puzzles (mazes, crosswords, Sudoku,
//! logic grids, dot-to-dot), journal pages (rulings, calendars, dated
//! pages), paper-craft sheets, comic panels and balloons, and coloring
//! books.

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

/// The colour at a point of a page, in points from its corner, drawn at
/// `dpi`.
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

/// Text runs on a page, with where they are.
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

/// How many dark pixels a column of the page crosses between `y0` and `y1`.
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

/// The width in pixels at 288 dpi of the dark run that a row crosses
/// nearest `x`.
fn line_width_across(s: &Session, page: usize, x: f64, y: f64) -> u32 {
    let data = s.png(page, 288.0, false).expect("page renders");
    let img = image::load_from_memory(&data).unwrap().to_rgb8();
    let (cx, cy) = ((x * 4.0) as i64, (y * 4.0) as i64);
    let is_dark = |px: i64| img.get_pixel(px as u32, cy as u32).0.iter().all(|v| *v < 128);
    let start = (cx - 12..=cx + 12).find(|p| is_dark(*p)).expect("a line near the point");
    (start..start + 40).take_while(|p| is_dark(*p)).count() as u32
}

/// A 3 × 3 maze, in at the top-left and out at the bottom-right, whose
/// way through is an S: right along the top, left along the middle, right
/// along the bottom.
fn maze(solved: bool) -> Value {
    json!({"type": "maze", "id": if solved { "solved" } else { "puzzle" }, "x": 72, "y": 72, "cell": 72, "rows": ["553", "956", "c55"], "start": [0, 0], "end": [2, 2],
        "solution": [[0, 0], [0, 1], [0, 2], [1, 2], [1, 1], [1, 0], [2, 0], [2, 1], [2, 2]], "solved": solved})
}

#[test]
fn a_maze_draws_its_walls_and_on_a_solution_page_its_way_through() {
    let s = open(book(json!([page("p1", json!([maze(false)])), page("p2", json!([maze(true)]))])), &[]);
    assert_eq!(s.page_count(), 2);
    // The wall under the top row's first two cells, between them and the second row.
    assert!(dark(pixel(&s, 1, 72.0 + 36.0, 144.0)), "a wall where a cell says so");
    // Between (0, 1) and (0, 2) there is no wall: the way runs through.
    assert!(near(pixel(&s, 1, 72.0 + 144.0, 72.0 + 36.0), WHITE), "no wall between open cells");
    let red = [208, 64, 43];
    assert!(!near(pixel(&s, 1, 72.0 + 108.0, 72.0 + 36.0), red), "the puzzle hides the way through");
    assert!(near(pixel(&s, 2, 72.0 + 108.0, 72.0 + 36.0), red), "the solution draws it");
    assert!(near(pixel(&s, 2, 72.0 + 108.0, 72.0 + 108.0), red));
    // The start is a dot, the end a ring.
    assert!(near(pixel(&s, 1, 72.0 + 36.0, 72.0 + 36.0), red), "the start's dot");
    assert!(near(pixel(&s, 1, 72.0 + 180.0, 72.0 + 180.0), WHITE), "the end's ring is open");
    // Puzzle pages keep their folio.
    assert!(page_text(&s, 2).contains('2'));
}

#[test]
fn a_puzzle_grid_hides_its_answers_keeps_its_givens_and_marks_its_regions() {
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
    // The puzzle shows its 30 givens (and the crossword's number, and the
    // folio); the answers show all 81.
    assert_eq!(digits(1), 30 + 1 + 1);
    assert_eq!(digits(2), 81 + 1);
    let text = page_text(&s, 1);
    assert!(!text.contains('C') && !text.contains('O'), "the crossword hides its letters: {text}");
    // Region lines are heavier than cell lines.
    let thin = line_width_across(&s, 1, 54.0 + 30.0, 72.0 + 15.0);
    let thick = line_width_across(&s, 1, 54.0 + 90.0, 72.0 + 15.0);
    assert!(thick >= thin * 2, "a region line ({thick} px) is heavier than a cell line ({thin} px)");
    // The circled cell has a ring inside it.
    assert!(dark(pixel_at(&s, 1, 54.0 + 45.0 - 12.0, 400.0 + 45.0, 144.0)), "the ring");
    assert!(!dark(pixel_at(&s, 1, 54.0 + 45.0 - 6.0, 400.0 + 45.0, 144.0)), "inside the ring");
}

#[test]
fn a_logic_grid_sets_its_categories_in_a_staircase_with_its_marks() {
    let g = json!({"type": "logic-grid", "id": "lg", "x": 54, "y": 72, "cell": 18, "style": "small",
        "categories": [
            {"name": "Friend", "items": ["Ana", "Ben", "Cleo"]},
            {"name": "Pet", "items": ["Cat", "Dog", "Owl"]},
            {"name": "Town", "items": ["Avon", "Bath", "Cork"]}
        ],
        "marks": [{"a": [0, 1], "b": [1, 2], "mark": "Y"}, {"a": [2, 0], "b": [1, 1], "mark": "Z"}]});
    let s = open(book(json!([page("p1", json!([g]))])), &[]);
    let t = texts(&s, 1);
    // A category down the side and across the top is set twice: the lowest is the side's.
    let at = |w: &str| {
        t.iter().filter(|(x, _)| x == w).map(|(_, r)| *r).max_by(|a, b| a.y0.total_cmp(&b.y0)).unwrap_or_else(|| panic!("{w} is set: {t:?}"))
    };
    for w in ["Friend", "Pet", "Town", "Ana", "Ben", "Cleo", "Avon", "Bath", "Cork"] {
        at(w);
    }
    // Friend and Town down the side, Town under Friend.
    assert!(at("Avon").y0 > at("Cleo").y0, "the last category goes down the side, under the first");
    // Ben with the Owl: on Ben's row, right of the side labels.
    let (y, ben) = (at("Y"), at("Ben"));
    assert!((y.y0 - ben.y0).abs() < 4.0, "the mark is on Ben's row: {y:?} {ben:?}");
    assert!(y.x0 > ben.x1);
    // Avon with the Dog: in the second block down, on Avon's row.
    let (z, avon) = (at("Z"), at("Avon"));
    assert!((z.y0 - avon.y0).abs() < 4.0, "Town × Pet is under Friend × Pet: {z:?} {avon:?}");
    assert!(z.x0 < y.x0, "Dog's column is left of Owl's");
}

#[test]
fn dot_to_dot_numbers_its_dots_and_a_closed_shape_returns_to_the_first() {
    let pts = json!([[0, 0], [100, 0], [100, 100], [0, 100], [0, 0]]);
    let s = open(
        book(json!([page("p1", json!([
            {"type": "dots", "id": "d", "x": 100, "y": 100, "w": 200, "h": 200, "points": pts, "viewbox": [0, 0, 100, 100]},
            {"type": "dots", "id": "d2", "x": 100, "y": 380, "w": 200, "h": 200, "points": pts, "viewbox": [0, 0, 100, 100], "solved": true, "start": 7}
        ]))])),
        &[],
    );
    let text = page_text(&s, 1);
    let mut nums: Vec<u32> = text.split_whitespace().filter_map(|w| w.parse().ok()).collect();
    nums.sort();
    // 1–4 on the puzzle, 7–10 on the solved one (it starts at 7), and the
    // folio; the closing point is not numbered again.
    assert_eq!(nums, vec![1, 1, 2, 3, 4, 7, 8, 9, 10]);
    assert!(dark(pixel_at(&s, 1, 100.0, 100.0, 144.0)), "the first dot");
    assert!(near(pixel(&s, 1, 200.0, 100.0), WHITE), "no line on the puzzle");
    assert!(dark(pixel_at(&s, 1, 200.0, 380.0, 144.0)), "the solved line");
    assert!(dark(pixel_at(&s, 1, 100.0, 480.0, 144.0)), "and its closing side, back to the first dot");
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
    // Lines every 20 pt from 20 pt down: 11 in 220 pt.
    assert_eq!(dark_runs_down(&s, 1, 200.0, 72.0, 72.0 + 220.0 + 0.4), 11);
    // A graph's every fifth line is heavier.
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
    // 1 March 2027 is a Monday: the first column from Monday, the second from Sunday.
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
fn a_craft_sheet_draws_faces_tabs_and_folds() {
    let s = open(
        book(json!([page("p1", json!([{"type": "craft", "id": "net", "x": 100, "y": 100, "w": 200, "h": 200, "viewbox": [0, 0, 100, 100],
            "glue": "#00c000",
            "faces": [{"points": [[20, 20], [60, 20], [60, 60], [20, 60]], "fill": "#3060e0", "label": "A"}],
            "edges": [
                {"from": [20, 20], "to": [60, 20], "tab": {"depth": 12}},
                {"from": [60, 20], "to": [60, 60]}, {"from": [60, 60], "to": [20, 60], "kind": "valley"}, {"from": [20, 60], "to": [20, 20]}
            ]}]))])),
        &[],
    );
    assert!(near(pixel(&s, 1, 150.0, 150.0), [48, 96, 224]), "the face's colour");
    assert!(near(pixel(&s, 1, 180.0, 128.0), [0, 192, 0]), "the tab, glue-coloured, outside the edge");
    assert!(page_text(&s, 1).contains('A'));
}

#[test]
fn comic_panels_tile_a_page_and_balloons_join_their_tails() {
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
    // Shares: the first row takes what the second (200 pt) and the gutter leave.
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
    // Just above the tail's root, the balloon's edge (where the tailless
    // balloon has its outline) is covered by the tail: no line across the join.
    let joined = (-8..=-2).any(|dy| {
        let y = ((300 + dy) * 2) as u32;
        let from = ((words.x1 + 2.0) * 2.0) as u32;
        (from..from + 140).find(|x| is_dark(&without, *x, y)).is_some_and(|edge| !is_dark(&with, edge, y))
    });
    assert!(joined, "the tail opens the balloon's outline where they meet");
    // Away from the tail the outline is the same.
    let y = (290.0 - (words.y1 - words.y0)) as u32 * 2;
    let x = (216.0 * 2.0) as u32;
    let edge = (y.saturating_sub(80)..y).rev().find(|yy| is_dark(&without, x, *yy));
    assert!(edge.is_none() || is_dark(&with, x, edge.unwrap()));
}

#[test]
fn a_coloring_book_puts_each_picture_on_a_right_hand_page_with_a_blank_back() {
    let line_art = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100"><circle cx="50" cy="50" r="40" fill="none" stroke="#000" stroke-width="2"/></svg>"##.to_vec();
    let doc = matenadar_model::template::compose(
        "coloring",
        &json!({"title": "Lights", "pages": [{"src": "a.svg", "title": "One"}, {"src": "a.svg"}], "back_note": "Left blank"}),
    )
    .expect("composes");
    let v = serde_json::to_value(&doc).unwrap();
    let s = open(v, &[("a.svg", line_art)]);
    assert_eq!(s.page_count(), 8);
    let images: Vec<usize> = inspect::items(&s)
        .into_iter()
        .filter(|i| matches!(i.kind, inspect::Kind::Image { .. }))
        .map(|i| i.page)
        .collect();
    assert_eq!(images, vec![5, 7], "pictures on rectos");
    assert_eq!(page_text(&s, 6).trim(), "Left blank", "its back carries only the note");
}

#[test]
fn an_a_plus_proof_is_one_page_as_wide_as_the_store_that_grows_with_its_modules() {
    let photo =
        std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/black.png")).expect("picture");
    let proof = |modules: Value, width: &str| {
        let doc = matenadar_model::template::compose(
            "a-plus",
            &json!({"title": "Lights", "width": width, "modules": modules}),
        )
        .expect("composes");
        open(
            serde_json::to_value(&doc).unwrap(),
            &[("a.png", photo.clone())],
        )
    };
    let one = json!([{"layout": "image-header-with-text", "heading": "A winter of lanterns", "body": "Words under the picture.", "images": [{"src": "a.png"}]}]);
    let s = proof(one.clone(), "desktop");
    assert_eq!(s.page_count(), 1);
    let page = &s.pages()[0];
    assert!(
        (page.trim.w - 727.5).abs() < 0.01,
        "970 px wide: {:?}",
        page.trim
    );
    // The header's 970 × 600 picture alone is 450 pt; the heading and words add to it.
    assert!(
        page.trim.h > 450.0 && page.trim.h < 700.0,
        "as tall as what it holds: {:?}",
        page.trim
    );
    let mut more = one.as_array().unwrap().clone();
    more.push(json!({"layout": "single-left-image", "heading": "Who it is for", "body": "A beside the picture.", "images": [{"src": "a.png"}]}));
    more.push(json!({"layout": "comparison-chart", "comparisons": [{"title": "Book one", "asin": "B000000001"}, {"title": "Book two"}],
        "rows": [{"label": "Pages", "value": "32"}, {"label": "Ages", "value": "4-8"}]}));
    let taller = proof(Value::Array(more.clone()), "desktop");
    assert_eq!(taller.page_count(), 1, "still one page");
    assert!(
        taller.pages()[0].trim.h > page.trim.h + 200.0,
        "it grows: {:?}",
        taller.pages()[0].trim
    );
    assert!(page_text(&taller, 1).contains("B000000001"));
    let phone = proof(Value::Array(more), "mobile");
    assert!((phone.pages()[0].trim.w - 360.0).abs() < 0.01);
    assert_eq!(phone.page_count(), 1);
}
