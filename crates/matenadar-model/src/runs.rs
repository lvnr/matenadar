//! A block's words as runs of inline content, for editing them on the page:
//! cut at a byte offset of the plain text, marked over a range, and tidied.
//! Text runs are strings or objects with `text`; notes, fields, breaks and
//! pictures give no text and are kept as they are.

use serde_json::Value;

/// Run fields whose `false` is the same as leaving them out.
const FLAGS: [&str; 5] = ["em", "strong", "sup", "sub", "nobreak"];

/// The text a run gives the plain text, if it is a text run.
pub fn text_of(run: &Value) -> Option<&str> {
    match run {
        Value::String(s) => Some(s),
        Value::Object(m) => m.get("text").and_then(Value::as_str),
        _ => None,
    }
}

/// A node's words as runs: its `text` as one run, or its `content`.
pub fn runs_of(node: &Value) -> Vec<Value> {
    match node.get("text") {
        Some(Value::String(t)) if t.is_empty() => Vec::new(),
        Some(Value::String(t)) => vec![Value::String(t.clone())],
        _ => node.get("content").and_then(Value::as_array).cloned().unwrap_or_default(),
    }
}

/// The plain text of runs.
pub fn plain(runs: &[Value]) -> String {
    runs.iter().filter_map(text_of).collect()
}

/// The words as plain `text`, when they have no marks or anything but text.
pub fn as_text(runs: &[Value]) -> Option<String> {
    runs.iter().all(Value::is_string).then(|| plain(runs))
}

/// A text run with other words, its marks kept.
fn with_text(run: &Value, text: &str) -> Value {
    match run {
        Value::Object(m) => {
            let mut m = m.clone();
            m.insert("text".into(), Value::String(text.into()));
            Value::Object(m)
        }
        _ => Value::String(text.into()),
    }
}

/// The runs cut at a byte offset of their plain text: those before and
/// those after. Something without text (a note, a field) at the cut stays
/// with the words before it, unless the cut is at the start.
pub fn split_at(runs: &[Value], offset: usize) -> Result<(Vec<Value>, Vec<Value>), String> {
    let total: usize = runs.iter().filter_map(text_of).map(str::len).sum();
    if offset > total {
        return Err(format!("offset {offset} is past the end of the words ({total} bytes)"));
    }
    let (mut before, mut after) = (Vec::new(), Vec::new());
    let mut at = 0;
    for run in runs {
        let Some(t) = text_of(run) else {
            if at < offset || (at == offset && offset > 0) { before.push(run.clone()) } else { after.push(run.clone()) }
            continue;
        };
        let (rs, re) = (at, at + t.len());
        at = re;
        if re <= offset {
            before.push(run.clone());
        } else if rs >= offset {
            after.push(run.clone());
        } else {
            let cut = offset - rs;
            if !t.is_char_boundary(cut) {
                return Err(format!("offset {offset} is not a character boundary"));
            }
            before.push(with_text(run, &t[..cut]));
            after.push(with_text(run, &t[cut..]));
        }
    }
    Ok((tidy(before), tidy(after)))
}

/// Sets `field` on the words in `start..end`, cutting runs at both ends: a
/// null value takes the mark off, as `false` does for a flag such as `em`.
pub fn set_mark(runs: &[Value], start: usize, end: usize, field: &str, value: &Value) -> Result<Vec<Value>, String> {
    if field == "text" || field.is_empty() {
        return Err(format!("\"{field}\" is not a mark"));
    }
    let (start, end) = (start.min(end), start.max(end));
    let (head, rest) = split_at(runs, start)?;
    let (mid, tail) = split_at(&rest, end - start)?;
    let off = value.is_null() || (*value == Value::Bool(false) && FLAGS.contains(&field));
    let mid = mid.into_iter().map(|run| match run {
        Value::String(s) if !off => {
            let mut m = serde_json::Map::new();
            m.insert("text".into(), Value::String(s));
            m.insert(field.into(), value.clone());
            Value::Object(m)
        }
        Value::Object(mut m) if m.get("text").is_some_and(Value::is_string) => {
            if off {
                m.shift_remove(field);
            } else {
                m.insert(field.into(), value.clone());
            }
            Value::Object(m)
        }
        other => other,
    });
    Ok(tidy(head.into_iter().chain(mid).chain(tail).collect()))
}

/// The words of two blocks as one.
pub fn join(a: Vec<Value>, b: Vec<Value>) -> Vec<Value> {
    tidy(a.into_iter().chain(b).collect())
}

/// Runs written as simply as they can be: empty text runs gone, neighbours
/// with the same marks joined, a run with nothing but text a string.
pub fn tidy(runs: Vec<Value>) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for run in runs.into_iter().map(simplest) {
        if text_of(&run).is_some_and(str::is_empty) {
            continue;
        }
        if let Some(last) = out.last_mut()
            && same_marks(last, &run)
        {
            let words = format!("{}{}", text_of(last).unwrap_or(""), text_of(&run).unwrap_or(""));
            *last = with_text(last, &words);
            continue;
        }
        out.push(run);
    }
    out
}

/// A text run without flags set to false, and a string when only its
/// words are left.
fn simplest(run: Value) -> Value {
    match run {
        Value::Object(mut m) if m.get("text").is_some_and(Value::is_string) => {
            m.retain(|k, v| !(FLAGS.contains(&k.as_str()) && *v == Value::Bool(false)));
            if m.len() == 1 { m.shift_remove("text").unwrap_or_default() } else { Value::Object(m) }
        }
        other => other,
    }
}

/// Whether two runs are text with the same marks.
fn same_marks(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(_), Value::String(_)) => true,
        (Value::Object(x), Value::Object(y)) if x.get("text").is_some_and(Value::is_string) && y.get("text").is_some_and(Value::is_string) => {
            x.len() == y.len() && x.iter().all(|(k, v)| k == "text" || y.get(k) == Some(v))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn runs(v: Value) -> Vec<Value> {
        v.as_array().unwrap().clone()
    }

    #[test]
    fn a_cut_keeps_the_marks_on_both_sides() {
        let r = runs(json!(["Before ", {"text": "in the middle", "em": true}, " after."]));
        let (a, b) = split_at(&r, 10).unwrap();
        assert_eq!(a, runs(json!(["Before ", {"text": "in ", "em": true}])));
        assert_eq!(b, runs(json!([{"text": "the middle", "em": true}, " after."])));
        assert_eq!(join(a, b), r);
        let (a, b) = split_at(&r, 0).unwrap();
        assert!(a.is_empty());
        assert_eq!(b, r);
        assert!(split_at(&r, 99).is_err());
        assert!(split_at(&runs(json!(["café"])), 4).is_err(), "inside the é");
    }

    #[test]
    fn a_note_stays_with_the_words_before_it() {
        let r = runs(json!(["One", {"note": ["A note."]}, " two"]));
        let (a, b) = split_at(&r, 3).unwrap();
        assert_eq!(a, runs(json!(["One", {"note": ["A note."]}])));
        assert_eq!(b, runs(json!([" two"])));
    }

    #[test]
    fn marks_go_on_and_come_off_a_range() {
        let r = runs(json!(["The night shift kept its hours."]));
        let strong = set_mark(&r, 4, 9, "strong", &json!(true)).unwrap();
        assert_eq!(strong, runs(json!(["The ", {"text": "night", "strong": true}, " shift kept its hours."])));
        let both = set_mark(&strong, 6, 15, "em", &json!(true)).unwrap();
        assert_eq!(
            both,
            runs(json!(["The ", {"text": "ni", "strong": true}, {"text": "ght", "strong": true, "em": true}, {"text": " shift", "em": true}, " kept its hours."]))
        );
        let back = set_mark(&set_mark(&both, 6, 15, "em", &Value::Null).unwrap(), 4, 9, "strong", &json!(false)).unwrap();
        assert_eq!(back, r, "taking the marks off gives the plain words back");
        assert_eq!(as_text(&back).as_deref(), Some("The night shift kept its hours."));
        assert_eq!(set_mark(&r, 9, 4, "color", &json!("#8a2a1c")).unwrap()[1]["color"], json!("#8a2a1c"), "either way round");
        assert!(set_mark(&r, 0, 3, "text", &json!("x")).is_err());
    }
}
