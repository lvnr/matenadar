//! Text drawn as outlines (lettering), font metrics for line boxes, and QR
//! codes: things the compiler makes itself as SVG.

use std::fmt::Write;

use typst::text::Font;

use crate::world::Fonts;

/// Vertical metrics of a font in em: ascender above the baseline,
/// descender below it (positive), as browsers read them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VMetrics {
    pub ascender: f64,
    pub descender: f64,
}

impl VMetrics {
    pub const DEFAULT: VMetrics = VMetrics { ascender: 0.9, descender: 0.25 };

    pub fn of(font: &Font) -> Self {
        match ttf_parser::Face::parse(font.data().as_slice(), font.index()) {
            Ok(face) => {
                let upem = face.units_per_em() as f64;
                VMetrics { ascender: face.ascender() as f64 / upem, descender: -(face.descender() as f64) / upem }
            }
            Err(_) => VMetrics::DEFAULT,
        }
    }
}

/// A CSS weight word or number as a number.
pub fn weight_number(v: &serde_json::Value) -> u16 {
    match v {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(400.0).clamp(1.0, 1000.0) as u16,
        serde_json::Value::String(s) => match s.as_str() {
            "thin" => 100,
            "extralight" => 200,
            "light" => 300,
            "medium" => 500,
            "semibold" => 600,
            "bold" => 700,
            "extrabold" => 800,
            "black" => 900,
            _ => 400,
        },
        _ => 400,
    }
}

/// The first family of `families` the fonts have, in the given variant.
pub fn pick(fonts: &Fonts, families: &[String], weight: u16, italic: bool) -> Option<Font> {
    families.iter().find_map(|f| fonts.select(f, weight, italic))
}

/// Lettering: the text shaped with the font's features and drawn as one
/// SVG path. The box is CSS's: from the ascender to the descender.
pub struct Lettered {
    pub svg: String,
    pub width_pt: f64,
    pub height_pt: f64,
    /// Where the drawing's top-left corner sits from the element's (text on
    /// a path reaches above and left of the path's own coordinates).
    pub offset: (f64, f64),
}

pub struct LetterSpec<'a> {
    pub text: &'a str,
    pub font: &'a Font,
    pub size_pt: f64,
    pub tracking_pt: f64,
    pub features: &'a [String],
    pub fill: &'a str,
    pub stroke: Option<(&'a str, f64)>,
    pub line_height: f64,
    /// 0 left, 0.5 centre, 1 right, within `width_pt` when given.
    pub align: f64,
    pub width_pt: Option<f64>,
    /// Variable-font axes (`wght`, `wdth`, `opsz`…); axes the font lacks are skipped.
    pub variations: Vec<([u8; 4], f32)>,
    /// An SVG path (in points) to set the text along.
    pub path: Option<&'a str>,
    /// How far along the path the text's start (or middle, or end) falls.
    pub start_pt: Option<f64>,
}

pub fn letter(spec: &LetterSpec) -> Result<Lettered, String> {
    let data = spec.font.data();
    let mut face = rustybuzz::Face::from_slice(data.as_slice(), spec.font.index()).ok_or("the font can't be shaped")?;
    let axes: Vec<ttf_parser::Tag> = face.variation_axes().into_iter().map(|a| a.tag).collect();
    let variations: Vec<rustybuzz::Variation> = spec
        .variations
        .iter()
        .map(|(tag, value)| rustybuzz::Variation { tag: ttf_parser::Tag::from_bytes(tag), value: *value })
        .filter(|v| axes.contains(&v.tag))
        .collect();
    if !variations.is_empty() {
        face.set_variations(&variations);
    }
    let upem = face.units_per_em() as f64;
    let scale = spec.size_pt / upem;
    let m = VMetrics::of(spec.font);
    let line_box = (m.ascender + m.descender) * spec.size_pt;
    let advance_line = spec.line_height.max(m.ascender + m.descender) * spec.size_pt;
    let features: Vec<rustybuzz::Feature> = spec
        .features
        .iter()
        .filter_map(|f| {
            let f = if let Some(off) = f.strip_prefix('-') { format!("{off}=0") } else { f.clone() };
            f.parse().ok()
        })
        .collect();

    // Each line's glyphs: (id, pen x, x offset, y offset, advance), in points.
    let shape = |line: &str| {
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(line);
        buf.guess_segment_properties();
        let shaped = rustybuzz::shape(&face, &features, buf);
        let mut x = 0.0;
        let mut glyphs = Vec::new();
        let count = shaped.glyph_infos().len();
        for (i, (info, pos)) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()).enumerate() {
            let adv = pos.x_advance as f64 * scale;
            glyphs.push((info.glyph_id as u16, x, pos.x_offset as f64 * scale, pos.y_offset as f64 * scale, adv));
            x += adv;
            if i + 1 < count {
                x += spec.tracking_pt;
            }
        }
        (glyphs, x)
    };

    let mut d = String::new();
    let mut bounds = Bounds::default();
    if let Some(path) = spec.path {
        let along = Polyline::from_svg(path)?;
        let (glyphs, line_w) = shape(&spec.text.replace('\n', " "));
        let at = spec.start_pt.unwrap_or(along.length * spec.align);
        let first = at - line_w * spec.align;
        for &(gid, x, gx, gy, adv) in &glyphs {
            // Each letter stands on the path at its middle, turned to the path.
            let ((px, py), angle) = along.at(first + x + adv / 2.0);
            let (sin, cos) = angle.sin_cos();
            let mut pen = Pen { d: &mut d, ox: px, oy: py, s: scale, u0: gx - adv / 2.0, v0: -gy, cos, sin, bounds: &mut bounds };
            face.outline_glyph(ttf_parser::GlyphId(gid), &mut pen);
        }
    } else {
        let mut lines = Vec::new();
        let mut widest: f64 = 0.0;
        for line in spec.text.split('\n') {
            let (glyphs, w) = shape(line);
            widest = widest.max(w);
            lines.push((glyphs, w));
        }
        let width = spec.width_pt.unwrap_or(widest);
        for (row, (glyphs, line_w)) in lines.iter().enumerate() {
            let baseline = m.ascender * spec.size_pt + advance_line * row as f64;
            let dx = (width - line_w) * spec.align;
            for &(gid, x, gx, gy, _) in glyphs {
                let mut pen = Pen { d: &mut d, ox: dx + x + gx, oy: baseline - gy, s: scale, u0: 0.0, v0: 0.0, cos: 1.0, sin: 0.0, bounds: &mut bounds };
                face.outline_glyph(ttf_parser::GlyphId(gid), &mut pen);
            }
        }
        let height = if lines.len() <= 1 { line_box } else { line_box + advance_line * (lines.len() - 1) as f64 };
        bounds = Bounds { x0: 0.0, y0: 0.0, x1: width, y1: height, any: true };
    }
    if !bounds.any {
        bounds = Bounds { x0: 0.0, y0: 0.0, x1: 0.0, y1: 0.0, any: true };
    }
    let stroke = match spec.stroke {
        Some((color, w)) => {
            // Room for the stroke outside the outlines.
            bounds.x0 -= w;
            bounds.y0 -= w;
            bounds.x1 += w;
            bounds.y1 += w;
            format!(r#" stroke="{color}" stroke-width="{w}""#)
        }
        None => String::new(),
    };
    let (w, h) = (bounds.x1 - bounds.x0, bounds.y1 - bounds.y0);
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}pt" height="{h}pt" viewBox="{x} {y} {w} {h}"><path fill="{fill}"{stroke} d="{d}"/></svg>"#,
        x = round(bounds.x0),
        y = round(bounds.y0),
        w = round(w),
        h = round(h),
        fill = spec.fill,
    );
    Ok(Lettered { svg, width_pt: w, height_pt: h, offset: (bounds.x0, bounds.y0) })
}

#[derive(Default)]
struct Bounds {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    any: bool,
}

impl Bounds {
    fn add(&mut self, x: f64, y: f64) {
        if self.any {
            self.x0 = self.x0.min(x);
            self.y0 = self.y0.min(y);
            self.x1 = self.x1.max(x);
            self.y1 = self.y1.max(y);
        } else {
            *self = Bounds { x0: x, y0: y, x1: x, y1: y, any: true };
        }
    }
}

/// The length of an SVG path's first subpath, in its own units.
pub fn path_length(d: &str) -> Option<f64> {
    Polyline::from_svg(d).ok().map(|p| p.length)
}

/// A path flattened to points, for finding where a distance along it lands.
struct Polyline {
    points: Vec<(f64, f64)>,
    /// Distance from the start to each point.
    dist: Vec<f64>,
    length: f64,
}

impl Polyline {
    fn from_svg(d: &str) -> Result<Self, String> {
        let path = kurbo::BezPath::from_svg(d).map_err(|e| format!("the path can't be read: {e}"))?;
        let mut points: Vec<(f64, f64)> = Vec::new();
        let mut first = None;
        kurbo::flatten(&path, 0.02, |el| match el {
            // Text follows the first subpath only.
            kurbo::PathEl::MoveTo(p) if points.is_empty() => {
                points.push((p.x, p.y));
                first = Some((p.x, p.y));
            }
            kurbo::PathEl::LineTo(p) if first.is_some() => points.push((p.x, p.y)),
            kurbo::PathEl::ClosePath => {
                if let Some(f) = first.take() {
                    points.push(f);
                }
            }
            _ => {}
        });
        if points.len() < 2 {
            return Err("the path has no length".into());
        }
        let mut dist = vec![0.0];
        for w in points.windows(2) {
            let last = *dist.last().unwrap();
            dist.push(last + ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt());
        }
        let length = *dist.last().unwrap();
        Ok(Polyline { points, dist, length })
    }

    /// The point `s` along the path and the path's direction there (radians).
    /// Beyond the ends, the path continues straight.
    fn at(&self, s: f64) -> ((f64, f64), f64) {
        let i = self.dist.partition_point(|&d| d <= s).clamp(1, self.points.len() - 1);
        let (a, b) = (self.points[i - 1], self.points[i]);
        let seg = self.dist[i] - self.dist[i - 1];
        let t = if seg > 0.0 { (s - self.dist[i - 1]) / seg } else { 0.0 };
        ((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t), (b.1 - a.1).atan2(b.0 - a.0))
    }
}

/// Writes glyph outlines as SVG path data: font units scaled by `s`, offset
/// by (`u0`, `v0`) from the glyph's origin, turned by the angle whose cosine
/// and sine are given, and placed at (`ox`, `oy`).
struct Pen<'a> {
    d: &'a mut String,
    ox: f64,
    oy: f64,
    s: f64,
    u0: f64,
    v0: f64,
    cos: f64,
    sin: f64,
    bounds: &'a mut Bounds,
}

impl Pen<'_> {
    fn p(&mut self, x: f32, y: f32) -> (String, String) {
        let u = self.u0 + x as f64 * self.s;
        let v = self.v0 - y as f64 * self.s;
        let (px, py) = (self.ox + u * self.cos - v * self.sin, self.oy + u * self.sin + v * self.cos);
        self.bounds.add(px, py);
        (round(px), round(py))
    }
}

impl ttf_parser::OutlineBuilder for Pen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (a, b) = self.p(x, y);
        let _ = write!(self.d, "M{a} {b}");
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (a, b) = self.p(x, y);
        let _ = write!(self.d, "L{a} {b}");
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, b) = self.p(x1, y1);
        let (c, e) = self.p(x, y);
        let _ = write!(self.d, "Q{a} {b} {c} {e}");
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b) = self.p(x1, y1);
        let (c, e) = self.p(x2, y2);
        let (f, g) = self.p(x, y);
        let _ = write!(self.d, "C{a} {b} {c} {e} {f} {g}");
    }
    fn close(&mut self) {
        self.d.push('Z');
    }
}

/// An SVG with everything inside its root wrapped in a group carrying `attrs`
/// (`opacity="0.5"`, `fill="#333"`).
pub fn svg_group(svg: &str, attrs: &str) -> String {
    let Some(open) = svg.find("<svg") else { return svg.to_string() };
    let Some(tag_end) = svg[open..].find('>').map(|i| open + i + 1) else { return svg.to_string() };
    if svg[..tag_end].ends_with("/>") {
        return svg.to_string();
    }
    let Some(close) = svg.rfind("</svg>") else { return svg.to_string() };
    format!("{}<g {attrs}>{}</g>{}", &svg[..tag_end], &svg[tag_end..close], &svg[close..])
}

/// An SVG with every colour mixed towards `toward` (`#rrggbb`) by `amount`:
/// hex colours, the names black and white, and the default black fill.
pub fn tint_svg(svg: &str, toward: &str, amount: f64) -> String {
    let target = parse_hex(toward.trim_start_matches('#')).unwrap_or((255, 255, 255));
    let a = amount.clamp(0.0, 1.0);
    let mix = |(r, g, b): (u8, u8, u8)| {
        let m = |c: u8, t: u8| (c as f64 + (t as f64 - c as f64) * a).round().clamp(0.0, 255.0) as u8;
        format!("#{:02x}{:02x}{:02x}", m(r, target.0), m(g, target.1), m(b, target.2))
    };
    let mut svg = svg.to_string();
    for (name, rgb) in [("black", (0u8, 0u8, 0u8)), ("white", (255, 255, 255))] {
        for (before, after) in [("=\"", "\""), ("='", "'"), (":", ";"), (": ", ";")] {
            svg = svg.replace(&format!("{before}{name}{after}"), &format!("{before}{}{after}", mix(rgb)));
        }
    }
    let bytes = svg.as_bytes();
    let mut out = String::with_capacity(svg.len());
    let mut i = 0;
    while i < svg.len() {
        if bytes[i] == b'#' {
            let before = &svg[..i];
            let in_ref = before.ends_with("url(") || before.ends_with("href=\"") || before.ends_with("href='");
            let len = svg[i + 1..].bytes().take_while(|b| b.is_ascii_hexdigit()).count();
            let next = svg.as_bytes().get(i + 1 + len).copied().unwrap_or(b' ');
            if !in_ref && (len == 3 || len == 6) && !next.is_ascii_alphanumeric() && next != b'-' && next != b'_' {
                if let Some(rgb) = parse_hex(&svg[i + 1..i + 1 + len]) {
                    out.push_str(&mix(rgb));
                    i += 1 + len;
                    continue;
                }
            }
        }
        let ch = svg[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    // Shapes with no fill are black.
    svg_group(&out, &format!(r#"fill="{}""#, mix((0, 0, 0))))
}

fn parse_hex(h: &str) -> Option<(u8, u8, u8)> {
    let v = |s: &str| u8::from_str_radix(s, 16).ok();
    match h.len() {
        3 => {
            let c: Vec<String> = h.chars().map(|c| format!("{c}{c}")).collect();
            Some((v(&c[0])?, v(&c[1])?, v(&c[2])?))
        }
        6 => Some((v(&h[0..2])?, v(&h[2..4])?, v(&h[4..6])?)),
        _ => None,
    }
}

fn round(v: f64) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    if r == 0.0 { "0".into() } else { format!("{r}") }
}

/// The pixel size of a PNG, JPEG, GIF or WebP file, from its header.
pub fn image_size(path: &std::path::Path) -> Option<(u32, u32)> {
    let data = std::fs::read(path).ok()?;
    let be16 = |b: &[u8]| u16::from_be_bytes([b[0], b[1]]) as u32;
    let be32 = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    if data.starts_with(b"\x89PNG") && data.len() > 24 {
        return Some((be32(&data[16..20]), be32(&data[20..24])));
    }
    if data.starts_with(b"GIF") && data.len() > 10 {
        return Some((u16::from_le_bytes([data[6], data[7]]) as u32, u16::from_le_bytes([data[8], data[9]]) as u32));
    }
    if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") && data.len() > 30 {
        if &data[12..16] == b"VP8X" {
            let w = 1 + (data[24] as u32 | (data[25] as u32) << 8 | (data[26] as u32) << 16);
            let h = 1 + (data[27] as u32 | (data[28] as u32) << 8 | (data[29] as u32) << 16);
            return Some((w, h));
        }
        return None;
    }
    if data.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2;
        while i + 9 < data.len() {
            if data[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = data[i + 1];
            let len = be16(&data[i + 2..i + 4]) as usize;
            if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
                return Some((be16(&data[i + 7..i + 9]), be16(&data[i + 5..i + 7])));
            }
            i += 2 + len;
        }
    }
    None
}

/// A QR code as SVG, one unit per module, dark modules merged per row.
/// A QR code as runs of dark modules, `(x, y, length)` in modules from the
/// top-left corner (quiet zone included), and the side in modules.
pub fn qr_runs(data: &str, ecc: &str, quiet: u32) -> Result<(Vec<(u32, u32, u32)>, u32), String> {
    use qrcode::{Color, EcLevel, QrCode};
    let level = match ecc {
        "L" => EcLevel::L,
        "Q" => EcLevel::Q,
        "H" => EcLevel::H,
        _ => EcLevel::M,
    };
    let code = QrCode::with_error_correction_level(data.as_bytes(), level).map_err(|e| format!("QR code: {e}"))?;
    let n = code.width() as u32;
    let mut runs = Vec::new();
    for y in 0..n {
        let mut x = 0;
        while x < n {
            if code[(x as usize, y as usize)] == Color::Dark {
                let start = x;
                while x < n && code[(x as usize, y as usize)] == Color::Dark {
                    x += 1;
                }
                runs.push((start + quiet, y + quiet, x - start));
            } else {
                x += 1;
            }
        }
    }
    Ok((runs, n + 2 * quiet))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_codes_decode_to_a_square_of_modules() {
        let (runs, size) = qr_runs("https://askaldus.com", "M", 0).unwrap();
        assert!(!runs.is_empty());
        assert_eq!(size, 25, "a short URL at level M fits version 2 (25 modules)");
    }

    #[test]
    fn weights_from_words_and_numbers() {
        assert_eq!(weight_number(&serde_json::json!("semibold")), 600);
        assert_eq!(weight_number(&serde_json::json!(350)), 350);
    }
}
