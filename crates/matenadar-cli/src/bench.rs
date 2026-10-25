//! `matenadar bench`: how fast a document lays out, and how fast it lays out
//! again after small edits, which is what an editor or an agent loop feels.

use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use matenadar_model::block::Block;
use matenadar_model::ops::Target;
use matenadar_model::{Item, Op};
use matenadar_typst::Session;
use serde_json::json;

pub fn bench(mut s: Session, edits: usize) -> Result<serde_json::Value> {
    let first = s.compile_time();
    // Paragraphs with ids and plain text, spread through the document.
    let mut targets = Vec::new();
    for item in &s.document().content {
        if let Item::Section(sec) = item {
            for b in &sec.flow {
                if let Block::P(p) = b
                    && let (Some(id), Some(_)) = (&p.id, &p.text)
                {
                    targets.push(id.clone());
                }
            }
        }
    }
    let mut times: Vec<Duration> = Vec::new();
    let mut pages_changed = Vec::new();
    if !targets.is_empty() {
        for i in 0..edits {
            let id = &targets[(i * 7919) % targets.len()];
            let start = Instant::now();
            let change = s.apply(&Op::InsertText { node: Target::id(id.clone()), offset: 0, text: "Edited ".into() }).map_err(|e| anyhow!("{e}"))?;
            times.push(start.elapsed());
            pages_changed.push(change.changed_pages.len());
            s.apply(&change.inverse).map_err(|e| anyhow!("{e}"))?;
        }
    }
    times.sort();
    let pct = |q: f64| times.get(((times.len() as f64 - 1.0) * q).round() as usize).map(|d| d.as_secs_f64() * 1000.0);
    Ok(json!({
        "pages": s.page_count(),
        "first_layout_ms": first.as_secs_f64() * 1000.0,
        "edits": times.len(),
        "edit_p50_ms": pct(0.5),
        "edit_p95_ms": pct(0.95),
        "edit_max_ms": times.last().map(|d| d.as_secs_f64() * 1000.0),
        "mean_pages_changed": if pages_changed.is_empty() { 0.0 } else { pages_changed.iter().sum::<usize>() as f64 / pages_changed.len() as f64 },
    }))
}
