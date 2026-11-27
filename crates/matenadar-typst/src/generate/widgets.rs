//! Structured vector and layout elements: rulings, calendars, framed panel
//! grids, and callout/speech balloons. Lines and dots are drawn as vector SVG;
//! words remain live text.

use matenadar_model::Length;
use matenadar_model::block::{Para, Track, content_of};
use matenadar_model::element::{Element, ImageEl, Place};
use matenadar_model::style::{Stroke, TextStyle};
use matenadar_model::units::num;
use matenadar_model::widgets::{Balloon, BarcodeSpec, Calendar, ChartSpec, Panels, Ruling};

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

    pub(super) fn barcode_body(&mut self, spec: &BarcodeSpec, w: f64, h: f64) -> String {
        let bc = match crate::barcode::encode(&spec.data, spec.format.as_deref(), spec.quiet) {
            Ok(bc) => bc,
            Err(err) => {
                self.problem(err);
                return "none".into();
            }
        };
        let bar_svg = spec.color.as_ref().map(|c| self.pal.svg(c)).unwrap_or_else(|| "#000000".into());
        let bar_typ = spec.color.as_ref().map(|c| self.pal.color(c)).unwrap_or_else(|| "black".into());
        let bg_svg = spec.background.as_ref().map(|c| self.pal.svg(c));
        let bg_typ = spec.background.as_ref().map(|c| self.pal.color(c));
        let show_text = spec.show_text.unwrap_or(true);
        let text_h = if show_text { (h * 0.22).clamp(8.5, 13.0) } else { 0.0 };
        let bars_h = (h - text_h).max(8.0);
        let svg_doc = crate::barcode::to_svg(&bc, w, bars_h, &bar_svg, bg_svg.as_deref(), show_text);
        let file = self.file("svg", svg_doc.into_bytes());
        if !show_text {
            return format!(
                "box(width: {}pt, height: {}pt, image({}, width: {}pt, height: {}pt))",
                num(w),
                num(h),
                str_lit(&file),
                num(w),
                num(h)
            );
        }
        let font_size = (text_h * 0.78).clamp(6.5, 10.5);
        let has_size = spec.text_style.as_deref().is_some_and(|n| {
            self.doc.design.styles.get(n).and_then(|s| s.text.size).is_some()
                || self.doc.design.spans.get(n).and_then(|s| s.size).is_some()
        });
        let ts = self.text_of(spec.text_style.as_deref(), (!has_size).then_some(font_size));
        let fill_arg = bg_typ.map(|bg| format!(", fill: {bg}")).unwrap_or_default();
        let label = super::escape::escape(&bc.label).0;
        format!(
            "box(width: {}pt, height: {}pt{fill_arg}, {{ \
                place(top + left, image({}, width: {}pt, height: {}pt)); \
                let ts = {ts}; \
                place(top + left, dx: 0pt, dy: {}pt, box(width: {}pt, height: {}pt, align(center + horizon, {{ set text(fill: {bar_typ}, tracking: 0.06em); mt-span(ts, text(top-edge: \"cap-height\", bottom-edge: \"baseline\", [{label}])) }}))); \
            }})",
            num(w),
            num(h),
            str_lit(&file),
            num(w),
            num(bars_h),
            num(bars_h),
            num(w),
            num(text_h),
        )
    }

    pub(super) fn chart_body(&mut self, spec: &ChartSpec, w: f64, h: f64) -> String {
        let rendered = crate::chart::render(spec, w, h, &|c| self.pal.svg(c));
        let file = self.file("svg", rendered.svg.into_bytes());
        let cs = self.text_of(spec.style.as_deref(), None);
        let mut labels_out = String::new();
        for lbl in &rendered.labels {
            let al = match lbl.align {
                "center" => "center",
                "right" => "right",
                _ => "left",
            };
            let weight = if lbl.bold { ", weight: \"bold\"" } else { "" };
            let fill = if lbl.muted { ", fill: luma(42%)" } else { "" };
            let escaped = super::escape::escape(&lbl.text).0;
            labels_out.push_str(&format!(
                "place(top + left, dx: {}pt, dy: {}pt, box(width: {}pt, height: {}pt, align({al} + horizon, mt-span(cs, text(size: {}pt{weight}{fill}, top-edge: \"cap-height\", bottom-edge: \"baseline\", [{escaped}]))))); ",
                num(lbl.x),
                num(lbl.y),
                num(lbl.w),
                num(lbl.h),
                num(lbl.size_pt),
            ));
        }
        format!(
            "box(width: {}pt, height: {}pt, {{ \
                let cs = {cs}; \
                place(top + left, image({}, width: {}pt, height: {}pt)); \
                {labels_out}\
            }})",
            num(w),
            num(h),
            str_lit(&file),
            num(w),
            num(h),
        )
    }
}
