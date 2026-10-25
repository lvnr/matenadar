/// Escapes `text` for Typst markup. Returns the markup and the offsets, in
/// the markup, of every backslash added, so a source offset can be mapped
/// back to an offset in `text`. Newlines are not handled here: the
/// generator turns them into `#linebreak()`.
pub fn escape(text: &str) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(text.len() + text.len() / 16);
    let mut escapes = Vec::new();
    for c in text.chars() {
        if matches!(
            c,
            '\\' | '#' | '*' | '_' | '`' | '$' | '<' | '>' | '@' | '[' | ']' | '~' | '-' | '.' | '\'' | '"' | '/' | '='
                | '+'
        ) {
            escapes.push(out.len());
            out.push('\\');
        }
        out.push(c);
    }
    (out, escapes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_markup_characters_and_records_where() {
        let (out, at) = escape("a#b 1. c//d");
        assert_eq!(out, r"a\#b 1\. c\/\/d");
        assert_eq!(at, [1, 6, 10, 12]);
    }

    #[test]
    fn leaves_plain_and_curly_text_alone() {
        let (out, at) = escape("“Dot,” she said… — fine");
        assert_eq!(out, "“Dot,” she said… — fine");
        assert!(at.is_empty());
    }
}
