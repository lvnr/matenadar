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
}

pub fn letter(spec: &LetterSpec) -> Result<Lettered, String> {
    let data = spec.font.data();
    let face = rustybuzz::Face::from_slice(data.as_slice(), spec.font.index()).ok_or("the font can't be shaped")?;
    let ttf = ttf_parser::Face::parse(data.as_slice(), spec.font.index()).map_err(|_| "the font can't be read")?;
    let upem = ttf.units_per_em() as f64;
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

    let mut lines = Vec::new();
    let mut widest: f64 = 0.0;
    for line in spec.text.split('\n') {
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(line);
        buf.guess_segment_properties();
        let shaped = rustybuzz::shape(&face, &features, buf);
        let mut x = 0.0;
        let mut glyphs = Vec::new();
        let count = shaped.glyph_infos().len();
        for (i, (info, pos)) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()).enumerate() {
            glyphs.push((info.glyph_id as u16, x + pos.x_offset as f64 * scale, pos.y_offset as f64 * scale));
            x += pos.x_advance as f64 * scale;
            if i + 1 < count {
                x += spec.tracking_pt;
            }
        }
        widest = widest.max(x);
        lines.push((glyphs, x));
    }
    let width = spec.width_pt.unwrap_or(widest);
    let height = if lines.len() <= 1 { line_box } else { line_box + advance_line * (lines.len() - 1) as f64 };

    let mut d = String::new();
    for (row, (glyphs, line_w)) in lines.iter().enumerate() {
        let baseline = m.ascender * spec.size_pt + advance_line * row as f64;
        let dx = (width - line_w) * spec.align;
        for &(gid, gx, gy) in glyphs {
            let mut pen = Pen { d: &mut d, x: dx + gx, y: baseline - gy, s: scale };
            ttf.outline_glyph(ttf_parser::GlyphId(gid), &mut pen);
        }
    }
    let stroke = match spec.stroke {
        Some((color, w)) => format!(r#" stroke="{color}" stroke-width="{w}""#),
        None => String::new(),
    };
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}pt" height="{h}pt" viewBox="0 0 {w} {h}"><path fill="{fill}"{stroke} d="{d}"/></svg>"#,
        w = round(width),
        h = round(height),
        fill = spec.fill,
    );
    Ok(Lettered { svg, width_pt: width, height_pt: height })
}

struct Pen<'a> {
    d: &'a mut String,
    x: f64,
    y: f64,
    s: f64,
}

impl Pen<'_> {
    fn p(&self, x: f32, y: f32) -> (String, String) {
        (round(self.x + x as f64 * self.s), round(self.y - y as f64 * self.s))
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

fn round(v: f64) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    if r == 0.0 { "0".into() } else { format!("{r}") }
}

/// A QR code as SVG, one unit per module, dark modules merged per row.
pub fn qr_svg(data: &str, ecc: &str, quiet: u32, dark: &str, light: Option<&str>) -> Result<(String, u32), String> {
    use qrcode::{Color, EcLevel, QrCode};
    let level = match ecc {
        "L" => EcLevel::L,
        "Q" => EcLevel::Q,
        "H" => EcLevel::H,
        _ => EcLevel::M,
    };
    let code = QrCode::with_error_correction_level(data.as_bytes(), level).map_err(|e| format!("QR code: {e}"))?;
    let n = code.width() as u32;
    let size = n + 2 * quiet;
    let mut d = String::new();
    for y in 0..n {
        let mut x = 0;
        while x < n {
            if code[(x as usize, y as usize)] == Color::Dark {
                let start = x;
                while x < n && code[(x as usize, y as usize)] == Color::Dark {
                    x += 1;
                }
                let _ = write!(d, "M{} {}h{}v1h-{}z", start + quiet, y + quiet, x - start, x - start);
            } else {
                x += 1;
            }
        }
    }
    let bg = light.map(|c| format!(r#"<rect width="{size}" height="{size}" fill="{c}"/>"#)).unwrap_or_default();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" shape-rendering="crispEdges">{bg}<path fill="{dark}" d="{d}"/></svg>"#
    );
    Ok((svg, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_codes_decode_to_a_square_of_modules() {
        let (svg, size) = qr_svg("https://askaldus.com", "M", 0, "#000", None).unwrap();
        assert!(svg.starts_with("<svg"));
        assert_eq!(size, 25, "a short URL at level M fits version 2 (25 modules)");
    }

    #[test]
    fn weights_from_words_and_numbers() {
        assert_eq!(weight_number(&serde_json::json!("semibold")), 600);
        assert_eq!(weight_number(&serde_json::json!(350)), 350);
    }
}
