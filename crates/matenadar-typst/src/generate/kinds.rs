//! Elements for book kinds: mazes, logic grids, dot-to-dot, rulings,
//! calendars, paper-craft sheets, comic panels and balloons. Lines and dots
//! are drawn as SVG (vector, one drawing per element); words are live text.

use std::collections::BTreeMap;

use matenadar_model::Length;
use matenadar_model::block::{Para, Track, content_of};
use matenadar_model::element::{Clip, Element, ImageEl, Place};
use matenadar_model::kinds::{Balloon, Calendar, Craft, Dots, LogicGrid, Maze, Panels, Ruling};
use matenadar_model::style::{Stroke, TextStyle};
use matenadar_model::units::num;

use super::Gen;
use super::typ::str_lit;

/// A stroke as SVG draws it.
struct Pen {
    width: f64,
    color: String,
    /// `stroke-dasharray`, and whether its dots want round caps.
    dash: Option<(String, bool)>,
}

impl Pen {
    fn attrs(&self) -> String {
        let mut out = format!(r#" stroke="{}" stroke-width="{}""#, self.color, num(self.width));
        if let Some((d, round)) = &self.dash {
            out.push_str(&format!(r#" stroke-dasharray="{d}""#));
            if *round {
                out.push_str(r#" stroke-linecap="round""#);
            }
        }
        out
    }
}

fn pt_of(l: Option<Length>, default: f64) -> f64 {
    l.and_then(|v| v.pt()).unwrap_or(default)
}

/// `"M x y L x y …"` for a list of points.
fn polyline(pts: &[(f64, f64)]) -> String {
    let mut d = String::new();
    for (i, (x, y)) in pts.iter().enumerate() {
        d.push_str(&format!("{}{} {} ", if i == 0 { "M " } else { "L " }, num(*x), num(*y)));
    }
    d.trim_end().to_string()
}

fn svg(w: f64, h: f64, pad: f64, body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}pt" height="{}pt" viewBox="{} {} {} {}">{body}</svg>"#,
        num(w + 2.0 * pad),
        num(h + 2.0 * pad),
        num(-pad),
        num(-pad),
        num(w + 2.0 * pad),
        num(h + 2.0 * pad)
    )
}

/// Intervals on one line, merged where they meet.
fn merge(mut v: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f64)> = Vec::new();
    for (a, b) in v {
        match out.last_mut() {
            Some(last) if a <= last.1 + 1e-9 => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

impl Gen<'_> {
    fn pen(&self, s: Option<&Stroke>, width: f64, color: &str, dash: Option<&str>) -> Pen {
        let mut pen = Pen { width, color: color.to_string(), dash: None };
        let mut dash_word = dash.map(str::to_string);
        let mut dash_list: Option<Vec<f64>> = None;
        match s {
            Some(Stroke::Color(c)) => pen.color = self.pal.svg(c),
            Some(Stroke::Full(spec)) => {
                if let Some(c) = &spec.color {
                    pen.color = self.pal.svg(c);
                }
                if let Some(w) = spec.width {
                    pen.width = w.resolve(11.0, 0.0);
                }
                match &spec.dash {
                    Some(serde_json::Value::String(w)) => dash_word = Some(w.clone()),
                    Some(serde_json::Value::Array(items)) => {
                        dash_list = Some(items.iter().filter_map(|i| serde_json::from_value::<Length>(i.clone()).ok()).map(|l| l.resolve(11.0, 0.0)).collect())
                    }
                    _ => {}
                }
            }
            None => {}
        }
        let w = pen.width.max(0.1);
        pen.dash = match (dash_list, dash_word.as_deref()) {
            (Some(list), _) => Some((list.iter().map(|v| num(*v)).collect::<Vec<_>>().join(" "), false)),
            (None, Some("dotted")) => Some((format!("0 {}", num(w * 2.5)), true)),
            (None, Some("dashed")) => Some((format!("{} {}", num(w * 5.0), num(w * 3.0)), false)),
            (None, Some("dash-dotted")) => Some((format!("{} {} 0 {}", num(w * 6.0), num(w * 2.5), num(w * 2.5)), true)),
            _ => None,
        };
        pen
    }

    /// The text settings a style name gives (a paragraph style's characters,
    /// or a character style), as a Typst dictionary.
    fn text_of(&mut self, style: Option<&str>, size: Option<f64>) -> String {
        let mut t = match style {
            Some(n) if self.doc.design.styles.contains_key(n) => self.doc.design.style(n).text,
            Some(n) => self.doc.design.spans.get(n).cloned().unwrap_or_default(),
            None => TextStyle::default(),
        };
        if let Some(s) = size {
            t.size = Some(Length::Pt(s));
        }
        self.text_dict(&t, None)
    }

    pub(super) fn maze(&mut self, m: &Maze, place: &Place) -> String {
        let grid = !m.rows.is_empty();
        let (cols, rows) = if grid {
            (m.rows.first().map(|r| r.chars().count()).unwrap_or(1).max(1) as f64, m.rows.len().max(1) as f64)
        } else {
            let s = m.size.unwrap_or([10.0, 10.0]);
            (s[0].max(1.0), s[1].max(1.0))
        };
        let cell = m.cell.and_then(|c| c.pt()).or_else(|| place.w.and_then(|w| w.pt()).map(|w| w / cols)).unwrap_or(12.0);
        let (w, h) = (cols * cell, rows * cell);
        let wall = self.pen(m.walls.as_ref(), (cell * 0.08).clamp(0.6, 2.5), "#000000", None);
        // Walls as segments in cells, then merged along each line.
        let mut segs: Vec<[f64; 4]> = m.segments.clone();
        if grid {
            for (r, row) in m.rows.iter().enumerate() {
                for (c, ch) in row.chars().enumerate() {
                    let bits = ch.to_digit(16).unwrap_or(0);
                    let (x, y) = (c as f64, r as f64);
                    if bits & 1 != 0 {
                        segs.push([x, y, x + 1.0, y]);
                    }
                    if bits & 2 != 0 {
                        segs.push([x + 1.0, y, x + 1.0, y + 1.0]);
                    }
                    if bits & 4 != 0 {
                        segs.push([x, y + 1.0, x + 1.0, y + 1.0]);
                    }
                    if bits & 8 != 0 {
                        segs.push([x, y, x, y + 1.0]);
                    }
                }
            }
        }
        let key = |v: f64| (v * 1000.0).round() as i64;
        let mut horizontal: BTreeMap<i64, Vec<(f64, f64)>> = BTreeMap::new();
        let mut vertical: BTreeMap<i64, Vec<(f64, f64)>> = BTreeMap::new();
        let mut other = Vec::new();
        for [x1, y1, x2, y2] in segs {
            if (y1 - y2).abs() < 1e-9 {
                horizontal.entry(key(y1)).or_default().push((x1.min(x2), x1.max(x2)));
            } else if (x1 - x2).abs() < 1e-9 {
                vertical.entry(key(x1)).or_default().push((y1.min(y2), y1.max(y2)));
            } else {
                other.push([x1, y1, x2, y2]);
            }
        }
        let mut d = String::new();
        for (y, spans) in horizontal {
            for (a, b) in merge(spans) {
                d.push_str(&format!("M {} {} H {} ", num(a * cell), num(y as f64 / 1000.0 * cell), num(b * cell)));
            }
        }
        for (x, spans) in vertical {
            for (a, b) in merge(spans) {
                d.push_str(&format!("M {} {} V {} ", num(x as f64 / 1000.0 * cell), num(a * cell), num(b * cell)));
            }
        }
        for [x1, y1, x2, y2] in other {
            d.push_str(&format!("M {} {} L {} {} ", num(x1 * cell), num(y1 * cell), num(x2 * cell), num(y2 * cell)));
        }
        let mut body = format!(r#"<path d="{}" fill="none"{} stroke-linecap="square" stroke-linejoin="miter"/>"#, d.trim_end(), wall.attrs());
        // A cell's middle, or a point in cells.
        let at = |p: &[f64; 2]| if grid { ((p[1] + 0.5) * cell, (p[0] + 0.5) * cell) } else { (p[0] * cell, p[1] * cell) };
        if m.solved == Some(true) && m.solution.len() > 1 {
            let path = self.pen(m.path.as_ref(), (cell * 0.22).clamp(1.0, 4.0), "#d0402b", None);
            let pts: Vec<(f64, f64)> = m.solution.iter().map(at).collect();
            body.push_str(&format!(r#"<path d="{}" fill="none"{} stroke-linecap="round" stroke-linejoin="round"/>"#, polyline(&pts), path.attrs()));
        }
        let mark = m.marks.as_ref().map(|c| self.pal.svg(c)).unwrap_or_else(|| "#d0402b".into());
        if let Some(s) = &m.start {
            let (x, y) = at(s);
            body.push_str(&format!(r#"<circle cx="{}" cy="{}" r="{}" fill="{mark}"/>"#, num(x), num(y), num(cell * 0.24)));
        }
        if let Some(e) = &m.end {
            let (x, y) = at(e);
            body.push_str(&format!(
                r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{mark}" stroke-width="{}"/>"#,
                num(x),
                num(y),
                num(cell * 0.22),
                num((cell * 0.08).max(0.8))
            ));
        }
        let pad = wall.width.max(cell * 0.3);
        let file = self.file("svg", svg(w, h, pad, &body).into_bytes());
        format!(
            "box(width: {}pt, height: {}pt, place(top + left, dx: -{}pt, dy: -{}pt, image({}, width: {}pt, height: {}pt)))",
            num(w),
            num(h),
            num(pad),
            num(pad),
            str_lit(&file),
            num(w + 2.0 * pad),
            num(h + 2.0 * pad)
        )
    }

    pub(super) fn logic_grid(&mut self, g: &LogicGrid) -> String {
        let cell = pt_of(g.cell, 14.0);
        let cats: Vec<String> = g
            .categories
            .iter()
            .map(|c| format!("({}, ({},))", str_lit(&c.name), c.items.iter().map(|i| str_lit(i)).collect::<Vec<_>>().join(", ")))
            .collect();
        let marks: Vec<String> = g.marks.iter().map(|m| format!("({}, {}, {}, {}, {})", m.a[0], m.a[1], m.b[0], m.b[1], str_lit(&m.mark))).collect();
        let st = self.text_of(g.style.as_deref(), None);
        let lines = g.lines.as_ref().map(|s| self.pal.stroke(s, false)).unwrap_or_else(|| "0.5pt + luma(45%)".into());
        let frame = g.frame.as_ref().map(|s| self.pal.stroke(s, false)).unwrap_or_else(|| "1.2pt + black".into());
        format!("mt-logic(({},), {}pt, ({}), {st}, {lines}, {frame})", cats.join(", "), num(cell), if marks.is_empty() { String::new() } else { format!("{},", marks.join(", ")) })
    }

    pub(super) fn dots(&mut self, d: &Dots, place: &Place) -> String {
        let (w, h) = (pt_of(place.w, 300.0), pt_of(place.h, 300.0));
        let vb = d.viewbox.unwrap_or([0.0, 0.0, w, h]);
        let (sx, sy) = (w / vb[2].max(1e-9), h / vb[3].max(1e-9));
        let pts: Vec<(f64, f64)> = d.points.iter().map(|p| ((p[0] - vb[0]) * sx, (p[1] - vb[1]) * sy)).collect();
        // Strokes run between breaks. One that ends on its first dot is
        // closed: the line goes back to that dot, which isn't numbered again.
        let mut starts: Vec<usize> = vec![0];
        starts.extend(d.breaks.iter().copied().filter(|b| *b > 0 && *b < pts.len()));
        starts.sort_unstable();
        starts.dedup();
        let strokes: Vec<(std::ops::Range<usize>, bool)> = starts
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let r = *s..starts.get(k + 1).copied().unwrap_or(pts.len());
                let closed = r.len() > 2 && {
                    let (a, b) = (pts[r.start], pts[r.end - 1]);
                    (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
                };
                (r, closed)
            })
            .collect();
        let dot = pt_of(d.dot, 3.0);
        let size = pt_of(d.size, 7.0);
        let color = d.color.as_ref().map(|c| self.pal.svg(c)).unwrap_or_else(|| "#000000".into());
        let mut body = String::new();
        if d.solved == Some(true) {
            let line = self.pen(d.line.as_ref(), 1.2, "#000000", None);
            let paths: Vec<String> = strokes.iter().filter(|(r, _)| r.len() > 1).map(|(r, _)| polyline(&pts[r.clone()])).collect();
            body.push_str(&format!(r#"<path d="{}" fill="none"{} stroke-linecap="round" stroke-linejoin="round"/>"#, paths.join(" "), line.attrs()));
        }
        let st = self.text_of(d.style.as_deref(), Some(size));
        let fill = d.color.as_ref().map(|c| format!(", fill: {}", self.pal.color(c))).unwrap_or_default();
        let gap = dot / 2.0 + size * 0.75;
        let unit = |ax: f64, ay: f64| {
            let l = (ax * ax + ay * ay).sqrt();
            if l < 1e-9 { (0.0, 0.0) } else { (ax / l, ay / l) }
        };
        let mut labels = String::new();
        let mut number = d.start.unwrap_or(1) as usize;
        for (r, closed) in &strokes {
            let real = if *closed { r.start..r.end - 1 } else { r.clone() };
            // A closed shape's numbers go outside it, even at its inner corners.
            let centre = {
                let n = real.len().max(1) as f64;
                let (sx, sy) = pts[real.clone()].iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
                (sx / n, sy / n)
            };
            for (k, i) in real.clone().enumerate() {
                let (x, y) = pts[i];
                body.push_str(&format!(r#"<circle cx="{}" cy="{}" r="{}" fill="{color}"/>"#, num(x), num(y), num(dot / 2.0)));
                let prev = if k > 0 { Some(pts[i - 1]) } else if *closed { Some(pts[real.end - 1]) } else { None };
                let next = if i + 1 < real.end { Some(pts[i + 1]) } else if *closed { Some(pts[real.start]) } else { None };
                let (dx, dy) = match d.labels.get(i) {
                    Some([dx, dy]) => (*dx, *dy),
                    None => {
                        // Outside the turn the line makes here, or beside a straight run.
                        let a = prev.map_or((0.0, 0.0), |p| unit(x - p.0, y - p.1));
                        let b = next.map_or((0.0, 0.0), |p| unit(x - p.0, y - p.1));
                        let mut dir = unit(a.0 + b.0, a.1 + b.1);
                        if dir == (0.0, 0.0) {
                            let along = if a != (0.0, 0.0) { a } else { (-b.0, -b.1) };
                            dir = if along == (0.0, 0.0) { (-0.7, -0.7) } else { (along.1, -along.0) };
                            if dir.1 > 0.0 {
                                dir = (-dir.0, -dir.1);
                            }
                        }
                        if *closed && dir.0 * (x - centre.0) + dir.1 * (y - centre.1) < 0.0 {
                            dir = (-dir.0, -dir.1);
                        }
                        (dir.0 * gap, dir.1 * gap)
                    }
                };
                labels.push_str(&format!(
                    "mt-at({}pt, {}pt, 0.5, 0.5, mt-span(sp, text(top-edge: \"cap-height\", bottom-edge: \"baseline\"{fill}, [{number}]))); ",
                    num(x + dx),
                    num(y + dy),
                ));
                number += 1;
            }
        }
        let file = self.file("svg", svg(w, h, dot, &body).into_bytes());
        format!(
            "box(width: {}pt, height: {}pt, {{ let sp = {st}; place(top + left, dx: -{}pt, dy: -{}pt, image({}, width: {}pt, height: {}pt)); {labels}}})",
            num(w),
            num(h),
            num(dot),
            num(dot),
            str_lit(&file),
            num(w + 2.0 * dot),
            num(h + 2.0 * dot)
        )
    }

    pub(super) fn ruling(&mut self, r: &Ruling, place: &Place) -> String {
        let (w, h) = (pt_of(place.w, 300.0), pt_of(place.h, 300.0));
        let pattern = r.pattern.as_deref().unwrap_or("lines");
        let pitch = pt_of(r.pitch, if pattern == "lines" { 21.6 } else { 14.4 }).max(2.0);
        let mut body = String::new();
        let line = |x1: f64, y1: f64, x2: f64, y2: f64| format!(r#"M {} {} L {} {} "#, num(x1), num(y1), num(x2), num(y2));
        // Grids and dots sit centred, so what is left over is even on every side.
        let span = |len: f64, step: f64| {
            let n = (len / step + 1e-6).floor() as usize;
            let start = (len - n as f64 * step) / 2.0;
            (0..=n).map(move |i| start + i as f64 * step)
        };
        match pattern {
            "dots" | "isodots" => {
                let pen = self.pen(r.stroke.as_ref(), 1.3, "#8f8f8f", None);
                let rad = pen.width / 2.0;
                if pattern == "dots" {
                    for y in span(h, pitch) {
                        for x in span(w, pitch) {
                            body.push_str(&format!(r#"<circle cx="{}" cy="{}" r="{}"/>"#, num(x), num(y), num(rad)));
                        }
                    }
                } else {
                    let rise = pitch * 3f64.sqrt() / 2.0;
                    for (k, y) in span(h, rise).enumerate() {
                        let off = if k % 2 == 1 { pitch / 2.0 } else { 0.0 };
                        for x in span(w - pitch / 2.0, pitch) {
                            body.push_str(&format!(r#"<circle cx="{}" cy="{}" r="{}"/>"#, num(x + off), num(y), num(rad)));
                        }
                    }
                }
                body = format!(r#"<g fill="{}">{body}</g>"#, pen.color);
            }
            "grid" | "graph" => {
                let minor = self.pen(r.stroke.as_ref(), 0.4, "#a8a8a8", None);
                let every = r.major.unwrap_or(5).max(1) as usize;
                let major = self.pen(r.major_stroke.as_ref(), 0.9, "#8a8a8a", None);
                let (mut thin, mut thick) = (String::new(), String::new());
                let xs: Vec<f64> = span(w, pitch).collect();
                let ys: Vec<f64> = span(h, pitch).collect();
                let (x0, x1, y0, y1) = (xs[0], *xs.last().unwrap_or(&w), ys[0], *ys.last().unwrap_or(&h));
                for (i, x) in xs.iter().enumerate() {
                    let target = if pattern == "graph" && i % every == 0 { &mut thick } else { &mut thin };
                    target.push_str(&line(*x, y0, *x, y1));
                }
                for (i, y) in ys.iter().enumerate() {
                    let target = if pattern == "graph" && i % every == 0 { &mut thick } else { &mut thin };
                    target.push_str(&line(x0, *y, x1, *y));
                }
                body.push_str(&format!(r#"<path d="{}" fill="none"{}/>"#, thin.trim_end(), minor.attrs()));
                if !thick.is_empty() {
                    body.push_str(&format!(r#"<path d="{}" fill="none"{}/>"#, thick.trim_end(), major.attrs()));
                }
            }
            _ => {
                let pen = self.pen(r.stroke.as_ref(), 0.5, "#9a9a9a", None);
                let mut d = String::new();
                let mut y = pt_of(r.first, pitch);
                while y <= h + 1e-6 {
                    d.push_str(&line(0.0, y, w, y));
                    y += pitch;
                }
                body.push_str(&format!(r#"<path d="{}" fill="none"{}/>"#, d.trim_end(), pen.attrs()));
                if let Some(m) = r.margin.and_then(|m| m.pt()) {
                    let mp = self.pen(r.margin_stroke.as_ref(), 0.6, "#d98c8c", None);
                    body.push_str(&format!(r#"<path d="{}" fill="none"{}/>"#, line(m, 0.0, m, h).trim_end(), mp.attrs()));
                }
            }
        }
        let pad = 2.0;
        let file = self.file("svg", svg(w, h, pad, &body).into_bytes());
        format!(
            "box(width: {}pt, height: {}pt, place(top + left, dx: -{pad}pt, dy: -{pad}pt, image({}, width: {}pt, height: {}pt)))",
            num(w),
            num(h),
            str_lit(&file),
            num(w + 2.0 * pad),
            num(h + 2.0 * pad)
        )
    }

    pub(super) fn calendar(&mut self, c: &Calendar, place: &Place) -> String {
        let (w, h) = (pt_of(place.w, 400.0), pt_of(place.h, 300.0));
        let sunday = c.week_start.as_deref() == Some("sunday");
        let (y, m) = (c.year as i64, c.month.clamp(1, 12));
        let first = matenadar_model::weekday(y, m, 1);
        let offset = if sunday { (first + 1) % 7 } else { first } as i64;
        let days = matenadar_model::days_in_month(y, m) as i64;
        let weeks = ((offset + days) as f64 / 7.0).ceil() as i64;
        let names: Vec<String> = if c.days.len() == 7 {
            c.days.clone()
        } else {
            let base = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
            (0..7).map(|i| base[(i + if sunday { 6 } else { 0 }) % 7].to_string()).collect()
        };
        let head = pt_of(c.header, 18.0).min(h * 0.4);
        let (cw, ch) = (w / 7.0, (h - head) / weeks.max(1) as f64);
        let num_st = self.text_of(c.style.as_deref(), None);
        let head_st = self.text_of(c.header_style.as_deref().or(c.style.as_deref()), None);
        let lines = c.lines.as_ref().map(|s| self.pal.stroke(s, false)).unwrap_or_else(|| "0.6pt + luma(55%)".into());
        let grey = c.outside_color.as_ref().map(|x| self.pal.color(x)).unwrap_or_else(|| "luma(68%)".into());
        let mut out = String::new();
        for (i, n) in names.iter().enumerate() {
            out.push_str(&format!(
                "place(top + left, dx: {}pt, dy: 0pt, box(width: {}pt, height: {}pt, align(center + horizon, mt-span(hs, [{}])))); ",
                num(i as f64 * cw),
                num(cw),
                num(head),
                super::escape::escape(n).0
            ));
        }
        let (py, pm) = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
        let prev_days = matenadar_model::days_in_month(py, pm) as i64;
        for k in 0..weeks * 7 {
            let (row, col) = (k / 7, k % 7);
            let day = k - offset + 1;
            let (label, faded) = if (1..=days).contains(&day) {
                (day.to_string(), false)
            } else if c.outside == Some(true) {
                (if day < 1 { (prev_days + day).to_string() } else { (day - days).to_string() }, true)
            } else {
                continue;
            };
            let fill = if faded { format!(", fill: {grey}") } else { String::new() };
            out.push_str(&format!(
                "place(top + left, dx: {}pt, dy: {}pt, mt-span(ns, text(top-edge: \"cap-height\", bottom-edge: \"baseline\"{fill}, [{label}]))); ",
                num(col as f64 * cw + 4.0),
                num(head + row as f64 * ch + 4.0),
            ));
        }
        // The grid: the cells under the weekday row.
        let mut grid = String::new();
        for i in 0..=7 {
            grid.push_str(&format!("place(top + left, line(start: ({}pt, {}pt), end: ({}pt, {}pt), stroke: {lines})); ", num(i as f64 * cw), num(head), num(i as f64 * cw), num(h)));
        }
        for j in 0..=weeks {
            let yy = head + j as f64 * ch;
            grid.push_str(&format!("place(top + left, line(start: (0pt, {}pt), end: ({}pt, {}pt), stroke: {lines})); ", num(yy), num(w), num(yy)));
        }
        format!("box(width: {}pt, height: {}pt, {{ let hs = {head_st}; let ns = {num_st}; {grid}{out}}})", num(w), num(h))
    }

    pub(super) fn craft(&mut self, c: &Craft, place: &Place) -> String {
        let (w, h) = (pt_of(place.w, 300.0), pt_of(place.h, 300.0));
        let vb = c.viewbox.unwrap_or([0.0, 0.0, w, h]);
        let (sx, sy) = (w / vb[2].max(1e-9), h / vb[3].max(1e-9));
        let at = |p: [f64; 2]| ((p[0] - vb[0]) * sx, (p[1] - vb[1]) * sy);
        let mut fills = String::new();
        let mut pictures: Vec<Element> = Vec::new();
        // Words on the sheet: where, at what angle, the text, and which of
        // its points sits there (its middle, or its left end).
        let mut labels: Vec<((f64, f64), f64, String, f64)> = Vec::new();
        for f in &c.faces {
            let pts: Vec<(f64, f64)> = f.points.iter().map(|p| at(*p)).collect();
            if pts.len() < 3 {
                continue;
            }
            if let Some(fill) = &f.fill {
                fills.push_str(&format!(r#"<path d="{} Z" fill="{}"/>"#, polyline(&pts), self.pal.svg(fill)));
            }
            let (x0, y0) = pts.iter().fold((f64::MAX, f64::MAX), |a, p| (a.0.min(p.0), a.1.min(p.1)));
            let (x1, y1) = pts.iter().fold((f64::MIN, f64::MIN), |a, p| (a.0.max(p.0), a.1.max(p.1)));
            if let Some(src) = &f.src {
                // The picture covers the face's box and is cut to its shape.
                let rel: Vec<(f64, f64)> = pts.iter().map(|(x, y)| (x - x0, y - y0)).collect();
                pictures.push(Element::Image(ImageEl {
                    place: Place { x: Some(Length::Pt(x0)), y: Some(Length::Pt(y0)), w: Some(Length::Pt(x1 - x0)), h: Some(Length::Pt(y1 - y0)), ..Place::default() },
                    src: src.clone(),
                    fit: Some("cover".into()),
                    crop: f.crop,
                    focus: f.focus,
                    filter: f.filter.clone(),
                    clip: Some(Clip::Shape(format!("{} Z", polyline(&rel)))),
                    ..ImageEl::default()
                }));
            }
            if let Some(l) = &f.label {
                let (cx, cy) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
                labels.push(((cx / pts.len() as f64, cy / pts.len() as f64), 0.0, l.clone(), 0.5));
            }
        }
        let cut = self.pen(c.cut.as_ref(), 0.9, "#000000", None);
        let mountain = self.pen(c.mountain.as_ref(), 0.8, "#000000", Some("dash-dotted"));
        let valley = self.pen(c.valley.as_ref(), 0.8, "#000000", Some("dashed"));
        let score = self.pen(c.score.as_ref(), 0.7, "#000000", Some("dotted"));
        let glue = c.glue.as_ref().map(|g| self.pal.svg(g)).unwrap_or_else(|| "#e4e4e4".into());
        let mut tabs = String::new();
        let mut by_kind: BTreeMap<&str, String> = BTreeMap::new();
        for e in &c.edges {
            let (a, b) = (at(e.from), at(e.to));
            let mut kind = e.kind.as_deref().unwrap_or("cut");
            if let Some(t) = &e.tab {
                // The tab: its sides at an angle to the edge, its outer edge parallel.
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let len = (dx * dx + dy * dy).sqrt().max(1e-9);
                let (ux, uy) = (dx / len, dy / len);
                let left = t.side.as_deref() != Some("right");
                let (nx, ny) = if left { (uy, -ux) } else { (-uy, ux) };
                let depth = t.depth * (sx + sy) / 2.0;
                let inset = (depth / t.angle.unwrap_or(45.0).to_radians().tan()).min(len * 0.45);
                let quad = [
                    a,
                    (a.0 + ux * inset + nx * depth, a.1 + uy * inset + ny * depth),
                    (b.0 - ux * inset + nx * depth, b.1 - uy * inset + ny * depth),
                    b,
                ];
                tabs.push_str(&format!(r#"<path d="{}" fill="{glue}"{} stroke-linejoin="round"/>"#, polyline(&quad), cut.attrs()));
                if let Some(l) = &t.label {
                    let mid = ((a.0 + b.0) / 2.0 + nx * depth / 2.0, (a.1 + b.1) / 2.0 + ny * depth / 2.0);
                    let mut angle = uy.atan2(ux).to_degrees();
                    if angle > 90.0 || angle < -90.0 {
                        angle += 180.0;
                    }
                    labels.push((mid, angle, l.clone(), 0.5));
                }
                // The edge itself is where the tab folds.
                kind = "valley";
            }
            by_kind.entry(kind).or_default().push_str(&format!("M {} {} L {} {} ", num(a.0), num(a.1), num(b.0), num(b.1)));
        }
        let mut body = fills;
        body.push_str(&tabs);
        for (kind, d) in &by_kind {
            let pen = match *kind {
                "mountain" => &mountain,
                "valley" => &valley,
                "score" => &score,
                _ => &cut,
            };
            body.push_str(&format!(r#"<path d="{}" fill="none"{} stroke-linejoin="round"/>"#, d.trim_end(), pen.attrs()));
        }
        for mk in &c.marks {
            let (x, y) = at(mk.at);
            let r = 6.0;
            if mk.kind.as_deref() == Some("dot") {
                body.push_str(&format!(r##"<circle cx="{}" cy="{}" r="2.2" fill="#000000"/>"##, num(x), num(y)));
            } else {
                body.push_str(&format!(
                    r##"<g fill="none" stroke="#000000" stroke-width="0.5"><circle cx="{x}" cy="{y}" r="{h}"/><path d="M {} {y} H {} M {x} {} V {}"/></g>"##,
                    num(x - r),
                    num(x + r),
                    num(y - r),
                    num(y + r),
                    x = num(x),
                    y = num(y),
                    h = num(r * 0.55)
                ));
            }
            if let Some(l) = &mk.label {
                labels.push(((x + r + 4.0, y), 0.0, l.clone(), 0.0));
            }
        }
        if c.scissors == Some(true)
            && let Some(e) = c.edges.iter().find(|e| e.kind.as_deref().unwrap_or("cut") == "cut" && e.tab.is_none())
        {
            let (a, b) = (at(e.from), at(e.to));
            let angle = (b.1 - a.1).atan2(b.0 - a.0).to_degrees();
            let (mx, my) = ((a.0 * 3.0 + b.0) / 4.0, (a.1 * 3.0 + b.1) / 4.0);
            // Two blades and two rings, open along the line.
            body.push_str(&format!(
                r##"<g transform="translate({} {}) rotate({})" fill="none" stroke="#000000" stroke-width="0.8"><circle cx="-7" cy="-3.2" r="2.6" fill="#ffffff"/><circle cx="-7" cy="3.2" r="2.6" fill="#ffffff"/><path d="M -4.8 -2 L 7 1.6 M -4.8 2 L 7 -1.6" stroke-linecap="round"/></g>"##,
                num(mx),
                num(my),
                num(angle)
            ));
        }
        let pad = 8.0;
        let file = self.file("svg", svg(w, h, pad, &body).into_bytes());
        let pictures = self.capture(|g| {
            for p in &pictures {
                g.element(p, false);
            }
        });
        let st = self.text_of(c.style.as_deref(), None);
        let mut words = String::new();
        for ((x, y), angle, text, fx) in labels {
            words.push_str(&format!(
                "mt-at({}pt, {}pt, {}, 0.5, mt-rot({}deg, mt-span(sp, text(top-edge: \"cap-height\", bottom-edge: \"baseline\", [{}])))); ",
                num(x),
                num(y),
                num(fx),
                num(angle),
                super::escape::escape(&text).0
            ));
        }
        format!(
            "box(width: {}pt, height: {}pt, {{ let sp = {st}; place(top + left, box(width: {}pt, height: {}pt)[{pictures}]); place(top + left, dx: -{pad}pt, dy: -{pad}pt, image({}, width: {}pt, height: {}pt)); {words}}})",
            num(w),
            num(h),
            num(w),
            num(h),
            str_lit(&file),
            num(w + 2.0 * pad),
            num(h + 2.0 * pad)
        )
    }

    pub(super) fn panels(&mut self, p: &Panels, place: &Place) -> String {
        let (w, h) = (pt_of(place.w, 400.0), pt_of(place.h, 600.0));
        let gutter = pt_of(p.gutter, 9.0);
        // Lengths first, then shares of what is left.
        let share = |tracks: Vec<Option<&Track>>, total: f64| -> Vec<f64> {
            let parsed: Vec<(f64, f64)> = tracks
                .iter()
                .map(|t| match t {
                    Some(Track::Length(l)) => (l.resolve(11.0, total), 0.0),
                    Some(Track::Word(word)) => (0.0, word.trim().strip_suffix("fr").and_then(|v| v.trim().parse().ok()).unwrap_or(1.0)),
                    None => (0.0, 1.0),
                })
                .collect();
            let fixed: f64 = parsed.iter().map(|p| p.0).sum();
            let fr: f64 = parsed.iter().map(|p| p.1).sum();
            let left = (total - fixed).max(0.0);
            parsed.iter().map(|(len, f)| if *f > 0.0 { left * f / fr.max(1e-9) } else { *len }).collect()
        };
        let heights = share(p.rows.iter().map(|r| r.h.as_ref()).collect(), h - gutter * (p.rows.len().saturating_sub(1)) as f64);
        let border = p.border.as_ref().map(|s| self.pal.stroke(s, false)).unwrap_or_else(|| "1.5pt + black".into());
        let ground = p.fill.as_ref().map(|f| self.pal.fill(f)).unwrap_or_else(|| "white".into());
        let radius = p.radius.map(|r| r.typst()).unwrap_or_else(|| "0pt".into());
        let mut out = String::new();
        let mut y = 0.0;
        for (row, rh) in p.rows.iter().zip(heights) {
            let widths = share(row.panels.iter().map(|pn| pn.w.as_ref()).collect(), w - gutter * (row.panels.len().saturating_sub(1)) as f64);
            let mut x = 0.0;
            for (pn, pw) in row.panels.iter().zip(widths) {
                let fill = pn.fill.as_ref().map(|f| self.pal.fill(f)).unwrap_or_else(|| ground.clone());
                let stroke = pn.border.as_ref().map(|s| self.pal.stroke(s, false)).unwrap_or_else(|| border.clone());
                let picture = match &pn.src {
                    Some(src) => {
                        let img = Element::Image(ImageEl {
                            place: Place { id: pn.id.clone(), x: Some(Length::ZERO), y: Some(Length::ZERO), w: Some(Length::Pt(pw)), h: Some(Length::Pt(rh)), ..Place::default() },
                            src: src.clone(),
                            fit: Some("cover".into()),
                            crop: pn.crop,
                            focus: pn.focus,
                            filter: pn.filter.clone(),
                            alt: pn.alt.clone(),
                            ..ImageEl::default()
                        });
                        self.capture(|g| g.element(&img, false))
                    }
                    None => String::new(),
                };
                let over = self.capture(|g| {
                    for e in &pn.elements {
                        g.element(e, false);
                    }
                });
                out.push_str(&format!(
                    "place(top + left, dx: {x}pt, dy: {y}pt, box(width: {w}pt, height: {h}pt, fill: {fill}, radius: {radius}, clip: true)[{picture}]); \
                     place(top + left, dx: {x}pt, dy: {y}pt, rect(width: {w}pt, height: {h}pt, fill: none, stroke: {stroke}, radius: {radius})); \
                     place(top + left, dx: {x}pt, dy: {y}pt, box(width: {w}pt, height: {h}pt)[{over}]); ",
                    x = num(x),
                    y = num(y),
                    w = num(pw),
                    h = num(rh),
                ));
                x += pw + gutter;
            }
            y += rh + gutter;
        }
        format!("box(width: {}pt, height: {}pt, {{ {out}}})", num(w), num(h))
    }

    pub(super) fn balloon(&mut self, b: &Balloon, place: &Place) -> String {
        let style = b.style.clone().or_else(|| self.doc.design.styles.contains_key("balloon").then(|| "balloon".to_string()));
        let st = match (&style, &b.set) {
            (Some(n), s) => self.style_with(n, s.as_ref()),
            (None, Some(s)) => self.style_with("", Some(s)),
            (None, None) => "mt-base".to_string(),
        };
        let para = Para { id: place.id.as_ref().map(|id| format!("{id}.p")), content: content_of(&b.text, &b.content), ..Para::default() };
        if let (Some(frame), Some(pid)) = (&place.id, &para.id) {
            self.owners.insert(pid.clone(), (frame.clone(), 0, b.text.is_some()));
        }
        let shape = b.shape.as_deref().unwrap_or("speech");
        let saved = std::mem::replace(&mut self.hug, true);
        let words = self.capture(|g| g.flow(&[matenadar_model::Block::P(para)], &st, false));
        self.hug = saved;
        let (fx, fy) = super::typ::anchor(place.anchor.as_deref());
        let tail = match (&b.tail, shape) {
            (Some([tx, ty]), s) if s != "caption" => {
                let x = place.x.unwrap_or(Length::ZERO);
                let y = place.y.unwrap_or(Length::ZERO);
                format!("({} - {}, {} - {})", tx.typst(), x.typst(), ty.typst(), y.typst())
            }
            _ => "none".into(),
        };
        let fill = b.fill.as_ref().map(|c| self.pal.color(c)).unwrap_or_else(|| "white".into());
        let stroke = b.stroke.as_ref().map(|s| self.pal.stroke(s, false)).unwrap_or_else(|| "1pt + black".into());
        let maxw = place.w.map(|w| w.typst()).unwrap_or_else(|| "none".into());
        let inset = b.inset.map(|i| i.typst()).unwrap_or_else(|| "0.5em".into());
        // The longest words, which no line can be narrower than.
        let plain = matenadar_model::block::plain_text(&content_of(&b.text, &b.content));
        let mut longest: Vec<&str> = plain.split_whitespace().collect();
        longest.sort_by_key(|w| std::cmp::Reverse(w.chars().count()));
        longest.dedup();
        let longest: Vec<String> = longest.iter().take(3).map(|w| str_lit(w)).collect();
        format!(
            "mt-balloon({}, [{words}], ({}{}), {st}, {maxw}, {inset}, {tail}, {}, {}, {fill}, {stroke})",
            str_lit(shape),
            longest.join(", "),
            if longest.len() == 1 { "," } else { "" },
            num(fx),
            num(fy)
        )
    }
}
