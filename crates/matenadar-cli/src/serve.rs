//! `matenadar serve`: one document kept laid out in memory, driven by JSON
//! lines on stdin. Each request is `{"id": …, "method": …, "params": {…}}`
//! and gets `{"id": …, "result": …}` or `{"id": …, "error": "…"}` on stdout.
//!
//! Methods:
//! - `open`: `{"doc": {…} | "path": "doc.json", "root": "…", "fonts": [], "system_fonts": true}`
//! - `apply`: `{"ops": [op…]}`. Returns the changes, with inverses and changed pages.
//! - `render`: `{"page": 3, "format": "png" | "svg", "dpi": 100, "bleed": false, "out": "p3.png"}`
//! - `pdf`: `{"out": "book.pdf"}`
//! - `check`: `{"safe": "0.25in", "min_ppi": 300}`
//! - `layout`: `{"pages": [1, 2]}`
//! - `locate`: `{"page", "x", "y"}`
//! - `position`: `{"id"}`
//! - `document`: the current document
//! - `save`: `{"out": "doc.json"}`

use std::io::{BufRead, Write};
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use matenadar_model::{Document, Op};
use matenadar_typst::{CheckOptions, Paths, Session, check, inspect};
use serde_json::{Value, json};

pub fn serve() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut session: Option<Session> = None;
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                writeln!(stdout, "{}", json!({"id": null, "error": format!("not JSON: {e}")}))?;
                stdout.flush()?;
                continue;
            }
        };
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(json!({}));
        let reply = match handle(&mut session, method, &params) {
            Ok(result) => json!({"id": id, "result": result}),
            Err(e) => json!({"id": id, "error": format!("{e:#}")}),
        };
        writeln!(stdout, "{reply}")?;
        stdout.flush()?;
        if method == "quit" {
            break;
        }
    }
    Ok(())
}

fn handle(session: &mut Option<Session>, method: &str, p: &Value) -> Result<Value> {
    let str_of = |k: &str| p.get(k).and_then(Value::as_str);
    if method == "open" {
        let doc = match (p.get("doc"), str_of("path")) {
            (Some(d), _) => Document::from_json(&d.to_string()).map_err(|e| anyhow!(e))?,
            (None, Some(path)) => Document::from_json(&std::fs::read_to_string(path)?).map_err(|e| anyhow!(e))?,
            _ => return Err(anyhow!("open needs doc or path")),
        };
        let problems = matenadar_model::validate(&doc);
        if !problems.is_empty() {
            return Err(anyhow!("the document has problems: {}", serde_json::to_string(&problems)?));
        }
        let root = PathBuf::from(str_of("root").unwrap_or("."));
        let paths = Paths {
            root: root.canonicalize().unwrap_or(root),
            font_dirs: p.get("fonts").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(PathBuf::from).collect()).unwrap_or_default(),
            system_fonts: p.get("system_fonts").and_then(Value::as_bool).unwrap_or(true),
        };
        let s = Session::new(doc, paths).map_err(|e| anyhow!("{e}"))?;
        let out = json!({"pages": s.page_count(), "layout_ms": s.compile_time().as_secs_f64() * 1000.0, "problems": s.problems(), "warnings": s.warnings()});
        *session = Some(s);
        return Ok(out);
    }
    if method == "quit" {
        return Ok(json!(true));
    }
    let s = session.as_mut().ok_or_else(|| anyhow!("open a document first"))?;
    match method {
        "apply" => {
            let ops: Vec<Op> = serde_json::from_value(p.get("ops").cloned().unwrap_or(json!([])))?;
            let mut changes = Vec::new();
            for op in &ops {
                changes.push(s.apply(op).map_err(|e| anyhow!("{e}"))?);
            }
            Ok(serde_json::to_value(changes)?)
        }
        "render" => {
            let page = p.get("page").and_then(Value::as_u64).ok_or_else(|| anyhow!("page"))? as usize;
            let bleed = p.get("bleed").and_then(Value::as_bool).unwrap_or(false);
            let out = str_of("out").ok_or_else(|| anyhow!("out"))?;
            match str_of("format").unwrap_or("png") {
                "svg" => std::fs::write(out, s.svg(page, bleed).ok_or_else(|| anyhow!("no page {page}"))?)?,
                _ => {
                    let dpi = p.get("dpi").and_then(Value::as_f64).unwrap_or(100.0) as f32;
                    std::fs::write(out, s.png(page, dpi, bleed).ok_or_else(|| anyhow!("no page {page}"))?)?
                }
            }
            Ok(json!({"out": out}))
        }
        "pdf" => {
            let out = str_of("out").ok_or_else(|| anyhow!("out"))?;
            std::fs::write(out, s.pdf().map_err(|e| anyhow!("{e}"))?)?;
            Ok(json!({"out": out, "pages": s.page_count()}))
        }
        "check" => {
            let mut o = CheckOptions::default();
            if let Some(safe) = str_of("safe") {
                o.safe_pt = matenadar_model::Length::parse(safe).map_err(|e| anyhow!(e))?.resolve(11.0, 0.0);
            }
            if let Some(ppi) = p.get("min_ppi").and_then(Value::as_f64) {
                o.min_ppi = ppi;
            }
            Ok(serde_json::to_value(check(s, &o))?)
        }
        "layout" => {
            let wanted: Option<Vec<usize>> = p.get("pages").and_then(|v| serde_json::from_value(v.clone()).ok());
            let report: Vec<_> = inspect::report(s).into_iter().filter(|r| wanted.as_ref().is_none_or(|w| w.contains(&r.page))).collect();
            Ok(serde_json::to_value(report)?)
        }
        "locate" => {
            let f = |k: &str| p.get(k).and_then(Value::as_f64).ok_or_else(|| anyhow!("{k}"));
            Ok(match s.locate(f("page")? as usize, f("x")?, f("y")?) {
                Some((node, offset)) => json!({"node": node, "offset": offset}),
                None => Value::Null,
            })
        }
        "position" => {
            let id = str_of("id").ok_or_else(|| anyhow!("id"))?;
            Ok(match s.position(id) {
                Some((page, x, y)) => json!({"page": page, "x": x, "y": y}),
                None => Value::Null,
            })
        }
        "document" => Ok(serde_json::to_value(s.document())?),
        "save" => {
            let out = str_of("out").ok_or_else(|| anyhow!("out"))?;
            std::fs::write(out, s.document().to_json())?;
            Ok(json!({"out": out}))
        }
        other => Err(anyhow!("unknown method \"{other}\"")),
    }
}
