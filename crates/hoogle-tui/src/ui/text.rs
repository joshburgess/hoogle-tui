use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn truncate_width(text: &str, max_width: usize, suffix: &str) -> String {
    if UnicodeWidthStr::width(text) <= max_width {
        return text.to_string();
    }

    let suffix_width = UnicodeWidthStr::width(suffix);
    if max_width <= suffix_width {
        return truncate_suffix_to_width(suffix, max_width);
    }

    let keep_width = max_width - suffix_width;
    let mut used = 0;
    let mut truncated = String::new();

    for ch in text.chars() {
        let width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + width > keep_width {
            break;
        }
        used += width;
        truncated.push(ch);
    }

    truncated.push_str(suffix);
    truncated
}

fn truncate_suffix_to_width(suffix: &str, max_width: usize) -> String {
    let mut used = 0;
    let mut truncated = String::new();

    for ch in suffix.chars() {
        let width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + width > max_width {
            break;
        }
        used += width;
        truncated.push(ch);
    }

    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_width_accounts_for_wide_characters() {
        assert_eq!(truncate_width("型型abc", 5, "..."), "型...");
    }

    #[test]
    fn truncate_width_adds_suffix() {
        assert_eq!(truncate_width("containers", 6, "..."), "con...");
    }

    #[test]
    fn truncate_width_preserves_short_display_text() {
        assert_eq!(truncate_width("λx", 2, "..."), "λx");
    }

    #[test]
    fn truncate_width_handles_tiny_limit() {
        assert_eq!(truncate_width("containers", 2, "..."), "..");
    }
}
