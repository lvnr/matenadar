//! Writing Typst values: strings, lengths, colours, strokes, fills.

use matenadar_model::Length;
use matenadar_model::style::{Border, Fill, Gradient, Insets, Radius, Stroke};
use matenadar_model::units::{Color, num};

/// A Typst string literal.
pub fn str_lit(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A length, scaled by the Typst variable `k` when `scaled`.
pub fn len(l: &Length, scaled: bool) -> String {
    match l {
        Length::Pt(v) if scaled && *v != 0.0 => format!("({}pt * k)", num(*v)),
        other => other.typst(),
    }
}

pub fn pt(v: f64) -> String {
    format!("{}pt", num(v))
}

/// Colours resolve against the design's names.
pub struct Palette<'a> {
    pub names: &'a indexmap::IndexMap<String, Color>,
}

impl Palette<'_> {
    fn lookup<'b>(&'b self, c: &'b Color) -> &'b str {
        let s = c.0.trim();
        match self.names.get(s) {
            Some(v) => v.0.trim(),
            None => s,
        }
    }

    pub fn color(&self, c: &Color) -> String {
        let s = self.lookup(c);
        if s == "none" {
            "none".into()
        } else if s.starts_with('#') {
            format!("rgb({})", str_lit(s))
        } else {
            s.to_string()
        }
    }

    /// The colour for SVG, which knows only RGB.
    pub fn svg(&self, c: &Color) -> String {
        let s = self.lookup(c);
        if s == "none" || s.starts_with('#') {
            return s.to_string();
        }
        let args: Vec<f64> = s
            .split_once('(')
            .map(|(_, rest)| rest.trim_end_matches(')'))
            .unwrap_or("")
            .split(',')
            .map(|a| {
                let a = a.trim();
                match a.strip_suffix('%') {
                    Some(p) => p.parse::<f64>().unwrap_or(0.0) / 100.0,
                    None => a.parse::<f64>().unwrap_or(0.0) / 255.0,
                }
            })
            .collect();
        let hex = |r: f64, g: f64, b: f64| {
            format!("#{:02x}{:02x}{:02x}", (r.clamp(0.0, 1.0) * 255.0).round() as u8, (g.clamp(0.0, 1.0) * 255.0).round() as u8, (b.clamp(0.0, 1.0) * 255.0).round() as u8)
        };
        if s.starts_with("cmyk(") && args.len() == 4 {
            let k = 1.0 - args[3];
            hex((1.0 - args[0]) * k, (1.0 - args[1]) * k, (1.0 - args[2]) * k)
        } else if s.starts_with("luma(") && !args.is_empty() {
            hex(args[0], args[0], args[0])
        } else if s.starts_with("rgb(") && args.len() >= 3 {
            hex(args[0], args[1], args[2])
        } else {
            "#000000".into()
        }
    }

    pub fn stroke(&self, s: &Stroke, scaled: bool) -> String {
        match s {
            Stroke::Color(c) => {
                let c = self.color(c);
                if c == "none" { "none".into() } else { format!("(paint: {c}, thickness: 1pt)") }
            }
            Stroke::Full(spec) => {
                let mut parts = Vec::new();
                if let Some(c) = &spec.color {
                    let c = self.color(c);
                    if c == "none" {
                        return "none".into();
                    }
                    parts.push(format!("paint: {c}"));
                }
                parts.push(format!("thickness: {}", spec.width.as_ref().map_or("1pt".to_string(), |w| len(w, scaled))));
                if let Some(d) = &spec.dash {
                    match d {
                        serde_json::Value::String(w) => parts.push(format!("dash: {}", str_lit(w))),
                        serde_json::Value::Array(items) => {
                            let lens: Vec<String> = items
                                .iter()
                                .filter_map(|i| serde_json::from_value::<Length>(i.clone()).ok())
                                .map(|l| l.typst())
                                .collect();
                            if !lens.is_empty() {
                                match &spec.dash_offset {
                                    Some(o) => parts.push(format!("dash: (array: ({},), phase: {})", lens.join(", "), o.typst())),
                                    None => parts.push(format!("dash: ({},)", lens.join(", "))),
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(cap) = &spec.cap {
                    parts.push(format!("cap: {}", str_lit(cap)));
                }
                if let Some(join) = &spec.join {
                    parts.push(format!("join: {}", str_lit(join)));
                }
                format!("({})", parts.join(", "))
            }
        }
    }

    pub fn border(&self, b: &Border, scaled: bool) -> String {
        match b {
            Border::All(s) => self.stroke(s, scaled),
            Border::Sides(sides) => {
                let mut parts = Vec::new();
                for (name, s) in [("top", &sides.top), ("right", &sides.right), ("bottom", &sides.bottom), ("left", &sides.left)] {
                    if let Some(s) = s {
                        parts.push(format!("{name}: {}", self.stroke(s, scaled)));
                    }
                }
                if parts.is_empty() { "none".into() } else { format!("({})", parts.join(", ")) }
            }
        }
    }

    pub fn fill(&self, f: &Fill) -> String {
        match f {
            Fill::Color(c) => self.color(c),
            Fill::Gradient(g) => {
                let stops = |stops: &[(Color, f64)]| {
                    stops.iter().map(|(c, at)| format!("({}, {}%)", self.color(c), num(at * 100.0))).collect::<Vec<_>>().join(", ")
                };
                match g {
                    Gradient::Linear { angle, stops: s } => format!("gradient.linear({}, angle: {}deg)", stops(s), num(*angle)),
                    Gradient::Radial { stops: s, center, radius } => {
                        let (cx, cy) = center.unwrap_or((0.5, 0.5));
                        format!(
                            "gradient.radial({}, center: ({}%, {}%), radius: {}%)",
                            stops(s),
                            num(cx * 100.0),
                            num(cy * 100.0),
                            num(radius.unwrap_or(0.5) * 100.0)
                        )
                    }
                }
            }
        }
    }
}

pub fn insets(i: &Insets, scaled: bool) -> String {
    match i {
        Insets::All(l) => len(l, scaled),
        Insets::Sides(s) => {
            let mut parts = Vec::new();
            for (name, v) in [("x", &s.x), ("y", &s.y), ("top", &s.top), ("right", &s.right), ("bottom", &s.bottom), ("left", &s.left)] {
                if let Some(v) = v {
                    parts.push(format!("{name}: {}", len(v, scaled)));
                }
            }
            if parts.is_empty() { "0pt".into() } else { format!("({})", parts.join(", ")) }
        }
    }
}

/// Insets as (top, right, bottom, left) points, for computing inner sizes.
pub fn insets_pt(i: Option<&Insets>, size: f64) -> (f64, f64, f64, f64) {
    match i {
        None => (0.0, 0.0, 0.0, 0.0),
        Some(Insets::All(l)) => {
            let v = l.resolve(size, 0.0);
            (v, v, v, v)
        }
        Some(Insets::Sides(s)) => {
            let g = |side: &Option<Length>, axis: &Option<Length>| side.or(*axis).map_or(0.0, |l| l.resolve(size, 0.0));
            (g(&s.top, &s.y), g(&s.right, &s.x), g(&s.bottom, &s.y), g(&s.left, &s.x))
        }
    }
}

pub fn radius(r: &Radius, scaled: bool) -> String {
    match r {
        Radius::All(l) => len(l, scaled),
        Radius::Corners(c) => {
            let mut parts = Vec::new();
            for (name, v) in [("top-left", &c.top_left), ("top-right", &c.top_right), ("bottom-right", &c.bottom_right), ("bottom-left", &c.bottom_left)] {
                if let Some(v) = v {
                    parts.push(format!("{name}: {}", len(v, scaled)));
                }
            }
            if parts.is_empty() { "0pt".into() } else { format!("({})", parts.join(", ")) }
        }
    }
}

/// Fractions of a box for an anchor word: (0, 0) is the top-left corner.
pub fn anchor(word: Option<&str>) -> (f64, f64) {
    match word.unwrap_or("top-left") {
        "top" => (0.5, 0.0),
        "top-right" => (1.0, 0.0),
        "left" => (0.0, 0.5),
        "center" => (0.5, 0.5),
        "right" => (1.0, 0.5),
        "bottom-left" => (0.0, 1.0),
        "bottom" => (0.5, 1.0),
        "bottom-right" => (1.0, 1.0),
        "baseline" => (0.5, 0.0),
        "baseline-right" => (1.0, 0.0),
        _ => (0.0, 0.0),
    }
}

/// A `when` condition as a Typst expression.
pub fn when(cond: &str) -> String {
    let parts: Vec<String> = cond
        .split_whitespace()
        .map(|w| {
            let (neg, w) = match w.strip_prefix("not-") {
                Some(rest) => (true, rest),
                None => (false, w),
            };
            let e = match w {
                "recto" => "mt-is-recto()",
                "verso" => "not mt-is-recto()",
                "first" => "mt-is-first()",
                "blank" => "mt-is-blank()",
                _ => "true",
            };
            if neg { format!("not ({e})") } else { format!("({e})") }
        })
        .collect();
    if parts.is_empty() { "true".into() } else { parts.join(" and ") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matenadar_model::style::StrokeSpec;

    #[test]
    fn writes_strings_and_strokes() {
        assert_eq!(str_lit("a \"b\" \\ c\n"), r#""a \"b\" \\ c\n""#);
        let names = indexmap::IndexMap::from([("ink".to_string(), Color("#112233".into()))]);
        let p = Palette { names: &names };
        assert_eq!(p.color(&Color("ink".into())), "rgb(\"#112233\")");
        let s = Stroke::Full(StrokeSpec {
            width: Some(Length::Pt(0.5)),
            color: Some(Color("ink".into())),
            dash: Some(serde_json::json!("dashed")),
            cap: Some("round".into()),
            join: None,
            dash_offset: None,
        });
        assert_eq!(p.stroke(&s, false), "(paint: rgb(\"#112233\"), thickness: 0.5pt, dash: \"dashed\", cap: \"round\")");
        assert_eq!(p.svg(&Color("cmyk(0%, 0%, 0%, 100%)".into())), "#000000");
        assert_eq!(when("recto not-first"), "(mt-is-recto()) and not (mt-is-first())");
    }
}
