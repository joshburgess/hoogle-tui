pub fn truncate_chars(text: &str, max_chars: usize, suffix: &str) -> String {
    let text_len = text.chars().count();
    if text_len <= max_chars {
        return text.to_string();
    }

    let suffix_len = suffix.chars().count();
    if max_chars <= suffix_len {
        return suffix.chars().take(max_chars).collect();
    }

    let keep = max_chars - suffix_len;
    let mut truncated: String = text.chars().take(keep).collect();
    truncated.push_str(suffix);
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_chars_preserves_short_text() {
        assert_eq!(truncate_chars("lookup", 10, "..."), "lookup");
    }

    #[test]
    fn truncate_chars_adds_suffix() {
        assert_eq!(truncate_chars("containers", 6, "..."), "con...");
    }

    #[test]
    fn truncate_chars_handles_unicode_boundaries() {
        assert_eq!(truncate_chars("λx. café", 6, "..."), "λx....");
    }

    #[test]
    fn truncate_chars_handles_tiny_limit() {
        assert_eq!(truncate_chars("containers", 2, "..."), "..");
    }
}
