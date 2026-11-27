//! Integration tests for v0.3.0 features:
//! - Flow layout blocks: `row`, `grid`, `rule` (including hit-testing, rects, layout, and ops)
//! - Structured vectors: `barcode` (6 formats) and `chart` (6 kinds), as Element and Block
//! - PDF standards (PDF/A-2b, PDF/A-3b, PDF/UA-1) and `/AF` embedded attachments (Factur-X)
//! - Presentation ergonomics: speaker `notes` and `Master.preset` shorthands
//! - Deterministic post-layout image downsampling (`max_ppi`)

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use matenadar_model::{Document, Op};
use matenadar_typst::{CheckOptions, Editor, Files, Fonts, Paths, PdfAttachmentData, PdfOptions, Session, inspect};
use serde_json::{Value, json};

fn fonts() -> Arc<Fonts> {
    static FONTS: OnceLock<Arc<Fonts>> = OnceLock::new();
    FONTS.get_or_init(|| Arc::new(Fonts::new(&[], false))).clone()
}

fn open(v: Value, files: &[(&str, Vec<u8>)]) -> Session {
    let doc = Document::from_json(&v.to_string()).expect("document parses");
    let problems = matenadar_model::validate(&doc);
    assert_eq!(problems, vec![], "the test document is valid");
    let given = Files::memory();
    for (path, data) in files {
        given.give(path, data.clone());
    }
    let paths = Paths { root: PathBuf::new(), font_dirs: vec![], system_fonts: false };
    let s = Session::with_files(doc, paths, fonts(), given).expect("layout succeeds");
    assert_eq!(s.problems(), &[] as &[String], "no generator problems");
    s
}

fn base_doc(content: Value) -> Value {
    json!({
        "matenadar": "1",
        "meta": {"title": "Matenadar v0.3 Test", "lang": "en-US", "subject": "Integration suite"},
        "design": {
            "colors": {"ink": "#18181b", "accent": "#2563eb", "tint": "#f4f4f5", "line": "#d4d4d8"},
            "base": {"font": "Libertinus Serif", "size": 10, "line_height": 1.35, "color": "ink"},
            "styles": {
                "h1": {"size": 16, "weight": "bold", "space_after": 8},
                "caption": {"size": 8, "color": "#52525b"},
                "mono": {"font": "Libertinus Serif", "size": 8}
            },
            "masters": {
                "a4": {
                    "preset": "a4",
                    "margins": {"top": 48, "bottom": 48, "inside": 48, "outside": 48}
                }
            }
        },
        "content": content
    })
}

fn page_text(s: &Session, page: usize) -> String {
    inspect::items(s)
        .into_iter()
        .filter(|i| i.page == page)
        .filter_map(|i| match i.kind {
            inspect::Kind::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn row_grid_and_rule_layout_rects_hit_and_ops() {
    let doc = base_doc(json!([
        {
            "type": "section",
            "id": "sec1",
            "master": "a4",
            "flow": [
                {
                    "type": "row",
                    "id": "hdr-row",
                    "columns": ["auto", "1fr", "100pt"],
                    "gutter": 12,
                    "valign": "top",
                    "space_after": 10,
                    "cells": [
                        {
                            "id": "c-left",
                            "fill": "tint",
                            "inset": 6,
                            "radius": 4,
                            "blocks": [{"type": "p", "id": "p-left", "text": "ACME Corp"}]
                        },
                        {
                            "id": "c-mid",
                            "inset": 6,
                            "blocks": [{"type": "p", "id": "p-mid", "text": "Invoice #2026-041"}]
                        },
                        {
                            "id": "c-right",
                            "align": "right",
                            "inset": 6,
                            "blocks": [{"type": "p", "id": "p-right", "text": "$4,200.00"}]
                        }
                    ]
                },
                {
                    "type": "rule",
                    "id": "sep-1",
                    "stroke": {"width": 1, "color": "line"},
                    "width": "100%",
                    "space_before": 4,
                    "space_after": 12
                },
                {
                    "type": "grid",
                    "id": "kpi-grid",
                    "columns": ["1fr", "1fr", "1fr"],
                    "column_gutter": 10,
                    "row_gutter": 10,
                    "cells": [
                        {
                            "id": "g-wide",
                            "colspan": 2,
                            "fill": "tint",
                            "stroke": {"width": 0.6, "color": "line"},
                            "inset": 8,
                            "radius": 6,
                            "blocks": [{"type": "p", "id": "p-wide", "text": "Wide summary cell spanning two columns"}]
                        },
                        {
                            "id": "g-tall",
                            "rowspan": 2,
                            "fill": "tint",
                            "inset": 8,
                            "blocks": [{"type": "p", "id": "p-tall", "text": "Tall cell"}]
                        },
                        {
                            "id": "g-a",
                            "inset": 8,
                            "blocks": [{"type": "p", "id": "p-a", "text": "Metric A"}]
                        },
                        {
                            "id": "g-b",
                            "inset": 8,
                            "blocks": [{"type": "p", "id": "p-b", "text": "Metric B"}]
                        }
                    ]
                }
            ]
        }
    ]));

    let s = open(doc, &[]);
    assert_eq!(s.page_count(), 1);

    let mut ed = Editor::new(s, CheckOptions::default());

    // Rects exist for row, cells, rule, grid, and nested blocks.
    let r_left = ed.rects("p-left");
    let r_mid = ed.rects("p-mid");
    let r_right = ed.rects("p-right");
    assert_eq!(r_left.len(), 1);
    assert_eq!(r_mid.len(), 1);
    assert_eq!(r_right.len(), 1);
    assert!(
        r_left[0].rect.x1 < r_mid[0].rect.x0 && r_mid[0].rect.x1 < r_right[0].rect.x0,
        "row cells should sit left-to-right: left={:?}, mid={:?}, right={:?}",
        r_left[0].rect,
        r_mid[0].rect,
        r_right[0].rect
    );

    let r_wide = ed.rects("g-wide");
    let r_a = ed.rects("g-a");
    assert_eq!(r_wide.len(), 1);
    assert_eq!(r_a.len(), 1);
    let wide_w = r_wide[0].rect.x1 - r_wide[0].rect.x0;
    let single_w = r_a[0].rect.x1 - r_a[0].rect.x0;
    assert!(wide_w > single_w * 1.5, "colspan=2 cell ({wide_w}) should be significantly wider than single cell ({single_w})");

    assert!(!ed.rects("sep-1").is_empty(), "rule block reports rect");
    assert!(!ed.rects("hdr-row").is_empty(), "row reports rect");
    assert!(!ed.rects("kpi-grid").is_empty(), "grid reports rect");

    // Hit-testing lands on a nested paragraph inside a grid cell and tracks ancestors.
    let p_wide_rect = ed.rects("p-wide")[0].rect;
    let hit = ed
        .hit(1, (p_wide_rect.x0 + p_wide_rect.x1) / 2.0, (p_wide_rect.y0 + p_wide_rect.y1) / 2.0)
        .expect("hit inside grid cell");
    assert_eq!(hit.id, "p-wide");
    assert!(hit.ancestors.iter().any(|a| a.id == "g-wide"), "ancestors include cell: {:?}", hit.ancestors);
    assert!(hit.ancestors.iter().any(|a| a.id == "kpi-grid"), "ancestors include grid: {:?}", hit.ancestors);

    // Layout report contains the new kinds.
    let report = inspect::report(ed.session());
    let nodes = &report[0].nodes;
    assert!(nodes.iter().any(|n| n.id == "hdr-row" && n.kind == "row"));
    assert!(nodes.iter().any(|n| n.id == "kpi-grid" && n.kind == "grid"));
    assert!(nodes.iter().any(|n| n.id == "sep-1" && n.kind == "rule"));
    assert!(nodes.iter().any(|n| n.id == "g-wide" && n.kind == "cell"));

    // Ops (`set`, `insert`, `move`, `remove`) work inside row/grid cells.
    let set_op: Op = serde_json::from_value(json!({
        "op": "set", "id": "p-left", "path": "/text", "value": "Updated ACME"
    })).unwrap();
    ed.edit(vec![set_op]).expect("set inside row cell");
    assert!(page_text(ed.session(), 1).contains("Updated ACME"));

    let ins_op: Op = serde_json::from_value(json!({
        "op": "insert", "id": "g-a", "field": "blocks", "value": {"type": "p", "id": "p-a2", "text": "Sub-metric"}
    })).unwrap();
    ed.edit(vec![ins_op]).expect("insert inside grid cell");
    assert!(page_text(ed.session(), 1).contains("Sub-metric"));

    let move_op: Op = serde_json::from_value(json!({
        "op": "move", "id": "p-a2", "to": {"id": "g-b"}, "field": "blocks"
    })).unwrap();
    ed.edit(vec![move_op]).expect("move between grid cells");
    let p_a2_rect = ed.rects("p-a2")[0].rect;
    let g_b_rect = ed.rects("g-b")[0].rect;
    assert!(p_a2_rect.x0 >= g_b_rect.x0 - 1.0, "moved block should now sit in g-b");

    let rem_op: Op = serde_json::from_value(json!({"op": "remove", "id": "p-a2"})).unwrap();
    ed.edit(vec![rem_op]).expect("remove from grid cell");
    assert!(!page_text(ed.session(), 1).contains("Sub-metric"));
}

#[test]
fn barcodes_render_all_formats_as_elements_and_blocks() {
    let doc = base_doc(json!([
        {
            "type": "section",
            "id": "bc-sec",
            "master": "a4",
            "flow": [
                {"type": "barcode", "id": "bc-128", "data": "INV-2026-0042", "format": "code128", "w": 200, "h": 54, "color": "ink", "background": "#ffffff"},
                {"type": "barcode", "id": "bc-gs1", "data": "(01)09506000134352", "format": "gs1-128", "w": 220, "h": 54},
                {"type": "barcode", "id": "bc-ean", "data": "400638133393", "format": "ean13", "w": 160, "h": 60},
                {"type": "barcode", "id": "bc-isbn", "data": "978-0-306-40615-7", "format": "isbn13", "w": 160, "h": 60},
                {"type": "barcode", "id": "bc-upc", "data": "03600029145", "format": "upca", "w": 160, "h": 60},
                {"type": "barcode", "id": "bc-39", "data": "SKU-42A", "format": "code39", "w": 180, "h": 48, "show_text": false}
            ]
        },
        {
            "type": "page",
            "id": "bc-page",
            "master": "a4",
            "elements": [
                {"type": "barcode", "id": "bc-el", "data": "PLACED-99", "format": "code128", "x": 60, "y": 80, "w": 180, "h": 56, "quiet": 12, "text_style": "mono"}
            ]
        }
    ]));

    let s = open(doc, &[]);
    assert_eq!(s.page_count(), 2);

    let p1 = page_text(&s, 1);
    assert!(p1.contains("INV-2026-0042"), "code128 label: {p1}");
    assert!(p1.contains("4 006381 333931"), "ean13 check digit computed: {p1}");
    assert!(p1.contains("978-0-306-40615-7"), "isbn13 canonical digits: {p1}");
    assert!(p1.contains("0 36000 29145 2"), "upca check digit computed: {p1}");
    assert!(!p1.contains("SKU-42A"), "show_text=false omits label: {p1}");

    let p2 = page_text(&s, 2);
    assert!(p2.contains("PLACED-99"), "placed barcode label: {p2}");

    let pdf = s.pdf().expect("PDF with barcodes");
    assert!(pdf.starts_with(b"%PDF-"));

    // Validation catches bad formats / payloads.
    let bad = base_doc(json!([{
        "type": "section", "id": "s", "master": "a4", "flow": [
            {"type": "barcode", "data": "123", "format": "ean13"},
            {"type": "barcode", "data": "abc", "format": "bogus"}
        ]
    }]));
    let bad_doc = Document::from_json(&bad.to_string()).unwrap();
    let probs = matenadar_model::validate(&bad_doc);
    assert!(probs.len() >= 2, "expected validation errors, got {probs:?}");
}

#[test]
fn charts_render_all_kinds_as_blocks_and_elements() {
    let doc = base_doc(json!([
        {
            "type": "section",
            "id": "ch-sec",
            "master": "a4",
            "flow": [
                {
                    "type": "chart", "id": "ch-bar", "kind": "bar", "w": 460, "h": 150,
                    "title": "Quarterly Revenue",
                    "labels": ["Q1", "Q2", "Q3", "Q4"],
                    "series": [
                        {"name": "2025", "color": "#2563eb", "values": [12, 18.5, 24, 31.2]},
                        {"name": "2026", "color": "#0f766e", "values": [15, 22, 29, 38]}
                    ],
                    "show_legend": true, "show_grid": true, "show_values": true,
                    "value_prefix": "$", "value_suffix": "k"
                },
                {
                    "type": "chart", "id": "ch-hbar", "kind": "bar", "orientation": "horizontal", "stacked": true,
                    "w": 460, "h": 120,
                    "labels": ["North", "South", "West"],
                    "series": [
                        {"name": "Print", "values": [40, 28, 35]},
                        {"name": "Digital", "values": [20, 32, 25]}
                    ]
                },
                {
                    "type": "chart", "id": "ch-line", "kind": "line", "w": 460, "h": 130,
                    "labels": ["Jan", "Feb", "Mar", "Apr"],
                    "series": [{"name": "Orders", "values": [120, 145, 138, 172]}]
                },
                {
                    "type": "chart", "id": "ch-area", "kind": "area", "stacked": true, "w": 460, "h": 120,
                    "labels": ["Jan", "Feb", "Mar"],
                    "series": [
                        {"name": "Direct", "values": [50, 65, 80]},
                        {"name": "Partner", "values": [30, 40, 45]}
                    ]
                }
            ]
        },
        {
            "type": "page",
            "id": "ch-page",
            "master": "a4",
            "elements": [
                {
                    "type": "chart", "id": "ch-donut", "kind": "donut",
                    "x": 48, "y": 60, "w": 240, "h": 160, "inner_radius": 0.55,
                    "title": "Mix",
                    "labels": ["Books", "Catalogues", "Slides"],
                    "series": [{"name": "Share", "values": [52, 31, 17]}]
                },
                {
                    "type": "chart", "id": "ch-pie", "kind": "pie",
                    "x": 310, "y": 60, "w": 220, "h": 160,
                    "labels": ["A", "B"],
                    "series": [{"values": [70, 30]}]
                },
                {
                    "type": "chart", "id": "ch-spark", "kind": "sparkline",
                    "x": 48, "y": 260, "w": 140, "h": 28, "show_values": true, "value_suffix": "%",
                    "series": [{"values": [42, 48, 46, 55, 63]}]
                }
            ]
        }
    ]));

    let s = open(doc, &[]);
    assert!(s.page_count() >= 2);
    let p1 = page_text(&s, 1);
    assert!(p1.contains("Quarterly Revenue"), "chart title should be live text: {p1}");
    assert!(p1.contains("$38k"), "formatted value label should appear: {p1}");
    assert!(p1.contains("2025") && p1.contains("2026"), "legend labels should appear: {p1}");

    let last = s.page_count();
    let p_last = page_text(&s, last);
    assert!(p_last.contains("Mix"), "donut title on page {last}: {p_last}");
    assert!(p_last.contains("Books"), "donut legend on page {last}: {p_last}");
    assert!(p_last.contains("63%"), "sparkline last value on page {last}: {p_last}");

    let pdf = s.pdf().expect("PDF with charts");
    assert!(pdf.starts_with(b"%PDF-"));
}

#[test]
fn pdf_standards_and_facturx_embedded_attachments() {
    let facturx_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100">
  <rsm:ExchangedDocument><ram:ID xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100">INV-2026-001</ram:ID></rsm:ExchangedDocument>
</rsm:CrossIndustryInvoice>
"#;

    // 1. Document-model `pdf` config with PDF/A-3b + Factur-X attachment
    let mut doc_a3 = base_doc(json!([{
        "type": "section", "id": "inv", "master": "a4",
        "flow": [
            {"type": "h", "id": "h1", "level": 1, "text": "Invoice INV-2026-001"},
            {"type": "p", "id": "p1", "text": "Total due: EUR 1,250.00"}
        ]
    }]));
    doc_a3["pdf"] = json!({
        "standard": "a-3b",
        "tagged": true,
        "attachments": [{
            "name": "factur-x.xml",
            "src": "data/factur-x.xml",
            "mime": "text/xml",
            "description": "Factur-X / ZUGFeRD invoice XML",
            "relationship": "data"
        }]
    });
    let s_a3 = open(doc_a3, &[("data/factur-x.xml", facturx_xml.to_vec())]);
    let pdf_a3 = s_a3.pdf().expect("PDF/A-3b with doc.pdf attachment");
    assert!(pdf_a3.starts_with(b"%PDF-"));
    assert!(contains_bytes(&pdf_a3, b"/EmbeddedFiles"), "PDF/A-3b must contain /EmbeddedFiles");
    assert!(contains_bytes(&pdf_a3, b"/AF"), "PDF/A-3b must contain /AF");
    assert!(
        contains_bytes(&pdf_a3, b"/AFRelationship/Data") || contains_bytes(&pdf_a3, b"/AFRelationship /Data"),
        "PDF/A-3b must contain /AFRelationship /Data"
    );
    assert!(contains_bytes(&pdf_a3, b"factur-x.xml"), "PDF/A-3b must contain filename factur-x.xml");
    assert!(contains_bytes(&pdf_a3, b"pdfaid:part"), "PDF/A-3b must include PDF/A XMP metadata");

    // 2. Programmatic `Session::pdf_with_options` with in-memory attachment
    let doc_plain = base_doc(json!([{
        "type": "section", "id": "inv2", "master": "a4",
        "flow": [
            {"type": "h", "id": "h1", "level": 1, "text": "Invoice INV-2026-002"},
            {"type": "p", "id": "p1", "text": "Programmatic attachment test"}
        ]
    }]));
    let s_plain = open(doc_plain, &[]);
    let pdf_prog = s_plain
        .pdf_with_options(&PdfOptions {
            standards: vec!["a-3b".into()],
            tagged: Some(true),
            attachments: vec![PdfAttachmentData {
                name: "factur-x.xml".into(),
                data: facturx_xml.to_vec(),
                mime: Some("text/xml".into()),
                description: Some("Factur-X programmatic".into()),
                relationship: Some("data".into()),
            }],
            max_ppi: None,
            timestamp: Some("2026-11-26T12:00:00Z".into()),
        })
        .expect("programmatic PDF/A-3b");
    assert!(contains_bytes(&pdf_prog, b"/EmbeddedFiles"));
    assert!(contains_bytes(&pdf_prog, b"/AF"));
    assert!(contains_bytes(&pdf_prog, b"/AFRelationship/Data") || contains_bytes(&pdf_prog, b"/AFRelationship /Data"));
    assert!(contains_bytes(&pdf_prog, b"factur-x.xml"));

    // 3. PDF/A-2b and PDF/UA-1
    let pdf_a2b = s_plain
        .pdf_with_options(&PdfOptions { standards: vec!["a-2b".into()], ..Default::default() })
        .expect("PDF/A-2b export");
    assert!(contains_bytes(&pdf_a2b, b"pdfaid:part"));

    let pdf_ua1 = s_plain
        .pdf_with_options(&PdfOptions { standards: vec!["ua-1".into()], tagged: Some(true), ..Default::default() })
        .expect("PDF/UA-1 export");
    assert!(contains_bytes(&pdf_ua1, b"pdfuaid:part"));

    // 4. Unknown standard fails cleanly
    assert!(
        s_plain
            .pdf_with_options(&PdfOptions { standards: vec!["not-a-standard".into()], ..Default::default() })
            .is_err()
    );
}

#[test]
fn speaker_notes_and_master_presets() {
    let doc = json!({
        "matenadar": "1",
        "meta": {"title": "Quarterly Deck", "lang": "en"},
        "design": {
            "base": {"font": "Libertinus Serif", "size": 18, "line_height": 1.3},
            "masters": {
                "slide": {"preset": "16:9", "margins": {"top": 48, "bottom": 48, "inside": 60, "outside": 60}},
                "slide-override": {"inherits": "slide", "width": "12in"},
                "square": {"preset": "social"},
                "a4-land": {"preset": "a4-landscape"}
            }
        },
        "content": [
            {
                "type": "page",
                "id": "slide-1",
                "master": "slide",
                "notes": "Welcome the audience and pause before revealing Q4 numbers.",
                "elements": [
                    {"type": "text", "id": "t1", "x": 60, "y": 100, "w": 600, "text": "Q4 Roadmap"}
                ]
            },
            {
                "type": "section",
                "id": "slide-2",
                "master": "slide-override",
                "notes": ["Mention ", {"text": "deterministic", "em": true}, " WASM rendering."],
                "flow": [
                    {"type": "p", "id": "p2", "text": "Key Highlights"}
                ]
            },
            {
                "type": "page",
                "id": "post-1",
                "master": "square",
                "elements": [{"type": "text", "id": "t3", "x": 40, "y": 40, "w": 200, "text": "Social card"}]
            },
            {
                "type": "page",
                "id": "sheet-1",
                "master": "a4-land",
                "elements": [{"type": "text", "id": "t4", "x": 40, "y": 40, "w": 200, "text": "Landscape sheet"}]
            }
        ]
    });

    let s = open(doc, &[]);
    let pages = s.pages();
    assert_eq!(pages.len(), 4);

    // 16:9 -> 960 x 540 pt
    assert!((pages[0].trim.w - 960.0).abs() < 0.01);
    assert!((pages[0].trim.h - 540.0).abs() < 0.01);
    // Inherited 16:9 with width override -> 12in (864pt) x 540pt
    assert!((pages[1].trim.w - 864.0).abs() < 0.01);
    assert!((pages[1].trim.h - 540.0).abs() < 0.01);
    // social -> 1080 x 1080 pt
    assert!((pages[2].trim.w - 1080.0).abs() < 0.01);
    assert!((pages[2].trim.h - 1080.0).abs() < 0.01);
    // a4-landscape -> 297mm x 210mm
    assert!((pages[3].trim.w - 297.0 * 72.0 / 25.4).abs() < 0.1);
    assert!((pages[3].trim.h - 210.0 * 72.0 / 25.4).abs() < 0.1);

    // Speaker notes exposed via `notes(page)` and `PageInfo.notes`, never drawn on the page.
    assert_eq!(s.notes(1).as_deref(), Some("Welcome the audience and pause before revealing Q4 numbers."));
    assert_eq!(pages[0].notes.as_deref(), Some("Welcome the audience and pause before revealing Q4 numbers."));
    assert_eq!(s.notes(2).as_deref(), Some("Mention deterministic WASM rendering."));
    assert_eq!(s.notes(3), None);
    assert_eq!(pages[2].notes, None);

    let svg1 = s.svg(1, false).unwrap();
    assert!(!svg1.contains("pause before revealing"), "speaker notes must not appear in rendered SVG");
    assert!(!page_text(&s, 1).contains("pause before revealing"));
    assert!(!page_text(&s, 2).contains("deterministic"));
}

#[test]
fn deterministic_max_ppi_downsamples_oversized_rasters_only() {
    // Build a deterministic 900x900 high-entropy RGB PNG (non-trivial compression).
    let (w, h) = (900u32, 900u32);
    let mut rgb = vec![0u8; (w * h * 3) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 3) as usize;
            let hash = x.wrapping_mul(2654435761) ^ y.wrapping_mul(2246822519);
            rgb[i] = ((x * 255 / w) ^ (hash & 0x3f)) as u8;
            rgb[i + 1] = ((y * 255 / h) ^ ((hash >> 8) & 0x3f)) as u8;
            rgb[i + 2] = (((x + y) * 127 / w) ^ ((hash >> 16) & 0x3f)) as u8;
        }
    }
    let mut big_png = Vec::new();
    {
        let img = image::RgbImage::from_raw(w, h, rgb).unwrap();
        let mut cursor = std::io::Cursor::new(&mut big_png);
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    }
    let tiny_png = include_bytes!("black.png").to_vec();
    let sample_svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="72" height="72"><circle cx="36" cy="36" r="30" fill="#2563eb"/></svg>"##.to_vec();

    // Place 900x900 photo at 1in x 1in (900 PPI), tiny 8x8 at 1in x 1in (8 PPI), and an SVG.
    let doc = base_doc(json!([{
        "type": "page",
        "id": "p1",
        "master": "a4",
        "elements": [
            {"type": "image", "id": "big", "src": "img/big.png", "x": 48, "y": 48, "w": "1in", "h": "1in"},
            {"type": "image", "id": "tiny", "src": "img/tiny.png", "x": 160, "y": 48, "w": "1in", "h": "1in"},
            {"type": "image", "id": "vec", "src": "img/vec.svg", "x": 272, "y": 48, "w": "1in", "h": "1in"}
        ]
    }]));

    let files = [
        ("img/big.png", big_png),
        ("img/tiny.png", tiny_png),
        ("img/vec.svg", sample_svg),
    ];
    let s = open(doc.clone(), &files);
    let pdf_full = s.pdf().expect("full-res PDF");
    let pdf_150_a = s
        .pdf_with_options(&PdfOptions { max_ppi: Some(150.0), ..Default::default() })
        .expect("150 PPI PDF (1)");
    let pdf_150_b = s
        .pdf_with_options(&PdfOptions { max_ppi: Some(150.0), ..Default::default() })
        .expect("150 PPI PDF (2)");

    assert_eq!(pdf_150_a, pdf_150_b, "max_ppi resampling must be deterministic");
    assert!(
        pdf_150_a.len() * 3 < pdf_full.len(),
        "downsampled PDF ({} bytes) should be much smaller than full-res PDF ({} bytes)",
        pdf_150_a.len(),
        pdf_full.len()
    );

    // Document-level `print.max_ppi` produces the exact same downsampled PDF via `s.pdf()`.
    let mut doc_with_print = doc;
    doc_with_print["print"] = json!({"max_ppi": 150});
    let s_print = open(doc_with_print, &files);
    let pdf_from_doc = s_print.pdf().expect("PDF honoring print.max_ppi");
    assert_eq!(pdf_from_doc, pdf_150_a, "doc.print.max_ppi must match PdfOptions.max_ppi");
}
