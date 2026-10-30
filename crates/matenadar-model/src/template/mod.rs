//! Templates: a book kind's pages from a short description. Each takes a
//! spec (JSON) and gives a whole document, which is then edited like any
//! other. Items keep the ids the spec gives them, so edits kept as
//! operations land where they were made when the document is composed again.
//!
//! - `picture-book`: lettered full-bleed story pages and spreads, typeset
//!   front and back matter, 24, 32 or 40 pages, self or separate ends.
//! - `coloring`: one picture per recto with its back left blank.

mod picture_book;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::Document;
use crate::units::{Length, PT_PER_IN};

pub use picture_book::PictureBook;

/// The templates there are.
pub const KINDS: [&str; 1] = ["picture-book"];

/// The document `kind`'s template makes from `spec`.
pub fn compose(kind: &str, spec: &Value) -> Result<Document, String> {
    let value = match kind {
        "picture-book" => picture_book::compose(&parse(spec)?)?,
        other => return Err(format!("no template \"{other}\" (there are {})", KINDS.join(", "))),
    };
    let doc = Document::from_json(&value.to_string())?;
    let problems = crate::validate(&doc);
    if let Some(p) = problems.first() {
        return Err(format!("the template made an invalid document: {}: {}", p.path, p.message));
    }
    Ok(doc)
}

fn parse<T: for<'de> Deserialize<'de>>(spec: &Value) -> Result<T, String> {
    serde_path_to_error::deserialize(spec).map_err(|e| {
        let path = e.path().to_string();
        format!("{path}: {}", e.into_inner())
    })
}

/// A trim size in points, from `"8.5x11"` (width × height in inches).
pub(crate) fn trim(s: &str, allowed: &[&str]) -> Result<(f64, f64), String> {
    if !allowed.contains(&s) {
        return Err(format!("trim \"{s}\" is not one of {}", allowed.join(", ")));
    }
    let (w, h) = s.split_once('x').ok_or_else(|| format!("trim \"{s}\" is width x height in inches"))?;
    let inch = |v: &str| v.trim().parse::<f64>().map(|v| v * PT_PER_IN).map_err(|_| format!("trim \"{s}\" is width x height in inches"));
    Ok((inch(w)?, inch(h)?))
}

/// Points as a length string.
pub(crate) fn pt(v: f64) -> String {
    format!("{}pt", crate::units::num(v))
}

/// A length in points.
pub(crate) fn points(l: Option<Length>, default: f64) -> f64 {
    l.and_then(|l| l.pt()).unwrap_or(default)
}

/// The type families a template sets: display (titles) and text.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Faces {
    pub display: Option<String>,
    pub text: Option<String>,
}

impl Faces {
    /// The design's font stacks (Typst's own Libertinus Serif behind each).
    pub(crate) fn stacks(&self) -> Value {
        let stack = |f: &Option<String>| match f {
            Some(f) if f != "Libertinus Serif" => json!([f, "Libertinus Serif"]),
            _ => json!(["Libertinus Serif"]),
        };
        json!({"display": stack(&self.display), "text": stack(&self.text)})
    }
}

/// Merges `over` into `base` object by object: a designer's changes to a
/// template's design (`"design": {"styles": {"title": {"size": 48}}}`).
pub(crate) fn merge(base: &mut Value, over: &Value) {
    match (base, over) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(k) {
                    Some(slot) => merge(slot, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (b, o) => *b = o.clone(),
    }
}

/// A page of centred lines in the safe area: a title page, a dedication.
pub(crate) fn centred(x: f64, y: f64, w: f64, style: &str, text: &str, id: &str) -> Value {
    json!({"type": "text", "id": id, "x": pt(x), "y": pt(y), "w": pt(w), "style": style, "text": text})
}
