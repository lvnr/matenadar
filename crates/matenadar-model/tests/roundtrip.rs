//! Documents survive being written out and read back, as every operation
//! does to them.

use matenadar_model::Document;

#[test]
fn every_sample_reads_back_as_it_was_written() {
    let samples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples");
    let mut seen = 0;
    for entry in std::fs::read_dir(&samples).expect("samples") {
        let path = entry.unwrap().path().join("doc.json");
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let doc = Document::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let again = Document::from_json(&doc.to_json()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if doc != again {
            let (a, b) = (serde_json::to_value(&doc).unwrap(), serde_json::to_value(&again).unwrap());
            panic!("{} changes when written out and read back, at {}", path.display(), first_difference(&a, &b, String::new()).unwrap_or_default());
        }
        seen += 1;
    }
    assert!(seen >= 8);
}

#[test]
fn a_length_line_height_stays_a_length() {
    let doc = Document::from_json(
        r#"{"matenadar": "1", "meta": {"title": "t"}, "design": {"base": {"line_height": "14pt"}, "styles": {"a": {"line_height": 1.4}}}, "content": []}"#,
    )
    .unwrap();
    let text = doc.to_json();
    assert!(text.contains(r#""line_height": "14pt""#), "{text}");
    assert!(text.contains(r#""line_height": 1.4"#), "{text}");
}

fn first_difference(a: &serde_json::Value, b: &serde_json::Value, at: String) -> Option<String> {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                match y.get(k) {
                    Some(w) => {
                        if let Some(d) = first_difference(v, w, format!("{at}/{k}")) {
                            return Some(d);
                        }
                    }
                    None => return Some(format!("{at}/{k} (gone)")),
                }
            }
            y.keys().find(|k| !x.contains_key(*k)).map(|k| format!("{at}/{k} (new)"))
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => x.iter().zip(y).enumerate().find_map(|(i, (v, w))| first_difference(v, w, format!("{at}/{i}"))),
        _ if a == b => None,
        _ => Some(format!("{at}: {a} became {b}")),
    }
}
