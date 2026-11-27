//! Print-crisp vector charts: SVG geometry paired with Typst live-text labels.
//! Uses `libm` for transcendental math so native and WebAssembly builds emit
//! bit-identical coordinates and SVG paths.

use matenadar_model::units::num;
use matenadar_model::widgets::ChartSpec;

const DEFAULT_PALETTE: [&str; 6] = [
    "#2563eb", // cobalt
    "#0f766e", // teal
    "#d97706", // amber
    "#dc2626", // crimson
    "#7c3aed", // violet
    "#475569", // slate
];

#[derive(Clone, Debug)]
pub struct TextLabel {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub align: &'static str, // "left", "center", "right"
    pub text: String,
    pub size_pt: f64,
    pub bold: bool,
    pub muted: bool,
}

#[derive(Clone, Debug)]
pub struct ChartRender {
    pub svg: String,
    pub labels: Vec<TextLabel>,
}

/// Renders `spec` into vector SVG geometry + positioned Typst text labels inside `w` × `h` pt.
pub fn render(
    spec: &ChartSpec,
    w: f64,
    h: f64,
    resolve_color: &dyn Fn(&matenadar_model::Color) -> String,
) -> ChartRender {
    let colors: Vec<String> = spec
        .series
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.color
                .as_ref()
                .map(resolve_color)
                .unwrap_or_else(|| DEFAULT_PALETTE[i % DEFAULT_PALETTE.len()].to_string())
        })
        .collect();
    let grid_color = spec
        .grid_color
        .as_ref()
        .map(resolve_color)
        .unwrap_or_else(|| "#e4e4e7".to_string());
    let axis_color = "#a1a1aa";

    let kind = spec.kind.trim().to_ascii_lowercase();
    let mut svg_body = String::with_capacity(2048);
    let mut labels = Vec::new();

    if kind == "sparkline" {
        render_sparkline(spec, w, h, &colors, &mut svg_body, &mut labels);
        return finish(w, h, svg_body, labels);
    }

    let mut top = 4.0;
    if let Some(title) = spec.title.as_deref().filter(|t| !t.trim().is_empty()) {
        labels.push(TextLabel {
            x: 0.0,
            y: 0.0,
            w,
            h: 16.0,
            align: "left",
            text: title.to_string(),
            size_pt: 10.5,
            bold: true,
            muted: false,
        });
        top += 20.0;
    }

    let is_pie = kind == "donut" || kind == "pie";
    let show_legend = spec
        .show_legend
        .unwrap_or(if is_pie { true } else { spec.series.len() > 1 });

    if is_pie {
        render_donut(spec, w, h, top, kind == "pie", show_legend, &colors, resolve_color, &mut svg_body, &mut labels);
        return finish(w, h, svg_body, labels);
    }

    // Legend row at top (under title) for cartesian charts.
    if show_legend && !spec.series.is_empty() {
        let mut lx = 0.0;
        let ly = top + 2.0;
        for (i, s) in spec.series.iter().enumerate() {
            let name = s.name.as_deref().unwrap_or("");
            let display = if name.is_empty() { format!("Series {}", i + 1) } else { name.to_string() };
            let item_w = (display.chars().count() as f64 * 4.8 + 18.0).min(w * 0.45);
            if lx + item_w > w && lx > 0.0 {
                lx = 0.0;
                top += 13.0;
            }
            svg_body.push_str(&format!(
                r#"<rect x="{}" y="{}" width="6" height="6" rx="1.5" fill="{}"/>"#,
                num(lx),
                num(top + 2.0),
                colors[i]
            ));
            labels.push(TextLabel {
                x: lx + 9.0,
                y: top,
                w: item_w - 9.0,
                h: 11.0,
                align: "left",
                text: display,
                size_pt: 7.5,
                bold: false,
                muted: true,
            });
            lx += item_w + 6.0;
        }
        let _ = ly;
        top += 16.0;
    }

    let horizontal = kind == "bar" && spec.orientation.as_deref() == Some("horizontal");
    let stacked = spec.stacked.unwrap_or(false);
    let show_grid = spec.show_grid.unwrap_or(true);
    let show_values = spec.show_values.unwrap_or(false);

    if horizontal {
        render_horizontal_bar(
            spec,
            w,
            h,
            top,
            stacked,
            show_grid,
            show_values,
            &colors,
            &grid_color,
            axis_color,
            &mut svg_body,
            &mut labels,
        );
    } else {
        render_cartesian(
            spec,
            &kind,
            w,
            h,
            top,
            stacked,
            show_grid,
            show_values,
            &colors,
            &grid_color,
            axis_color,
            &mut svg_body,
            &mut labels,
        );
    }

    finish(w, h, svg_body, labels)
}

fn finish(w: f64, h: f64, svg_body: String, labels: Vec<TextLabel>) -> ChartRender {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}pt" height="{h}pt" viewBox="0 0 {w} {h}">{svg_body}</svg>"#,
        w = num(w),
        h = num(h),
    );
    ChartRender { svg, labels }
}

fn format_value(v: f64, prefix: Option<&str>, suffix: Option<&str>) -> String {
    let pre = prefix.unwrap_or("");
    let suf = suffix.unwrap_or("");
    let body = if v.abs() >= 1_000_000.0 {
        format!("{}M", trim_dec(v / 1_000_000.0))
    } else if v.abs() >= 10_000.0 && v.fract().abs() < 1e-6 {
        format!("{}k", trim_dec(v / 1_000.0))
    } else {
        trim_dec(v)
    };
    format!("{pre}{body}{suf}")
}

fn trim_dec(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    let r = if r == 0.0 { 0.0 } else { r };
    if (r - r.round()).abs() < 1e-6 {
        format!("{}", r.round() as i64)
    } else {
        let s = format!("{r:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Computes clean rounded tick steps from `lo..=hi`.
fn nice_ticks(min_v: f64, max_v: f64, count: usize) -> (f64, f64, Vec<f64>) {
    let lo = min_v.min(0.0);
    let hi = max_v.max(0.0);
    let span = (hi - lo).abs();
    if span < 1e-9 {
        return (0.0, 1.0, vec![0.0, 0.5, 1.0]);
    }
    let raw_step = span / (count.max(2) as f64);
    let mag = libm::pow(10.0, libm::floor(libm::log10(raw_step)));
    let norm = raw_step / mag;
    let step = if norm <= 1.0 {
        1.0
    } else if norm <= 2.0 {
        2.0
    } else if norm <= 2.5 {
        2.5
    } else if norm <= 5.0 {
        5.0
    } else {
        10.0
    } * mag;
    let nice_lo = (lo / step).floor() * step;
    let nice_hi = (hi / step).ceil() * step;
    let mut ticks = Vec::new();
    let mut cur = nice_lo;
    let mut guard = 0;
    while cur <= nice_hi + step * 1e-4 && guard < 20 {
        ticks.push(if cur.abs() < step * 1e-6 { 0.0 } else { cur });
        cur += step;
        guard += 1;
    }
    (nice_lo, nice_hi, ticks)
}

fn data_range(spec: &ChartSpec, stacked: bool) -> (usize, f64, f64) {
    let n = spec
        .labels
        .len()
        .max(spec.series.iter().map(|s| s.values.len()).max().unwrap_or(0))
        .max(1);
    let mut min_v = 0.0_f64;
    let mut max_v = 0.0_f64;
    if stacked {
        for i in 0..n {
            let (mut pos, mut neg) = (0.0, 0.0);
            for s in &spec.series {
                let v = s.values.get(i).copied().unwrap_or(0.0);
                if v >= 0.0 { pos += v; } else { neg += v; }
            }
            max_v = max_v.max(pos);
            min_v = min_v.min(neg);
        }
    } else {
        for s in &spec.series {
            for &v in &s.values {
                max_v = max_v.max(v);
                min_v = min_v.min(v);
            }
        }
    }
    (n, min_v, max_v)
}

#[allow(clippy::too_many_arguments)]
fn render_cartesian(
    spec: &ChartSpec,
    kind: &str,
    w: f64,
    h: f64,
    top: f64,
    stacked: bool,
    show_grid: bool,
    show_values: bool,
    colors: &[String],
    grid_color: &str,
    axis_color: &str,
    svg: &mut String,
    labels: &mut Vec<TextLabel>,
) {
    let (n, min_v, max_v) = data_range(spec, stacked);
    let (lo, hi, ticks) = nice_ticks(min_v, max_v, 4);
    let max_tick_chars = ticks
        .iter()
        .map(|&t| format_value(t, spec.value_prefix.as_deref(), spec.value_suffix.as_deref()).chars().count())
        .max()
        .unwrap_or(2);
    let left = (max_tick_chars as f64 * 4.6 + 8.0).clamp(24.0, w * 0.28);
    let right = 6.0;
    let bottom = if spec.labels.is_empty() { 8.0 } else { 18.0 };
    let plot_x = left;
    let plot_y = top + if show_values { 10.0 } else { 4.0 };
    let plot_w = (w - plot_x - right).max(12.0);
    let plot_h = (h - plot_y - bottom).max(12.0);
    let span = (hi - lo).max(1e-9);
    let y_of = |v: f64| plot_y + plot_h * (1.0 - (v - lo) / span);
    let zero_y = y_of(0.0);

    // Horizontal gridlines and y-axis labels.
    for &t in &ticks {
        let y = y_of(t);
        let is_zero = t.abs() < span * 1e-6;
        if show_grid || is_zero {
            let stroke = if is_zero { axis_color } else { grid_color };
            let sw = if is_zero { "0.7" } else { "0.5" };
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{stroke}" stroke-width="{sw}"/>"#,
                num(plot_x),
                num(y),
                num(plot_x + plot_w),
                num(y),
            ));
        }
        labels.push(TextLabel {
            x: 0.0,
            y: y - 5.5,
            w: left - 5.0,
            h: 11.0,
            align: "right",
            text: format_value(t, spec.value_prefix.as_deref(), spec.value_suffix.as_deref()),
            size_pt: 7.0,
            bold: false,
            muted: true,
        });
    }

    // X-axis category labels.
    let slot_w = plot_w / n as f64;
    for (i, cat) in spec.labels.iter().take(n).enumerate() {
        let cx = if kind == "bar" || n == 1 {
            plot_x + (i as f64 + 0.5) * slot_w
        } else {
            plot_x + i as f64 * (plot_w / (n - 1) as f64)
        };
        let lw = slot_w.max(24.0);
        labels.push(TextLabel {
            x: (cx - lw / 2.0).clamp(0.0, (w - lw).max(0.0)),
            y: plot_y + plot_h + 4.0,
            w: lw,
            h: 12.0,
            align: "center",
            text: cat.clone(),
            size_pt: 7.2,
            bold: false,
            muted: true,
        });
    }

    match kind {
        "bar" => {
            let m = spec.series.len().max(1);
            let group_pad = (slot_w * 0.16).min(12.0);
            let group_w = (slot_w - 2.0 * group_pad).max(3.0);
            if stacked {
                for i in 0..n {
                    let bx = plot_x + i as f64 * slot_w + group_pad;
                    let (mut pos_acc, mut neg_acc) = (0.0, 0.0);
                    for (si, s) in spec.series.iter().enumerate() {
                        let v = s.values.get(i).copied().unwrap_or(0.0);
                        let (y0, y1) = if v >= 0.0 {
                            let y0 = y_of(pos_acc + v);
                            let y1 = y_of(pos_acc);
                            pos_acc += v;
                            (y0, y1)
                        } else {
                            let y0 = y_of(neg_acc);
                            let y1 = y_of(neg_acc + v);
                            neg_acc += v;
                            (y0, y1)
                        };
                        let bh = (y1 - y0).max(0.0);
                        if bh > 0.1 {
                            svg.push_str(&format!(
                                r#"<rect x="{}" y="{}" width="{}" height="{}" rx="1.2" fill="{}"/>"#,
                                num(bx),
                                num(y0),
                                num(group_w),
                                num(bh),
                                colors[si]
                            ));
                        }
                    }
                    if show_values && pos_acc > 0.0 {
                        labels.push(TextLabel {
                            x: bx - 4.0,
                            y: y_of(pos_acc) - 11.0,
                            w: group_w + 8.0,
                            h: 10.0,
                            align: "center",
                            text: format_value(pos_acc, spec.value_prefix.as_deref(), spec.value_suffix.as_deref()),
                            size_pt: 6.8,
                            bold: false,
                            muted: false,
                        });
                    }
                }
            } else {
                let inner_gap = if m > 1 { (group_w * 0.08).min(3.0) } else { 0.0 };
                let bar_w = ((group_w - inner_gap * (m - 1) as f64) / m as f64).max(2.0);
                for i in 0..n {
                    for (si, s) in spec.series.iter().enumerate() {
                        let v = s.values.get(i).copied().unwrap_or(0.0);
                        let bx = plot_x + i as f64 * slot_w + group_pad + si as f64 * (bar_w + inner_gap);
                        let yv = y_of(v);
                        let (ry, rh) = if v >= 0.0 { (yv, zero_y - yv) } else { (zero_y, yv - zero_y) };
                        if rh > 0.1 {
                            svg.push_str(&format!(
                                r#"<rect x="{}" y="{}" width="{}" height="{}" rx="1.5" fill="{}"/>"#,
                                num(bx),
                                num(ry),
                                num(bar_w),
                                num(rh),
                                colors[si]
                            ));
                        }
                        if show_values {
                            let ly = if v >= 0.0 { ry - 11.0 } else { ry + rh + 1.0 };
                            labels.push(TextLabel {
                                x: bx - 6.0,
                                y: ly,
                                w: bar_w + 12.0,
                                h: 10.0,
                                align: "center",
                                text: format_value(v, spec.value_prefix.as_deref(), spec.value_suffix.as_deref()),
                                size_pt: 6.8,
                                bold: false,
                                muted: false,
                            });
                        }
                    }
                }
            }
        }
        "line" | "area" => {
            let is_area = kind == "area";
            let x_of = |i: usize| {
                if n == 1 {
                    plot_x + plot_w / 2.0
                } else {
                    plot_x + i as f64 * (plot_w / (n - 1) as f64)
                }
            };
            let mut prev_stack = vec![0.0_f64; n];
            for (si, s) in spec.series.iter().enumerate() {
                if s.values.is_empty() {
                    continue;
                }
                let mut pts = Vec::with_capacity(s.values.len());
                let mut base_pts = Vec::with_capacity(s.values.len());
                for (i, &raw) in s.values.iter().take(n).enumerate() {
                    let base = if is_area && stacked { prev_stack[i] } else { 0.0 };
                    let v = base + raw;
                    if is_area && stacked {
                        prev_stack[i] = v;
                    }
                    pts.push((x_of(i), y_of(v), raw));
                    base_pts.push((x_of(i), y_of(base)));
                }
                if is_area && pts.len() >= 2 {
                    let mut ad = format!("M{} {}", num(pts[0].0), num(pts[0].1));
                    for &(px, py, _) in &pts[1..] {
                        ad.push_str(&format!(" L{} {}", num(px), num(py)));
                    }
                    for &(bx, by) in base_pts.iter().rev() {
                        ad.push_str(&format!(" L{} {}", num(bx), num(by)));
                    }
                    ad.push('Z');
                    let op = if stacked { "0.78" } else { "0.16" };
                    svg.push_str(&format!(r#"<path d="{ad}" fill="{}" fill-opacity="{op}"/>"#, colors[si]));
                }
                if pts.len() >= 2 {
                    let mut ld = format!("M{} {}", num(pts[0].0), num(pts[0].1));
                    for &(px, py, _) in &pts[1..] {
                        ld.push_str(&format!(" L{} {}", num(px), num(py)));
                    }
                    svg.push_str(&format!(
                        r#"<path d="{ld}" fill="none" stroke="{}" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>"#,
                        colors[si]
                    ));
                }
                for &(px, py, raw) in &pts {
                    svg.push_str(&format!(
                        r#"<circle cx="{}" cy="{}" r="2.4" fill="{}" stroke="white" stroke-width="0.9"/>"#,
                        num(px),
                        num(py),
                        colors[si]
                    ));
                    if show_values {
                        labels.push(TextLabel {
                            x: px - 16.0,
                            y: py - 13.0,
                            w: 32.0,
                            h: 10.0,
                            align: "center",
                            text: format_value(raw, spec.value_prefix.as_deref(), spec.value_suffix.as_deref()),
                            size_pt: 6.8,
                            bold: false,
                            muted: false,
                        });
                    }
                }
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn render_horizontal_bar(
    spec: &ChartSpec,
    w: f64,
    h: f64,
    top: f64,
    stacked: bool,
    show_grid: bool,
    show_values: bool,
    colors: &[String],
    grid_color: &str,
    axis_color: &str,
    svg: &mut String,
    labels: &mut Vec<TextLabel>,
) {
    let (n, min_v, max_v) = data_range(spec, stacked);
    let (lo, hi, ticks) = nice_ticks(min_v, max_v, 4);
    let max_lbl_chars = spec.labels.iter().map(|s| s.chars().count()).max().unwrap_or(4);
    let left = (max_lbl_chars as f64 * 4.6 + 10.0).clamp(32.0, w * 0.34);
    let right = if show_values { 32.0 } else { 10.0 };
    let bottom = 16.0;
    let plot_x = left;
    let plot_y = top + 2.0;
    let plot_w = (w - plot_x - right).max(16.0);
    let plot_h = (h - plot_y - bottom).max(16.0);
    let span = (hi - lo).max(1e-9);
    let x_of = |v: f64| plot_x + plot_w * ((v - lo) / span);
    let zero_x = x_of(0.0);

    for &t in &ticks {
        let x = x_of(t);
        let is_zero = t.abs() < span * 1e-6;
        if show_grid || is_zero {
            let stroke = if is_zero { axis_color } else { grid_color };
            let sw = if is_zero { "0.7" } else { "0.5" };
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{stroke}" stroke-width="{sw}"/>"#,
                num(x),
                num(plot_y),
                num(x),
                num(plot_y + plot_h),
            ));
        }
        labels.push(TextLabel {
            x: (x - 18.0).clamp(0.0, (w - 36.0).max(0.0)),
            y: plot_y + plot_h + 3.0,
            w: 36.0,
            h: 11.0,
            align: "center",
            text: format_value(t, spec.value_prefix.as_deref(), spec.value_suffix.as_deref()),
            size_pt: 7.0,
            bold: false,
            muted: true,
        });
    }

    let slot_h = plot_h / n as f64;
    let m = spec.series.len().max(1);
    let group_pad = (slot_h * 0.18).min(8.0);
    let group_h = (slot_h - 2.0 * group_pad).max(3.0);

    for i in 0..n {
        if let Some(cat) = spec.labels.get(i) {
            labels.push(TextLabel {
                x: 0.0,
                y: plot_y + i as f64 * slot_h,
                w: left - 6.0,
                h: slot_h,
                align: "right",
                text: cat.clone(),
                size_pt: 7.3,
                bold: false,
                muted: false,
            });
        }
        if stacked {
            let by = plot_y + i as f64 * slot_h + group_pad;
            let (mut pos_acc, mut neg_acc) = (0.0, 0.0);
            for (si, s) in spec.series.iter().enumerate() {
                let v = s.values.get(i).copied().unwrap_or(0.0);
                let (x0, x1) = if v >= 0.0 {
                    let x0 = x_of(pos_acc);
                    let x1 = x_of(pos_acc + v);
                    pos_acc += v;
                    (x0, x1)
                } else {
                    let x0 = x_of(neg_acc + v);
                    let x1 = x_of(neg_acc);
                    neg_acc += v;
                    (x0, x1)
                };
                let bw = (x1 - x0).max(0.0);
                if bw > 0.1 {
                    svg.push_str(&format!(
                        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="1.2" fill="{}"/>"#,
                        num(x0),
                        num(by),
                        num(bw),
                        num(group_h),
                        colors[si]
                    ));
                }
            }
            if show_values && pos_acc > 0.0 {
                labels.push(TextLabel {
                    x: x_of(pos_acc) + 4.0,
                    y: by,
                    w: right - 4.0,
                    h: group_h,
                    align: "left",
                    text: format_value(pos_acc, spec.value_prefix.as_deref(), spec.value_suffix.as_deref()),
                    size_pt: 6.8,
                    bold: false,
                    muted: false,
                });
            }
        } else {
            let inner_gap = if m > 1 { (group_h * 0.1).min(2.5) } else { 0.0 };
            let bar_h = ((group_h - inner_gap * (m - 1) as f64) / m as f64).max(2.0);
            for (si, s) in spec.series.iter().enumerate() {
                let v = s.values.get(i).copied().unwrap_or(0.0);
                let by = plot_y + i as f64 * slot_h + group_pad + si as f64 * (bar_h + inner_gap);
                let xv = x_of(v);
                let (rx, rw) = if v >= 0.0 { (zero_x, xv - zero_x) } else { (xv, zero_x - xv) };
                if rw > 0.1 {
                    svg.push_str(&format!(
                        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="1.5" fill="{}"/>"#,
                        num(rx),
                        num(by),
                        num(rw),
                        num(bar_h),
                        colors[si]
                    ));
                }
                if show_values {
                    labels.push(TextLabel {
                        x: rx + rw + 4.0,
                        y: by - 1.0,
                        w: right - 4.0,
                        h: bar_h + 2.0,
                        align: "left",
                        text: format_value(v, spec.value_prefix.as_deref(), spec.value_suffix.as_deref()),
                        size_pt: 6.8,
                        bold: false,
                        muted: false,
                    });
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_donut(
    spec: &ChartSpec,
    w: f64,
    h: f64,
    top: f64,
    is_pie: bool,
    show_legend: bool,
    series_colors: &[String],
    resolve_color: &dyn Fn(&matenadar_model::Color) -> String,
    svg: &mut String,
    labels: &mut Vec<TextLabel>,
) {
    // Support either 1 series with N values + labels, OR N single-value series.
    let slices: Vec<(String, f64, String)> = if spec.series.len() == 1 && spec.series[0].values.len() > 1 {
        spec.series[0]
            .values
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let name = spec.labels.get(i).cloned().unwrap_or_else(|| format!("Item {}", i + 1));
                let col = DEFAULT_PALETTE[i % DEFAULT_PALETTE.len()].to_string();
                (name, v.max(0.0), col)
            })
            .collect()
    } else {
        spec.series
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let name = s
                    .name
                    .clone()
                    .or_else(|| spec.labels.get(i).cloned())
                    .unwrap_or_else(|| format!("Item {}", i + 1));
                let v = s.values.first().copied().unwrap_or(0.0).max(0.0);
                let col = s
                    .color
                    .as_ref()
                    .map(resolve_color)
                    .unwrap_or_else(|| series_colors.get(i).cloned().unwrap_or_else(|| DEFAULT_PALETTE[i % DEFAULT_PALETTE.len()].to_string()));
                (name, v, col)
            })
            .collect()
    };

    let total: f64 = slices.iter().map(|s| s.1).sum();
    if total <= 0.0 {
        return;
    }

    let avail_h = (h - top - 4.0).max(20.0);
    let legend_w = if show_legend && w > avail_h * 1.45 { (w * 0.42).clamp(72.0, 180.0) } else { 0.0 };
    let legend_bottom = if show_legend && legend_w == 0.0 {
        ((slices.len() as f64 / 2.0).ceil() * 13.0 + 6.0).min(avail_h * 0.45)
    } else {
        0.0
    };

    let plot_w = (w - legend_w).max(20.0);
    let plot_h = (avail_h - legend_bottom).max(20.0);
    let outer_r = (plot_w.min(plot_h) * 0.46).max(6.0);
    let inner_ratio = if is_pie { 0.0 } else { spec.inner_radius.unwrap_or(0.58).clamp(0.0, 0.85) };
    let inner_r = outer_r * inner_ratio;
    let cx = plot_w / 2.0;
    let cy = top + plot_h / 2.0;
    let show_values = spec.show_values.unwrap_or(false);

    let mut angle = -std::f64::consts::FRAC_PI_2;
    let two_pi = std::f64::consts::TAU;
    for (name, v, col) in &slices {
        if *v <= 0.0 {
            continue;
        }
        let frac = v / total;
        let sweep = frac * two_pi;
        if frac >= 0.9999 {
            // Full circle: two half-arcs to avoid degenerate zero-distance SVG arc.
            if inner_r > 0.1 {
                let d = format!(
                    "M{} {} A{r} {r} 0 1 1 {} {} A{r} {r} 0 1 1 {} {} M{} {} A{ir} {ir} 0 1 0 {} {} A{ir} {ir} 0 1 0 {} {} Z",
                    num(cx),
                    num(cy - outer_r),
                    num(cx),
                    num(cy + outer_r),
                    num(cx),
                    num(cy - outer_r),
                    num(cx),
                    num(cy - inner_r),
                    num(cx),
                    num(cy + inner_r),
                    num(cx),
                    num(cy - inner_r),
                    r = num(outer_r),
                    ir = num(inner_r),
                );
                svg.push_str(&format!(r#"<path d="{d}" fill="{col}" fill-rule="evenodd"/>"#));
            } else {
                svg.push_str(&format!(r#"<circle cx="{}" cy="{}" r="{}" fill="{col}"/>"#, num(cx), num(cy), num(outer_r)));
            }
        } else {
            let a1 = angle;
            let a2 = angle + sweep;
            let large = if sweep > std::f64::consts::PI { 1 } else { 0 };
            let ox1 = cx + outer_r * libm::cos(a1);
            let oy1 = cy + outer_r * libm::sin(a1);
            let ox2 = cx + outer_r * libm::cos(a2);
            let oy2 = cy + outer_r * libm::sin(a2);
            let d = if inner_r > 0.1 {
                let ix1 = cx + inner_r * libm::cos(a2);
                let iy1 = cy + inner_r * libm::sin(a2);
                let ix2 = cx + inner_r * libm::cos(a1);
                let iy2 = cy + inner_r * libm::sin(a1);
                format!(
                    "M{} {} A{r} {r} 0 {large} 1 {} {} L{} {} A{ir} {ir} 0 {large} 0 {} {} Z",
                    num(ox1),
                    num(oy1),
                    num(ox2),
                    num(oy2),
                    num(ix1),
                    num(iy1),
                    num(ix2),
                    num(iy2),
                    r = num(outer_r),
                    ir = num(inner_r),
                )
            } else {
                format!(
                    "M{} {} L{} {} A{r} {r} 0 {large} 1 {} {} Z",
                    num(cx),
                    num(cy),
                    num(ox1),
                    num(oy1),
                    num(ox2),
                    num(oy2),
                    r = num(outer_r),
                )
            };
            svg.push_str(&format!(r#"<path d="{d}" fill="{col}" stroke="white" stroke-width="0.8"/>"#));
        }
        if show_values && frac >= 0.06 {
            let mid = angle + sweep / 2.0;
            let lr = if inner_r > 0.1 { (outer_r + inner_r) / 2.0 } else { outer_r * 0.65 };
            let lx = cx + lr * libm::cos(mid);
            let ly = cy + lr * libm::sin(mid);
            labels.push(TextLabel {
                x: lx - 16.0,
                y: ly - 5.5,
                w: 32.0,
                h: 11.0,
                align: "center",
                text: format!("{:.0}%", frac * 100.0),
                size_pt: 7.0,
                bold: true,
                muted: false,
            });
        }
        let _ = name;
        angle += sweep;
    }

    if show_legend {
        if legend_w > 0.0 {
            let lx = plot_w + 8.0;
            let row_h = 14.0;
            let start_y = (cy - (slices.len() as f64 * row_h) / 2.0).max(top + 2.0);
            for (i, (name, v, col)) in slices.iter().enumerate() {
                let y = start_y + i as f64 * row_h;
                let pct = (*v / total * 100.0).round() as i64;
                svg.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="6.5" height="6.5" rx="1.5" fill="{col}"/>"#,
                    num(lx),
                    num(y + 2.5),
                ));
                let val_text = format_value(*v, spec.value_prefix.as_deref(), spec.value_suffix.as_deref());
                labels.push(TextLabel {
                    x: lx + 10.0,
                    y,
                    w: (legend_w - 18.0).max(20.0),
                    h: 12.0,
                    align: "left",
                    text: format!("{name} ({val_text}, {pct}%)"),
                    size_pt: 7.2,
                    bold: false,
                    muted: false,
                });
            }
        } else {
            let cols = 2usize;
            let cw = w / cols as f64;
            let y0 = top + plot_h + 4.0;
            for (i, (name, v, col)) in slices.iter().enumerate() {
                let col_idx = i % cols;
                let row_idx = i / cols;
                let lx = col_idx as f64 * cw;
                let y = y0 + row_idx as f64 * 13.0;
                let pct = (*v / total * 100.0).round() as i64;
                svg.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="6" height="6" rx="1.5" fill="{col}"/>"#,
                    num(lx),
                    num(y + 2.5),
                ));
                labels.push(TextLabel {
                    x: lx + 9.0,
                    y,
                    w: (cw - 12.0).max(20.0),
                    h: 11.0,
                    align: "left",
                    text: format!("{name} ({pct}%)"),
                    size_pt: 7.0,
                    bold: false,
                    muted: false,
                });
            }
        }
    }
}

fn render_sparkline(
    spec: &ChartSpec,
    w: f64,
    h: f64,
    colors: &[String],
    svg: &mut String,
    labels: &mut Vec<TextLabel>,
) {
    let Some(series) = spec.series.first() else { return };
    if series.values.is_empty() {
        return;
    }
    let col = colors.first().map(String::as_str).unwrap_or(DEFAULT_PALETTE[0]);
    let show_values = spec.show_values.unwrap_or(false);
    let right_pad = if show_values { 28.0 } else { 3.0 };
    let pad_y = 2.5;
    let pw = (w - right_pad - 3.0).max(8.0);
    let ph = (h - 2.0 * pad_y).max(4.0);
    let min_v = series.values.iter().copied().fold(f64::INFINITY, f64::min);
    let max_v = series.values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max_v - min_v).max(1e-9);
    let n = series.values.len();
    let pts: Vec<(f64, f64)> = series
        .values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = if n == 1 { 2.0 + pw / 2.0 } else { 2.0 + i as f64 * (pw / (n - 1) as f64) };
            let y = pad_y + ph * (1.0 - (v - min_v) / span);
            (x, y)
        })
        .collect();
    if pts.len() >= 2 {
        let mut ad = format!("M{} {}", num(pts[0].0), num(pts[0].1));
        for &(px, py) in &pts[1..] {
            ad.push_str(&format!(" L{} {}", num(px), num(py)));
        }
        let base_y = pad_y + ph;
        ad.push_str(&format!(" L{} {} L{} {} Z", num(pts.last().unwrap().0), num(base_y), num(pts[0].0), num(base_y)));
        svg.push_str(&format!(r#"<path d="{ad}" fill="{col}" fill-opacity="0.14"/>"#));

        let mut ld = format!("M{} {}", num(pts[0].0), num(pts[0].1));
        for &(px, py) in &pts[1..] {
            ld.push_str(&format!(" L{} {}", num(px), num(py)));
        }
        svg.push_str(&format!(
            r#"<path d="{ld}" fill="none" stroke="{col}" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"#
        ));
    }
    if let Some(&(lx, ly)) = pts.last() {
        svg.push_str(&format!(r#"<circle cx="{}" cy="{}" r="2.0" fill="{col}"/>"#, num(lx), num(ly)));
        if show_values && let Some(&last_v) = series.values.last() {
            labels.push(TextLabel {
                x: lx + 4.0,
                y: (ly - 5.5).clamp(0.0, (h - 11.0).max(0.0)),
                w: right_pad - 4.0,
                h: 11.0,
                align: "left",
                text: format_value(last_v, spec.value_prefix.as_deref(), spec.value_suffix.as_deref()),
                size_pt: 7.0,
                bold: true,
                muted: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matenadar_model::widgets::ChartSeries;

    #[test]
    fn renders_all_chart_kinds_without_panic() {
        let color = |c: &matenadar_model::Color| c.0.clone();
        for kind in ["bar", "line", "area", "donut", "pie", "sparkline"] {
            let spec = ChartSpec {
                kind: kind.into(),
                labels: vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
                series: vec![
                    ChartSeries { name: Some("2025".into()), color: None, values: vec![12.0, 18.5, 24.0, 31.2] },
                    ChartSeries { name: Some("2026".into()), color: None, values: vec![15.0, 22.0, 29.0, 38.0] },
                ],
                title: Some("Revenue".into()),
                show_values: Some(true),
                value_prefix: Some("$".into()),
                value_suffix: Some("k".into()),
                ..Default::default()
            };
            let out = render(&spec, 280.0, 160.0, &color);
            assert!(out.svg.starts_with("<svg ") && out.svg.ends_with("</svg>"), "{kind}");
            assert!(!out.labels.is_empty(), "{kind} labels");
        }
    }
}
