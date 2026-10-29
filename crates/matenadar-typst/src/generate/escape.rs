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

/// `text` with the breaks of hyphenation exceptions written in: soft hyphens
/// where an exception allows a break, and a word joiner after a word that
/// must not break. Either keeps Typst's patterns off the word. Also returns
/// where the added characters' bytes are. `exceptions` maps a lowercase word
/// to the character counts before each allowed break.
pub fn with_breaks(text: &str, exceptions: &std::collections::HashMap<String, Vec<usize>>) -> (String, Vec<usize>) {
    // No exceptions (most books): nothing to add, no word to look up.
    if exceptions.is_empty() {
        return (text.to_string(), Vec::new());
    }
    // Only words as long as some exception are looked up (lowercased into one buffer).
    let (shortest, longest) = exceptions.keys().map(|k| k.chars().count()).fold((usize::MAX, 0), |(lo, hi), n| (lo.min(n), hi.max(n)));
    let mut lower = String::new();
    let mut out = String::with_capacity(text.len() + 8);
    let mut added = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let word_len: usize = rest.chars().take_while(|c| c.is_alphabetic()).map(char::len_utf8).sum();
        if word_len == 0 {
            let c = rest.chars().next().unwrap_or(' ');
            out.push(c);
            rest = &rest[c.len_utf8()..];
            continue;
        }
        let word = &rest[..word_len];
        let chars = word.chars().count();
        let found = if chars < shortest || chars > longest {
            None
        } else {
            lower.clear();
            lower.extend(word.chars().flat_map(char::to_lowercase));
            exceptions.get(lower.as_str())
        };
        match found {
            Some(breaks) => {
                let mark = |out: &mut String, added: &mut Vec<usize>, c: char| {
                    added.extend(out.len()..out.len() + c.len_utf8());
                    out.push(c);
                };
                for (i, c) in word.chars().enumerate() {
                    if i > 0 && breaks.contains(&i) {
                        mark(&mut out, &mut added, '\u{ad}');
                    }
                    out.push(c);
                }
                if breaks.is_empty() {
                    mark(&mut out, &mut added, '\u{2060}');
                }
            }
            None => out.push_str(word),
        }
        rest = &rest[word_len..];
    }
    (out, added)
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
    fn writes_the_breaks_of_exceptions() {
        let ex = std::collections::HashMap::from([("describes".to_string(), vec![2]), ("bookforge".to_string(), vec![])]);
        let (out, added) = with_breaks("It Describes Bookforge.", &ex);
        assert_eq!(out, "It De\u{ad}scribes Bookforge\u{2060}.");
        assert_eq!(added, [5, 6, 24, 25, 26]);
    }

    #[test]
    fn leaves_plain_and_curly_text_alone() {
        let (out, at) = escape("“Dot,” she said… — fine");
        assert_eq!(out, "“Dot,” she said… — fine");
        assert!(at.is_empty());
    }
}
