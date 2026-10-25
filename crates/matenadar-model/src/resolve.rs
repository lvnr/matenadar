//! Style inheritance: a style's settings are its own, then those of the
//! style it inherits from, then the design's base.

use crate::style::{Design, Master, Style, TextStyle};

macro_rules! overlay {
    ($dst:expr, $src:expr; $($field:ident),* $(,)?) => {
        $( if $src.$field.is_some() { $dst.$field = $src.$field.clone(); } )*
    };
}

impl TextStyle {
    /// `self` with every field `over` sets replaced.
    pub fn with(&self, over: &TextStyle) -> TextStyle {
        let mut out = self.clone();
        overlay!(out, over; font, size, weight, italic, stretch, tracking, word_spacing, color, case, features,
            baseline, underline, strike, lang, outline, hyphenate, ligatures, variations);
        out
    }
}

impl Style {
    /// `self` with every field `over` sets replaced (`inherits` is not carried).
    pub fn with(&self, over: &Style) -> Style {
        let mut out = self.clone();
        out.text = self.text.with(&over.text);
        overlay!(out, over; line_height, align, indent, indent_first, hanging, margin_left, margin_right,
            space_before, space_after, keep_with_next, keep_together, allow_widows, allow_orphans,
            page_break_before, drop_cap, rule_above, rule_below, balance, fill, inset);
        out.inherits = None;
        out
    }
}

impl Master {
    pub fn with(&self, over: &Master) -> Master {
        let mut out = self.clone();
        overlay!(out, over; width, height, bleed, margins, columns, gutter, fill, notes, style);
        if !over.background.is_empty() {
            out.background = over.background.clone();
        }
        if !over.foreground.is_empty() {
            out.foreground = over.foreground.clone();
        }
        out.inherits = None;
        out
    }
}

impl Design {
    /// The style `name` with its inheritance resolved, over the base. An
    /// unknown name gives the base alone.
    pub fn style(&self, name: &str) -> Style {
        let mut chain = Vec::new();
        let mut at = Some(name.to_string());
        while let Some(n) = at {
            if chain.contains(&n) || chain.len() > 32 {
                break;
            }
            match self.styles.get(&n) {
                Some(s) => {
                    at = s.inherits.clone();
                    chain.push(n);
                }
                None => break,
            }
        }
        let mut out = self.base.with(&Style::default());
        for n in chain.iter().rev() {
            out = out.with(&self.styles[n]);
        }
        out
    }

    /// The master `name` with its inheritance resolved.
    pub fn master(&self, name: &str) -> Option<Master> {
        let mut chain = Vec::new();
        let mut at = Some(name.to_string());
        while let Some(n) = at {
            if chain.contains(&n) || chain.len() > 32 {
                break;
            }
            let m = self.masters.get(&n)?;
            at = m.inherits.clone();
            chain.push(n);
        }
        let mut out = Master::default();
        for n in chain.iter().rev() {
            out = out.with(&self.masters[n]);
        }
        Some(out)
    }

    /// Whether `name` inherits from itself somewhere along its chain.
    pub fn style_cycle(&self, name: &str) -> bool {
        let mut seen = Vec::new();
        let mut at = Some(name.to_string());
        while let Some(n) = at {
            if seen.contains(&n) {
                return true;
            }
            seen.push(n.clone());
            at = self.styles.get(&n).and_then(|s| s.inherits.clone());
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Length;
    use serde_json::json;

    #[test]
    fn styles_inherit_and_override() {
        let design: Design = serde_json::from_value(json!({
            "base": {"font": "Body Serif", "size": 10, "line_height": 1.4},
            "styles": {
                "body": {"indent": "1em"},
                "quote": {"inherits": "body", "italic": true, "size": 9},
            }
        }))
        .unwrap();
        let q = design.style("quote");
        assert_eq!(q.text.size, Some(Length::Pt(9.0)));
        assert_eq!(q.text.italic, Some(true));
        assert_eq!(q.indent, Some(Length::Em(1.0)));
        assert_eq!(q.text.font.unwrap().families(), vec!["Body Serif"]);
        assert_eq!(design.style("nope").text.size, Some(Length::Pt(10.0)));
    }

    #[test]
    fn cycles_are_found_not_followed() {
        let design: Design = serde_json::from_value(json!({
            "styles": {"a": {"inherits": "b"}, "b": {"inherits": "a"}}
        }))
        .unwrap();
        assert!(design.style_cycle("a"));
        let _ = design.style("a");
    }
}
