//! Parser-only inline word-style representation for QOL04.
//!
//! The rendering layer is deliberately not enabled until a story output can
//! carry its plain text and structured spans together. This parser gives that
//! integration a bounded and grapheme-safe source of truth rather than
//! interpreting inline tags from the UI update loop.
//!
//! Syntax:
//!   [[fx=ember]]fire[[/fx]]
//!   \[[  (literal opening brackets in the decoded dialogue string)
//! A .vn quoted literal uses "\\[[" to encode the backslash.
//! Tags cannot nest; adjacent tags can differ; each span must cover at
//! least one full extended grapheme cluster.

use unicode_segmentation::UnicodeSegmentation;
use serde::{Serialize, Deserialize};

/// Half-open UTF-8 byte range in \`StyledText::plain\`, always aligned to
/// extended grapheme boundaries. Styling must never affect source text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineSpan {
    pub style: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyledText {
    pub plain: String,
    pub spans: Vec<InlineSpan>,
}

pub const MAX_INLINE_BYTES: usize = 32 * 1024;
pub const MAX_INLINE_SPANS: usize = 64;

fn style_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// Parse already quote-decoded narrative text. Never parse markup during
/// rendering: this validation belongs to the story-loading/worker path.
pub fn parse_inline(input: &str) -> Result<StyledText, String> {
    if input.len() > MAX_INLINE_BYTES {
        return Err(format!("inline text exceeds {MAX_INLINE_BYTES} UTF-8 bytes"));
    }
    let mut plain = String::with_capacity(input.len());
    let mut spans = Vec::new();
    let mut opened: Option<(String, usize)> = None;
    let mut i = 0_usize;
    while i < input.len() {
        let rest = &input[i..];
        if rest.starts_with(r"\[[") {
            plain.push_str("[[");
            i += 3;
            continue;
        }
        if let Some(tag_body) = rest.strip_prefix("[[") {
            let Some(end) = tag_body.find("]]") else {
                return Err("unterminated inline style tag".into());
            };
            let tag = &tag_body[..end];
            if let Some(id) = tag.strip_prefix("fx=") {
                if !style_identifier(id) {
                    return Err(format!("invalid inline style identifier '{id}'"));
                }
                if opened.is_some() {
                    return Err("nested inline style tags are not supported".into());
                }
                if spans.len() == MAX_INLINE_SPANS {
                    return Err("too many inline style spans".into());
                }
                opened = Some((id.to_owned(), plain.len()));
            } else if tag == "/fx" {
                let Some((style, start)) = opened.take() else {
                    return Err("closing inline style tag without opening tag".into());
                };
                if plain.len() == start {
                    return Err("empty inline style span".into());
                }
                spans.push(InlineSpan {
                    style, start, end: plain.len(),
                });
            } else {
                return Err(format!("unknown inline style tag '{tag}'"));
            }
            i += 2 + end + 2;
            continue;
        }
        let c = rest.chars().next().expect("nonempty remaining UTF-8");
        plain.push(c);
        i += c.len_utf8();
    }
    if opened.is_some() {
        return Err("unclosed inline style tag".into());
    }

    // Bytes are always UTF-8 safe by construction, but a tag could still
    // bisect e + combining acute or an emoji ZWJ sequence. Reject instead
    // of letting Bevy split a grapheme into separate style TextSpans.
    let mut boundaries = Vec::new();
    boundaries.push(0);
    boundaries.extend(
        UnicodeSegmentation::grapheme_indices(plain.as_str(), true)
            .map(|(start, grapheme)| start + grapheme.len()),
    );
    for span in &spans {
        if boundaries.binary_search(&span.start).is_err()
            || boundaries.binary_search(&span.end).is_err()
        {
            return Err("style boundary splits a Unicode grapheme cluster".into());
        }
    }
    Ok(StyledText { plain, spans })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_markup_preserves_legacy_dialogue_byte_for_byte() {
        for sample in ["", "Buenas noches. おやすみ。", "👩‍💻 e\u{301} 🇲🇽", "just plain text"] {
            let parsed = parse_inline(sample).unwrap();
            assert_eq!(parsed.plain, sample);
            assert!(parsed.spans.is_empty());
        }
    }

    #[test]
    fn distinct_adjacent_styles_preserve_plain_readable_text() {
        let styled = parse_inline(
            "The [[fx=ember]]fire[[/fx]][[fx=shadow]]night[[/fx]] awakens."
        ).unwrap();
        assert_eq!(styled.plain, "The firenight awakens.");
        assert_eq!(styled.spans.len(), 2);
        assert_eq!(styled.spans[0].style, "ember");
        assert_eq!(&styled.plain[styled.spans[0].start..styled.spans[0].end], "fire");
        assert_eq!(styled.spans[1].style, "shadow");
        assert_eq!(&styled.plain[styled.spans[1].start..styled.spans[1].end], "night");
    }

    #[test]
    fn escaped_markers_are_literal_and_not_interpreted() {
        let styled = parse_inline(r"The \[[ sign is literal; [[fx=a]]ok[[/fx]]").unwrap();
        assert_eq!(styled.plain, "The [[ sign is literal; ok");
        assert_eq!(styled.spans.len(), 1);
        assert_eq!(&styled.plain[styled.spans[0].start..styled.spans[0].end], "ok");
    }

    #[test]
    fn never_split_unicode_graphemes() {
        let styled = parse_inline(
            "Start [[fx=spark]]👨‍👩‍👧‍👦e\u{301}🇲🇽नमस्ते[[/fx]] done."
        ).unwrap();
        assert!(styled.plain.contains("👨‍👩‍👧‍👦"));
        assert_eq!(&styled.plain[styled.spans[0].start..styled.spans[0].end],
                   "👨‍👩‍👧‍👦e\u{301}🇲🇽नमस्ते");
        assert!(parse_inline("e[[fx=glow]]\u{301}[[/fx]]").unwrap_err()
            .contains("grapheme"));
        assert!(parse_inline("[[fx=glow]]👨[[/fx]]‍👩").unwrap_err()
            .contains("grapheme"));
    }

    #[test]
    fn malformed_or_nested_markup_reports_actionable_errors() {
        for invalid in [
            "[[fx=ember]]word", "[[/fx]]", "[[fx=ember]]",
            "[[fx=ember]]x[[fx=blue]]y[[/fx]][[/fx]]",
            "[[fx=no spaces]]x[[/fx]]", "[[bad]]",
            "[[fx=glow]]x[[/unknown]]",
            "broken [[fx=glow",
        ] {
            assert!(parse_inline(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn fixed_span_and_source_limits_prevent_unbounded_work() {
        assert!(parse_inline(&"a".repeat(MAX_INLINE_BYTES + 1)).is_err());
        let many = "[[fx=a]]x[[/fx]]".repeat(MAX_INLINE_SPANS + 1);
        assert!(parse_inline(&many).unwrap_err().contains("too many"));
        assert_eq!(parse_inline(&"[[fx=a]]x[[/fx]]".repeat(MAX_INLINE_SPANS))
            .unwrap().spans.len(), MAX_INLINE_SPANS);
    }
}
