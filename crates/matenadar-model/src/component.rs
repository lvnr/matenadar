//! Components: named, reusable compositions of elements or blocks with
//! parameters. An instance's props replace `{{name}}` placeholders in the
//! component's body: a string that is exactly `"{{name}}"` takes the prop's
//! value as it is (a number, a list, an object); a placeholder inside a
//! longer string is replaced by the prop's text.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::block::Block;
use crate::element::Element;
use crate::units::Length;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Component {
    /// Parameters and their defaults (`null` for a required one).
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub params: IndexMap<String, Value>,
    /// The component's size, when it is made of elements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<Length>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<Length>,
    /// Placed elements, positioned from the component's top-left corner.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<Value>,
    /// Blocks, for components used in flows.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Value>,
}

/// A component with its props filled in.
#[derive(Clone, Debug, PartialEq)]
pub struct Expanded {
    pub w: Option<Length>,
    pub h: Option<Length>,
    pub elements: Vec<Element>,
    pub blocks: Vec<Block>,
}

impl Component {
    pub fn expand(&self, name: &str, props: &IndexMap<String, Value>) -> Result<Expanded, String> {
        for key in props.keys() {
            if !self.params.contains_key(key) {
                return Err(format!("component \"{name}\" has no parameter \"{key}\""));
            }
        }
        let mut values = IndexMap::new();
        for (key, default) in &self.params {
            match props.get(key).or(Some(default)).filter(|v| !v.is_null()) {
                Some(v) => {
                    values.insert(key.clone(), v.clone());
                }
                None => return Err(format!("component \"{name}\" needs the parameter \"{key}\"")),
            }
        }
        let fill = |v: &Value| substitute(v, &values);
        let elements = self
            .elements
            .iter()
            .map(fill)
            .map(|v| serde_json::from_value::<Element>(v).map_err(|e| format!("component \"{name}\": {e}")))
            .collect::<Result<Vec<_>, _>>()?;
        let blocks = self
            .blocks
            .iter()
            .map(fill)
            .map(|v| serde_json::from_value::<Block>(v).map_err(|e| format!("component \"{name}\": {e}")))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Expanded { w: self.w, h: self.h, elements, blocks })
    }
}

fn substitute(v: &Value, values: &IndexMap<String, Value>) -> Value {
    match v {
        Value::String(s) => {
            if let Some(name) = s.strip_prefix("{{").and_then(|r| r.strip_suffix("}}"))
                && !name.contains("{{")
                && let Some(value) = values.get(name.trim())
            {
                return value.clone();
            }
            if !s.contains("{{") {
                return v.clone();
            }
            let mut out = String::with_capacity(s.len());
            let mut rest = s.as_str();
            while let Some(start) = rest.find("{{") {
                out.push_str(&rest[..start]);
                match rest[start + 2..].find("}}") {
                    Some(end) => {
                        let name = rest[start + 2..start + 2 + end].trim();
                        match values.get(name) {
                            Some(Value::String(t)) => out.push_str(t),
                            Some(Value::Null) | None => out.push_str(&rest[start..start + 4 + end]),
                            Some(other) => out.push_str(&other.to_string()),
                        }
                        rest = &rest[start + 4 + end..];
                    }
                    None => {
                        out.push_str(&rest[start..]);
                        rest = "";
                    }
                }
            }
            out.push_str(rest);
            Value::String(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(|i| substitute(i, values)).collect()),
        Value::Object(map) => Value::Object(map.iter().map(|(k, i)| (k.clone(), substitute(i, values))).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn card() -> Component {
        serde_json::from_value(json!({
            "params": {"title": null, "n": 3, "tags": []},
            "w": "2in", "h": "1in",
            "elements": [
                {"type": "text", "x": 0, "y": 0, "w": "{{n}}", "text": "No. {{n}}: {{title}}"},
                {"type": "group", "x": 0, "y": 20, "children": "{{tags}}"}
            ]
        }))
        .unwrap()
    }

    #[test]
    fn fills_placeholders_with_text_and_values() {
        let props: IndexMap<String, Value> = [
            ("title".to_string(), json!("Sloth")),
            ("tags".to_string(), json!([{"type": "rect", "x": 1, "y": 1, "w": 4, "h": 4}])),
        ]
        .into_iter()
        .collect();
        let e = card().expand("card", &props).unwrap();
        match &e.elements[0] {
            Element::Text(t) => {
                assert_eq!(t.text.as_deref(), Some("No. 3: Sloth"));
                assert_eq!(t.place.w, Some(Length::Pt(3.0)));
            }
            other => panic!("{other:?}"),
        }
        match &e.elements[1] {
            Element::Group(g) => assert_eq!(g.children.len(), 1),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn refuses_unknown_and_missing_params() {
        let none = IndexMap::new();
        assert!(card().expand("card", &none).unwrap_err().contains("needs the parameter \"title\""));
        let bad: IndexMap<String, Value> = [("title".to_string(), json!("x")), ("colour".to_string(), json!("red"))].into_iter().collect();
        assert!(card().expand("card", &bad).unwrap_err().contains("no parameter \"colour\""));
    }
}
