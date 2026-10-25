//! Operations: every change to a document, by a person, an agent or a
//! program, is one of these. Each returns its inverse, so changes can be
//! undone, replayed and sent over the wire.
//!
//! A node is addressed by its `id`, or by `at`, a JSON pointer from the
//! document's root (`"/design/styles/body"`, `"/content/2/flow"`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Document;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum Op {
    /// Sets a field: `path` is a JSON pointer inside the node (`"/style"`,
    /// `"/set/size"`). Missing objects on the way are created.
    Set {
        #[serde(flatten)]
        node: Target,
        path: String,
        value: Value,
    },
    /// Removes a field.
    Unset {
        #[serde(flatten)]
        node: Target,
        path: String,
    },
    /// Inserts `value` into the array `field` of the node (at the end
    /// without an `index`).
    Insert {
        #[serde(flatten)]
        node: Target,
        field: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
        value: Value,
    },
    /// Removes the node from the array that holds it.
    Remove {
        #[serde(flatten)]
        node: Target,
    },
    /// Moves the node into the array `field` of `to`.
    Move {
        #[serde(flatten)]
        node: Target,
        to: Target,
        field: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
    /// Inserts text at a byte offset of a paragraph's or heading's plain text.
    InsertText {
        #[serde(flatten)]
        node: Target,
        offset: usize,
        text: String,
    },
    /// Deletes the byte range `start..end` of the plain text.
    DeleteText {
        #[serde(flatten)]
        node: Target,
        start: usize,
        end: usize,
    },
}

/// A node: by id, or by a JSON pointer from the root (`""` is the root).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Target {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

impl Target {
    pub fn id(id: impl Into<String>) -> Self {
        Self { id: Some(id.into()), at: None }
    }

    pub fn at(pointer: impl Into<String>) -> Self {
        Self { id: None, at: Some(pointer.into()) }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpError(pub String);

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for OpError {}

fn err<T>(msg: impl Into<String>) -> Result<T, OpError> {
    Err(OpError(msg.into()))
}

impl Document {
    /// Applies `op` and returns its inverse. The document is unchanged when
    /// the operation fails or would leave it malformed.
    pub fn apply(&mut self, op: &Op) -> Result<Op, OpError> {
        let mut root = serde_json::to_value(&*self).map_err(|e| OpError(e.to_string()))?;
        let inverse = apply_value(&mut root, op)?;
        let text = serde_json::to_string(&root).map_err(|e| OpError(e.to_string()))?;
        *self = Document::from_json(&text).map_err(|e| OpError(format!("the change would leave the document malformed: {e}")))?;
        Ok(inverse)
    }
}

/// Applies `op` to a document's JSON and returns its inverse.
pub fn apply_value(root: &mut Value, op: &Op) -> Result<Op, OpError> {
    match op {
        Op::Set { node, path, value } => {
            let base = resolve(root, node)?;
            let full = format!("{base}{path}");
            let old = root.pointer(&full).cloned();
            set_pointer(root, &full, value.clone())?;
            Ok(match old {
                Some(v) => Op::Set { node: Target::at(base), path: path.clone(), value: v },
                None => Op::Unset { node: Target::at(base), path: path.clone() },
            })
        }
        Op::Unset { node, path } => {
            let base = resolve(root, node)?;
            let full = format!("{base}{path}");
            let (parent, key) = split(&full)?;
            let obj = root.pointer_mut(&parent).and_then(Value::as_object_mut).ok_or_else(|| OpError(format!("nothing at {full}")))?;
            let old = obj.shift_remove(&key).ok_or_else(|| OpError(format!("nothing at {full}")))?;
            Ok(Op::Set { node: Target::at(base), path: path.clone(), value: old })
        }
        Op::Insert { node, field, index, value } => {
            let base = resolve(root, node)?;
            let arr_ptr = format!("{base}/{}", escape(field));
            if root.pointer(&arr_ptr).is_none() {
                set_pointer(root, &arr_ptr, Value::Array(vec![]))?;
            }
            let arr = root.pointer_mut(&arr_ptr).and_then(Value::as_array_mut).ok_or_else(|| OpError(format!("{arr_ptr} is not a list")))?;
            let i = index.unwrap_or(arr.len());
            if i > arr.len() {
                return err(format!("index {i} is past the end of {arr_ptr} ({} items)", arr.len()));
            }
            if let Some(id) = value.get("id").and_then(Value::as_str)
                && find_id(root, id).is_some()
            {
                return err(format!("the id \"{id}\" is already used"));
            }
            let arr = root.pointer_mut(&arr_ptr).and_then(Value::as_array_mut).expect("checked above");
            arr.insert(i, value.clone());
            Ok(Op::Remove { node: Target::at(format!("{arr_ptr}/{i}")) })
        }
        Op::Remove { node } => {
            let ptr = resolve(root, node)?;
            let (parent, key) = split(&ptr)?;
            let i: usize = key.parse().map_err(|_| OpError(format!("{ptr} is not an item of a list")))?;
            let arr = root.pointer_mut(&parent).and_then(Value::as_array_mut).ok_or_else(|| OpError(format!("{ptr} is not an item of a list")))?;
            if i >= arr.len() {
                return err(format!("nothing at {ptr}"));
            }
            let old = arr.remove(i);
            let (holder, field) = split(&parent)?;
            Ok(Op::Insert { node: Target::at(holder), field: unescape(&field), index: Some(i), value: old })
        }
        Op::Move { node, to, field, index } => {
            let from = resolve(root, node)?;
            let value = root.pointer(&from).cloned().ok_or_else(|| OpError(format!("nothing at {from}")))?;
            let target_before = resolve(root, to)?;
            if target_before.starts_with(&from) {
                return err("can't move a node into itself");
            }
            let undo_remove = apply_value(root, &Op::Remove { node: Target::at(from.clone()) })?;
            // Removing may shift the target's pointer; find it again by id when possible.
            let target = resolve(root, to).or_else(|_| Ok::<_, OpError>(target_before))?;
            let Op::Insert { node: old_holder, field: old_field, index: old_index, .. } = undo_remove else { unreachable!() };
            let undo_insert = apply_value(root, &Op::Insert { node: Target::at(target), field: field.clone(), index: *index, value: Value::Null })
                .and_then(|inv| {
                    // Put the real value where the placeholder went.
                    if let Op::Remove { node: Target { at: Some(p), .. } } = &inv {
                        *root.pointer_mut(p).expect("just inserted") = value.clone();
                    }
                    Ok(inv)
                })?;
            let Op::Remove { node: new_at } = undo_insert else { unreachable!() };
            Ok(Op::Move { node: new_at, to: old_holder, field: old_field, index: old_index })
        }
        Op::InsertText { node, offset, text } => {
            let ptr = resolve(root, node)?;
            let target = root.pointer_mut(&ptr).ok_or_else(|| OpError(format!("nothing at {ptr}")))?;
            let old_content = target.get("content").cloned();
            let (_, touched) = edit_text(target, *offset, *offset, text)?;
            if let Some(old) = old_content
                && touched == 0
            {
                return Ok(Op::Set { node: Target::at(ptr), path: "/content".into(), value: old });
            }
            Ok(Op::DeleteText { node: Target::at(ptr), start: *offset, end: offset + text.len() })
        }
        Op::DeleteText { node, start, end } => {
            let ptr = resolve(root, node)?;
            let target = root.pointer_mut(&ptr).ok_or_else(|| OpError(format!("nothing at {ptr}")))?;
            let old_content = target.get("content").cloned();
            let (removed, touched) = edit_text(target, *start, *end, "")?;
            // Across runs, putting the text back can't restore each run's
            // marks, so the inverse restores the runs themselves.
            match old_content {
                Some(old) if touched > 1 => Ok(Op::Set { node: Target::at(ptr), path: "/content".into(), value: old }),
                _ => Ok(Op::InsertText { node: Target::at(ptr), offset: *start, text: removed }),
            }
        }
    }
}

fn resolve(root: &Value, t: &Target) -> Result<String, OpError> {
    match (&t.id, &t.at) {
        (Some(id), None) => find_id(root, id).ok_or_else(|| OpError(format!("no node with id \"{id}\""))),
        (None, Some(at)) => {
            if at.is_empty() || root.pointer(at).is_some() {
                Ok(at.clone())
            } else {
                err(format!("nothing at {at}"))
            }
        }
        _ => err("address a node with either id or at"),
    }
}

/// The JSON pointer of the object whose `id` is `id`.
pub fn find_id(root: &Value, id: &str) -> Option<String> {
    fn go(v: &Value, id: &str, path: &mut String) -> bool {
        match v {
            Value::Object(map) => {
                if map.get("id").and_then(Value::as_str) == Some(id) {
                    return true;
                }
                for (k, child) in map {
                    let len = path.len();
                    path.push('/');
                    path.push_str(&escape(k));
                    if go(child, id, path) {
                        return true;
                    }
                    path.truncate(len);
                }
                false
            }
            Value::Array(items) => {
                for (i, child) in items.iter().enumerate() {
                    let len = path.len();
                    path.push_str(&format!("/{i}"));
                    if go(child, id, path) {
                        return true;
                    }
                    path.truncate(len);
                }
                false
            }
            _ => false,
        }
    }
    let mut path = String::new();
    go(root, id, &mut path).then_some(path)
}

fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn unescape(key: &str) -> String {
    key.replace("~1", "/").replace("~0", "~")
}

fn split(ptr: &str) -> Result<(String, String), OpError> {
    let i = ptr.rfind('/').ok_or_else(|| OpError(format!("{ptr:?} has no parent")))?;
    Ok((ptr[..i].to_string(), unescape(&ptr[i + 1..])))
}

fn set_pointer(root: &mut Value, ptr: &str, value: Value) -> Result<(), OpError> {
    if ptr.is_empty() {
        *root = value;
        return Ok(());
    }
    let (parent, key) = split(ptr)?;
    if root.pointer(&parent).is_none() {
        set_pointer(root, &parent, Value::Object(Default::default()))?;
    }
    match root.pointer_mut(&parent) {
        Some(Value::Object(map)) => {
            map.insert(key, value);
            Ok(())
        }
        Some(Value::Array(items)) => {
            let i: usize = key.parse().map_err(|_| OpError(format!("{ptr}: a list index must be a number")))?;
            if i < items.len() {
                items[i] = value;
                Ok(())
            } else if i == items.len() {
                items.push(value);
                Ok(())
            } else {
                err(format!("index {i} is past the end of {parent}"))
            }
        }
        _ => err(format!("{parent} is not an object or a list")),
    }
}

/// Replaces `start..end` of a block's plain text with `text`, in its `text`
/// field or across its `content` runs. Returns what was removed and how
/// many runs the edit touched.
fn edit_text(node: &mut Value, start: usize, end: usize, text: &str) -> Result<(String, usize), OpError> {
    if start > end {
        return err("start is after end");
    }
    if let Some(Value::String(s)) = node.get_mut("text") {
        if end > s.len() || !s.is_char_boundary(start) || !s.is_char_boundary(end) {
            return err(format!("offsets {start}..{end} are not character boundaries of the text"));
        }
        let removed = s[start..end].to_string();
        s.replace_range(start..end, text);
        return Ok((removed, 1));
    }
    let Some(Value::Array(runs)) = node.get_mut("content") else {
        return err("this node has no text to edit");
    };
    let total: usize = runs.iter().map(|r| run_text(r).map_or(0, str::len)).sum();
    if end > total {
        return err(format!("offset {end} is past the end of the text ({total} bytes)"));
    }
    let mut removed = String::new();
    let mut at = 0;
    let mut inserted = text.is_empty();
    let mut touched = 0;
    for run in runs.iter_mut() {
        let Some(t) = run_text(run).map(str::to_string) else { continue };
        let (rs, re) = (at, at + t.len());
        at = re;
        let (s, e) = (start.max(rs), end.min(re));
        let touches = s < e || (!inserted && start >= rs && start <= re);
        if !touches {
            continue;
        }
        let (ls, le) = (s.min(re) - rs, e.max(s).min(re) - rs);
        if !t.is_char_boundary(ls) || !t.is_char_boundary(le) {
            return err("offsets are not character boundaries of the text");
        }
        removed.push_str(&t[ls..le]);
        let mut new = t.clone();
        let insert_here = !inserted && start >= rs && start <= re;
        new.replace_range(ls..le, if insert_here { text } else { "" });
        inserted |= insert_here;
        touched += 1;
        set_run_text(run, new);
    }
    if !inserted {
        runs.push(Value::String(text.to_string()));
    }
    runs.retain(|r| run_text(r).is_none_or(|t| !t.is_empty()));
    Ok((removed, touched))
}

fn run_text(v: &Value) -> Option<&str> {
    match v {
        Value::String(s) => Some(s),
        Value::Object(m) => m.get("text").and_then(Value::as_str),
        _ => None,
    }
}

fn set_run_text(v: &mut Value, text: String) {
    match v {
        Value::String(s) => *s = text,
        Value::Object(m) => {
            m.insert("text".into(), Value::String(text));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc() -> Document {
        serde_json::from_value(json!({
            "matenadar": "1",
            "meta": {"title": "T"},
            "design": {"styles": {"body": {}}, "masters": {"page": {"width": "6in", "height": "9in"}}},
            "content": [{"type": "section", "id": "s1", "master": "page", "flow": [
                {"type": "p", "id": "p1", "content": ["I was ", {"text": "late", "em": true}, "."]},
                {"type": "p", "id": "p2", "text": "Second."}
            ]}]
        }))
        .unwrap()
    }

    fn roundtrip(op: Op) {
        let mut d = doc();
        let before = d.clone();
        let inv = d.apply(&op).unwrap();
        assert_ne!(d, before, "{op:?} changed nothing");
        d.apply(&inv).unwrap();
        assert_eq!(d, before, "undoing {op:?}");
    }

    #[test]
    fn every_operation_undoes() {
        roundtrip(Op::Set { node: Target::id("p2"), path: "/style".into(), value: json!("body") });
        roundtrip(Op::Set { node: Target::at("/design"), path: "/colors/ink".into(), value: json!("#222") });
        roundtrip(Op::Unset { node: Target::id("p2"), path: "/text".into() });
        roundtrip(Op::Insert { node: Target::id("s1"), field: "flow".into(), index: Some(1), value: json!({"type": "p", "id": "p9", "text": "New"}) });
        roundtrip(Op::Remove { node: Target::id("p1") });
        roundtrip(Op::Move { node: Target::id("p2"), to: Target::id("s1"), field: "flow".into(), index: Some(0) });
        roundtrip(Op::InsertText { node: Target::id("p1"), offset: 6, text: "very ".into() });
        roundtrip(Op::DeleteText { node: Target::id("p1"), start: 4, end: 10 });
        roundtrip(Op::InsertText { node: Target::id("p2"), offset: 0, text: "A ".into() });
    }

    #[test]
    fn edits_text_across_runs() {
        let mut d = doc();
        d.apply(&Op::DeleteText { node: Target::id("p1"), start: 4, end: 10 }).unwrap();
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v.pointer("/content/0/flow/0/content").unwrap(), &json!(["I wa", "."]));
    }

    #[test]
    fn refuses_bad_changes_and_leaves_the_document_alone() {
        let mut d = doc();
        let before = d.clone();
        assert!(d.apply(&Op::Remove { node: Target::id("ghost") }).is_err());
        assert!(d.apply(&Op::Set { node: Target::id("p1"), path: "/type".into(), value: json!("nonsense") }).is_err());
        assert!(d.apply(&Op::Insert { node: Target::id("s1"), field: "flow".into(), index: None, value: json!({"type": "p", "id": "p1"}) }).is_err());
        assert_eq!(d, before);
    }

    #[test]
    fn operations_round_trip_through_json() {
        let op = Op::Set { node: Target::id("p1"), path: "/style".into(), value: json!("body") };
        let text = serde_json::to_string(&op).unwrap();
        assert_eq!(text, r#"{"op":"set","id":"p1","path":"/style","value":"body"}"#);
        assert_eq!(serde_json::from_str::<Op>(&text).unwrap(), op);
    }
}
