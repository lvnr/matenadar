//! Lengths, angles and colours as documents write them.
//!
//! A length is a number of points, or a string with a unit: `"12pt"`,
//! `"0.5in"`, `"3mm"`, `"1.2cm"`, `"1.5em"` (relative to the text size) or
//! `"50%"` (relative to the containing box). A colour is `"#rrggbb"`,
//! `"#rrggbbaa"`, a name from the design's colours, `"cmyk(c%, m%, y%, k%)"`,
//! `"luma(n%)"` or `"none"`.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const PT_PER_IN: f64 = 72.0;
pub const PT_PER_MM: f64 = 72.0 / 25.4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Pt(f64),
    Em(f64),
    Pct(f64),
}

impl Length {
    pub const ZERO: Length = Length::Pt(0.0);

    /// Parses `"12pt"`, `"0.5in"`, `"3mm"`, `"1cm"`, `"1.5em"`, `"50%"` or a bare number (points).
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        let split = s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+')).unwrap_or(s.len());
        let (num, unit) = s.split_at(split);
        let n: f64 = num.parse().map_err(|_| format!("\"{s}\" is not a length (try \"12pt\", \"0.5in\", \"3mm\", \"1.5em\" or \"50%\")"))?;
        if !n.is_finite() {
            return Err(format!("\"{s}\" is not a finite length"));
        }
        match unit.trim() {
            "" | "pt" => Ok(Length::Pt(n)),
            "in" => Ok(Length::Pt(n * PT_PER_IN)),
            "mm" => Ok(Length::Pt(n * PT_PER_MM)),
            "cm" => Ok(Length::Pt(n * PT_PER_MM * 10.0)),
            "px" => Ok(Length::Pt(n * 0.75)),
            "em" => Ok(Length::Em(n)),
            "%" => Ok(Length::Pct(n)),
            other => Err(format!("\"{s}\": unknown unit \"{other}\" (use pt, in, mm, cm, px, em or %)")),
        }
    }

    /// Points, if this length is absolute.
    pub fn pt(&self) -> Option<f64> {
        match *self {
            Length::Pt(v) => Some(v),
            _ => None,
        }
    }

    /// Points, resolving em against `size_pt` and % against `of_pt`.
    pub fn resolve(&self, size_pt: f64, of_pt: f64) -> f64 {
        match *self {
            Length::Pt(v) => v,
            Length::Em(v) => v * size_pt,
            Length::Pct(v) => v / 100.0 * of_pt,
        }
    }

    /// As Typst writes it.
    pub fn typst(&self) -> String {
        match *self {
            Length::Pt(v) => format!("{}pt", num(v)),
            Length::Em(v) => format!("{}em", num(v)),
            Length::Pct(v) => format!("{}%", num(v)),
        }
    }
}

/// A number without a trailing `.0` and without float noise.
pub fn num(v: f64) -> String {
    let r = (v * 10_000.0).round() / 10_000.0;
    let r = if r == 0.0 { 0.0 } else { r };
    if r.fract() == 0.0 { format!("{}", r as i64) } else { format!("{r}") }
}

impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Length::Pt(v) => write!(f, "{}pt", num(v)),
            Length::Em(v) => write!(f, "{}em", num(v)),
            Length::Pct(v) => write!(f, "{}%", num(v)),
        }
    }
}

impl Serialize for Length {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match *self {
            Length::Pt(v) => s.serialize_f64(v),
            _ => s.serialize_str(&self.to_string()),
        }
    }
}

impl<'de> Deserialize<'de> for Length {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = Length;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a length: a number of points or a string such as \"0.5in\"")
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Length, E> {
                Ok(Length::Pt(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Length, E> {
                Ok(Length::Pt(v as f64))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Length, E> {
                Ok(Length::Pt(v as f64))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Length, E> {
                Length::parse(v).map_err(E::custom)
            }
        }
        d.deserialize_any(V)
    }
}

/// A colour as written; resolved against the design's colours when compiled.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Color(pub String);

impl Color {
    pub fn is_none(&self) -> bool {
        self.0.trim() == "none"
    }

    /// Checks the colour's syntax. A bare name must be one of `names`.
    pub fn check(&self, names: &dyn Fn(&str) -> bool) -> Result<(), String> {
        let s = self.0.trim();
        if s == "none" || names(s) {
            return Ok(());
        }
        if let Some(hex) = s.strip_prefix('#') {
            if matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Ok(());
            }
            return Err(format!("\"{s}\" is not a hex colour (#rgb, #rrggbb or #rrggbbaa)"));
        }
        for f in ["cmyk(", "luma(", "rgb(", "oklch("] {
            if let Some(rest) = s.strip_prefix(f) {
                if rest.ends_with(')') && rest[..rest.len() - 1].chars().all(|c| c.is_ascii_digit() || " .,%-deg".contains(c)) {
                    return Ok(());
                }
                return Err(format!("\"{s}\" is not a valid colour function"));
            }
        }
        Err(format!("unknown colour \"{s}\": not a hex colour, a colour function, or a name in design.colors"))
    }
}

/// An angle in degrees.
pub type Degrees = f64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lengths() {
        assert_eq!(Length::parse("12pt"), Ok(Length::Pt(12.0)));
        assert_eq!(Length::parse("0.5in"), Ok(Length::Pt(36.0)));
        assert_eq!(Length::parse("1.5em"), Ok(Length::Em(1.5)));
        assert_eq!(Length::parse("-2"), Ok(Length::Pt(-2.0)));
        assert_eq!(Length::parse("50%"), Ok(Length::Pct(50.0)));
        assert!((Length::parse("25.4mm").unwrap().pt().unwrap() - 72.0).abs() < 1e-9);
        assert!(Length::parse("3 furlongs").is_err());
        assert!(Length::parse("pt").is_err());
    }

    #[test]
    fn lengths_deserialize_from_numbers_and_strings() {
        let v: Vec<Length> = serde_json::from_str(r#"[12, 0.5, "1in", "2em"]"#).unwrap();
        assert_eq!(v, vec![Length::Pt(12.0), Length::Pt(0.5), Length::Pt(72.0), Length::Em(2.0)]);
        assert_eq!(Length::Pt(36.0).typst(), "36pt");
        assert_eq!(Length::Pt(1.0 / 3.0).typst(), "0.3333pt");
    }

    #[test]
    fn checks_colours() {
        let names = |n: &str| n == "ink";
        assert!(Color("#1a2b3c".into()).check(&names).is_ok());
        assert!(Color("ink".into()).check(&names).is_ok());
        assert!(Color("cmyk(0%, 10%, 20%, 90%)".into()).check(&names).is_ok());
        assert!(Color("#12".into()).check(&names).is_err());
        assert!(Color("paper".into()).check(&names).is_err());
    }
}
