//! Pictures: spreads, numbered figures and plates, words over art, and
//! black ink.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use matenadar_model::Document;
use matenadar_typst::{CheckOptions, Files, Fonts, Paths, Session, check, inspect};
use serde_json::{Value, json};

fn fonts() -> Arc<Fonts> {
    static FONTS: OnceLock<Arc<Fonts>> = OnceLock::new();
    FONTS.get_or_init(|| Arc::new(Fonts::new(&[], false))).clone()
}

/// A PNG `w` × `h` whose pixel at (x, y) is `f(x, y)`.
fn png(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb(f(x, y)));
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
}

const RED: [u8; 3] = [220, 30, 30];
const BLUE: [u8; 3] = [30, 60, 220];

/// A picture whose left half is red and right half blue.
fn halves() -> Vec<u8> {
    png(400, 200, |x, _| if x < 200 { RED } else { BLUE })
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

/// A 6 × 9 in book with a 0.125 in bleed and a folio.
fn book(content: Value, design: Value) -> Value {
    let mut d = json!({
        "base": {"font": "Libertinus Serif", "size": 10, "line_height": 1.4},
        "styles": {"body": {"align": "justify"}, "caption": {"size": 8, "italic": true}, "folio": {"size": 9, "align": "center"}},
        "masters": {"body": {
            "width": "6in", "height": "9in", "bleed": "0.125in",
            "margins": {"top": 54, "bottom": 54, "inside": 54, "outside": 54},
            "foreground": [{"type": "text", "id": "folio", "style": "folio", "x": 54, "y": 610, "w": 324, "content": [{"field": "page"}]}]
        }}
    });
    if let (Some(d), Value::Object(more)) = (d.as_object_mut(), design) {
        d.extend(more);
    }
    json!({"matenadar": "1", "meta": {"title": "Test", "lang": "en"}, "design": d, "content": content})
}

/// The colour at a point of a page, in points from the trim's corner (or,
/// with `bleed`, from the bleed's).
fn pixel(s: &Session, page: usize, x: f64, y: f64, bleed: bool) -> [u8; 3] {
    let data = s.png(page, 72.0, bleed).expect("page renders");
    let img = image::load_from_memory(&data).unwrap().to_rgb8();
    img.get_pixel(x.round() as u32, y.round() as u32).0
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (*x as i32 - y as i32).abs() < 40)
}

fn page_text(s: &Session, page: usize) -> String {
    let mut out = String::new();
    for item in inspect::items(s) {
        if item.page == page
            && let inspect::Kind::Text { text, .. } = &item.kind
        {
            out.push_str(text);
            out.push(' ');
        }
    }
    out
}

fn title_page() -> Value {
    json!({"type": "page", "id": "t", "master": "body", "elements": [{"type": "text", "id": "tt", "x": 54, "y": 200, "w": 324, "text": "Title"}]})
}

#[test]
fn a_spread_lays_one_picture_across_two_facing_pages() {
    let s = open(book(json!([title_page(), {"type": "spread", "id": "sp", "master": "body", "src": "art/two.png"}]), json!({})), &[("art/two.png", halves())]);
    assert_eq!(s.page_count(), 3);
    assert_eq!(s.position("sp").map(|p| p.0), Some(2), "a spread opens on a verso");
    assert!(near(pixel(&s, 2, 20.0, 300.0, false), RED), "the verso shows the picture's left");
    assert!(near(pixel(&s, 2, 420.0, 300.0, false), RED), "up to the fold");
    assert!(near(pixel(&s, 3, 12.0, 300.0, false), BLUE), "the recto shows its right");
    assert!(near(pixel(&s, 3, 420.0, 300.0, false), BLUE));
    // To the bleed: a verso's bleed is on the left, a recto's on the right.
    assert!(near(pixel(&s, 2, 2.0, 2.0, true), RED), "the verso's outer bleed");
    assert!(near(pixel(&s, 3, 432.0 + 9.0 - 2.0, 657.0 - 2.0, true), BLUE), "the recto's outer bleed");
}

#[test]
fn a_spread_opens_on_a_verso_after_a_verso() {
    let s = open(
        book(
            json!([title_page(), {"type": "page", "id": "t2", "master": "body", "elements": []}, {"type": "spread", "id": "sp", "master": "body", "src": "art/two.png"}]),
            json!({}),
        ),
        &[("art/two.png", halves())],
    );
    assert_eq!(s.page_count(), 5, "page 3 is left blank so the spread faces itself");
    assert_eq!(s.position("sp").map(|p| p.0), Some(4));
}

#[test]
fn the_gutter_repeats_the_strip_the_binding_hides() {
    let s = open(
        book(json!([title_page(), {"type": "spread", "id": "sp", "master": "body", "src": "art/two.png", "gutter": 18}]), json!({})),
        &[("art/two.png", halves())],
    );
    // The picture's middle now falls 18 pt before the spine on the verso
    // and 18 pt after it on the recto: the hidden strips repeat it.
    assert!(near(pixel(&s, 2, 432.0 - 8.0, 300.0, false), BLUE), "the verso's hidden strip carries the recto's start");
    assert!(near(pixel(&s, 2, 432.0 - 28.0, 300.0, false), RED));
    assert!(near(pixel(&s, 3, 8.0, 300.0, false), RED), "the recto's hidden strip carries the verso's end");
    assert!(near(pixel(&s, 3, 28.0, 300.0, false), BLUE));
}

#[test]
fn each_side_of_a_spread_can_have_its_own_picture_and_words() {
    let s = open(
        book(
            json!([title_page(), {"type": "spread", "id": "sp", "master": "body",
                "left": {"src": "art/red.png"},
                "right": {"src": "art/blue.png", "focus": [1, 0.5], "elements": [{"type": "text", "id": "words", "x": 72, "y": 72, "w": 200, "text": "Over the art"}]}}]),
            json!({}),
        ),
        &[("art/red.png", png(10, 10, |_, _| RED)), ("art/blue.png", png(10, 10, |_, _| BLUE))],
    );
    assert!(near(pixel(&s, 2, 400.0, 300.0, false), RED));
    assert!(near(pixel(&s, 3, 30.0, 300.0, false), BLUE));
    assert_eq!(s.position("words").map(|p| p.0), Some(3));
    assert!(page_text(&s, 3).contains("Over"));
}

#[test]
fn figures_are_numbered_by_chapter_listed_and_referenced() {
    let fig = |id: &str, caption: &str| json!({"type": "figure", "id": id, "src": "art/red.png", "h": 60, "caption": caption});
    let s = open(
        book(
            json!([
                {"type": "section", "id": "front", "master": "body", "flow": [{"type": "lof", "id": "lof", "leader": "."}]},
                {"type": "section", "id": "c3", "master": "body", "vars": {"chapter": "3"}, "flow": [
                    fig("f1", "The first."),
                    {"type": "p", "id": "p1", "content": ["As ", {"ref": "f2", "show": "label"}, " shows (", {"ref": "f2", "show": "number"}, ")."]},
                    fig("f2", "The second."),
                    {"type": "figure", "id": "plain", "src": "art/red.png", "h": 30}
                ]},
                {"type": "section", "id": "c4", "master": "body", "vars": {"chapter": "4"}, "flow": [fig("f3", "The third.")]}
            ]),
            json!({"figures": {"per": "chapter"}}),
        ),
        &[("art/red.png", png(10, 10, |_, _| RED))],
    );
    let text: String = (1..=s.page_count()).map(|p| page_text(&s, p)).collect();
    let squeezed: String = text.split_whitespace().collect();
    for want in ["Figure3.1", "Figure3.2", "Figure4.1", "AsFigure3.2shows(3.2)."] {
        assert!(squeezed.contains(want), "{want:?} in {text:?}");
    }
    assert!(!text.contains("Figure 3.3"), "a figure without a caption has no number");
    let lof = page_text(&s, 1);
    assert!(lof.contains("3.1") && lof.contains("The first") && !lof.contains("The first.") && lof.contains("4.1"), "the list of figures, without closing stops: {lof:?}");
}

#[test]
fn plates_take_half_a_page_a_page_or_the_page_to_its_bleed() {
    let words = "Words run on past the plate. ".repeat(30);
    let s = open(
        book(
            json!([{"type": "section", "id": "c", "master": "body", "flow": [
                {"type": "p", "id": "a", "text": words},
                {"type": "figure", "id": "half", "src": "art/red.png", "plate": "half", "caption": "Half."},
                {"type": "p", "id": "b", "text": words},
                {"type": "figure", "id": "full", "src": "art/red.png", "plate": "full", "caption": "Full."},
                {"type": "p", "id": "c2", "text": words},
                {"type": "figure", "id": "bleed", "src": "art/blue.png", "plate": "bleed"},
                {"type": "p", "id": "d", "text": words}
            ]}]),
            json!({"styles": {"body": {"align": "justify"}, "caption": {"size": 8}, "folio": {"size": 9, "align": "center"}}}),
        ),
        &[("art/red.png", png(40, 30, |_, _| RED)), ("art/blue.png", png(30, 40, |_, _| BLUE))],
    );
    let report = inspect::report(&s);
    let rect = |id: &str| report.iter().find_map(|p| p.nodes.iter().find(|n| n.id == id).map(|n| (p.page, n.rect))).expect(id);
    let (_, half) = rect("half");
    let body = 648.0 - 108.0;
    assert!(half.y1 - half.y0 <= body / 2.0 + 1.0, "a half plate takes half the text area: {half:?}");
    assert!(half.y0 > 594.0 - body / 2.0 - 1.0, "in the foot half of the page: {half:?}");
    let (full_page, full) = rect("full");
    assert!(((full.y0 + full.y1) / 2.0 - 324.0).abs() < 4.0, "a full plate and its caption are centred on the text area: {full:?}");
    assert!(!page_text(&s, full_page).contains("Words"), "a full plate has its page to itself");
    let bleed_page = s.position("bleed").expect("placed").0;
    assert!(near(pixel(&s, bleed_page, 1.0, 1.0, false), BLUE), "a full-bleed plate reaches the trim's corner");
    assert!(near(pixel(&s, bleed_page, 2.0, 2.0, true), BLUE), "and the bleed");
    assert_eq!(page_text(&s, bleed_page).trim(), "", "its page hides the folio");
    assert!(s.position("d").is_some_and(|(p, _, _)| p != bleed_page), "the text runs on around it");
    assert!(!page_text(&s, full_page).contains("Figure"), "no numbers without design.figures");
}

#[test]
fn words_over_art_stay_in_the_safe_zone_on_a_scrim() {
    let s = open(
        book(
            json!([{"type": "page", "id": "pg", "master": "body", "elements": [
                {"type": "image", "src": "art/dark.png", "x": -9, "y": -9, "w": 450, "h": 666},
                {"type": "text", "id": "w", "x": -40, "y": 700, "w": 180, "safe": true, "text": "Words over the night sky.",
                 "scrim": {"color": "#fff", "opacity": 0.85, "feather": 6, "inset": 10}}
            ]}]),
            json!({}),
        ),
        &[("art/dark.png", png(20, 30, |_, _| [20, 25, 40]))],
    );
    let report = inspect::report(&s);
    let w = report[0].nodes.iter().find(|n| n.id == "w").expect("the frame is placed").rect;
    assert!(w.x0 >= 54.0 - 0.5 && w.y1 <= 648.0 - 54.0 + 0.5, "moved inside the margins: {w:?}");
    // Between the frame's text and the scrim's edge, the night is lightened.
    let under = pixel(&s, 1, w.x0 - 3.0, (w.y0 + w.y1) / 2.0, false);
    assert!(under.iter().all(|c| *c > 150), "the scrim lightens the art behind the words: {under:?}");
    let away = pixel(&s, 1, 300.0, 100.0, false);
    assert!(away.iter().all(|c| *c < 60), "the rest stays dark: {away:?}");
}

#[test]
fn black_ink_reports_colour_and_the_grey_filter_clears_it() {
    let photo = png(60, 40, |x, y| if (x + y) % 2 == 0 { RED } else { BLUE });
    let mut doc = book(
        json!([{"type": "page", "id": "pg", "master": "body", "elements": [
            {"type": "image", "id": "colour", "src": "art/photo.png", "x": 54, "y": 54, "w": 120, "h": 80},
            {"type": "image", "id": "grey", "src": "art/photo.png", "x": 54, "y": 160, "w": 120, "h": 80, "filter": {"gray": true, "contrast": 3}},
            {"type": "text", "id": "warm", "x": 54, "y": 300, "w": 200, "text": "Warm words", "set": {"color": "#b03020"}},
            {"type": "text", "id": "ink", "x": 54, "y": 340, "w": 200, "text": "Black words", "set": {"color": "#222"}}
        ]}]),
        json!({}),
    );
    doc["print"] = json!({"ink": "black"});
    let s = open(doc, &[("art/photo.png", photo)]);
    let flagged: Vec<String> = check(&s, &CheckOptions::default()).into_iter().filter(|i| i.code == "color-in-black-ink").filter_map(|i| i.node).collect();
    assert!(flagged.contains(&"colour".to_string()) && flagged.contains(&"warm".to_string()), "{flagged:?}");
    assert!(!flagged.contains(&"grey".to_string()) && !flagged.contains(&"ink".to_string()), "{flagged:?}");
    let [r, g, b] = pixel(&s, 1, 54.0 + 60.0, 160.0 + 40.0, false);
    assert!(r == g && g == b, "the filtered picture prints in greys: {r} {g} {b}");
}

#[test]
fn the_book_wide_filter_greys_every_raster_picture_that_sets_none() {
    let mut doc = book(
        json!([{"type": "page", "id": "pg", "master": "body", "elements": [
            {"type": "image", "id": "a", "src": "art/photo.png", "x": 54, "y": 54, "w": 120, "h": 80},
            {"type": "image", "id": "kept", "src": "art/photo.png", "x": 54, "y": 160, "w": 120, "h": 80, "filter": {}}
        ]}]),
        json!({}),
    );
    doc["print"] = json!({"ink": "black", "filter": {"gray": true}});
    let s = open(doc, &[("art/photo.png", png(60, 40, |_, _| RED))]);
    let [r, g, b] = pixel(&s, 1, 100.0, 90.0, false);
    assert!(r == g && g == b, "{r} {g} {b}");
    let flagged: Vec<String> = check(&s, &CheckOptions::default()).into_iter().filter(|i| i.code == "color-in-black-ink").filter_map(|i| i.node).collect();
    assert_eq!(flagged, vec!["kept".to_string()], "a picture can opt out with an empty filter, and is reported");
}
