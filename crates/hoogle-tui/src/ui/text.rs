use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use ratatui::text::{Line, Span};

pub fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

pub fn spans_width<'a>(spans: impl IntoIterator<Item = &'a ratatui::text::Span<'a>>) -> usize {
    spans
        .into_iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum()
}

pub fn line_plain_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

pub fn lower_line_text(line: &Line<'_>) -> String {
    line_plain_text(line).to_lowercase()
}

pub fn normalized_filter(text: &str) -> String {
    text.trim().to_lowercase()
}

pub fn pad_to_width(text: &str, width: usize) -> String {
    let current = display_width(text);
    if current >= width {
        return text.to_string();
    }

    format!("{text}{}", " ".repeat(width - current))
}

pub fn truncate_width(text: &str, max_width: usize, suffix: &str) -> String {
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let suffix_width = display_width(suffix);
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

pub fn truncate_line(line: &Line<'static>, width: usize) -> Line<'static> {
    let mut remaining = width;
    let mut spans = Vec::new();

    for span in &line.spans {
        if remaining == 0 {
            break;
        }

        let text = span.content.as_ref();
        let span_width = display_width(text);
        if span_width <= remaining {
            spans.push(span.clone());
            remaining -= span_width;
            continue;
        }

        let clipped = truncate_width(text, remaining, "\u{2026}");
        spans.push(Span::styled(clipped, span.style));
        break;
    }

    Line::from(spans)
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

    #[test]
    fn display_width_counts_wide_characters() {
        assert_eq!(display_width("型a"), 3);
    }

    #[test]
    fn spans_width_counts_display_width() {
        let spans = [
            ratatui::text::Span::raw("型"),
            ratatui::text::Span::raw("ab"),
        ];
        assert_eq!(spans_width(spans.iter()), 4);
    }

    #[test]
    fn line_plain_text_flattens_spans() {
        let line = ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Data."),
            ratatui::text::Span::raw("Map"),
        ]);

        assert_eq!(line_plain_text(&line), "Data.Map");
    }

    #[test]
    fn lower_line_text_flattens_and_lowers_spans() {
        let line = ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("Functor"),
            ratatui::text::Span::raw(" MAP"),
        ]);

        assert_eq!(lower_line_text(&line), "functor map");
    }

    #[test]
    fn normalized_filter_trims_and_lowers() {
        assert_eq!(normalized_filter("  Data.Map  "), "data.map");
    }

    #[test]
    fn truncate_line_preserves_styles_across_spans() {
        let style = ratatui::style::Style::default().fg(ratatui::style::Color::Red);
        let line = ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("abc"),
            ratatui::text::Span::styled("defgh", style),
        ]);

        let truncated = truncate_line(&line, 6);

        assert_eq!(truncated.spans.len(), 2);
        assert_eq!(truncated.spans[0].content.as_ref(), "abc");
        assert_eq!(truncated.spans[1].content.as_ref(), "de\u{2026}");
        assert_eq!(truncated.spans[1].style, style);
    }

    #[test]
    fn pad_to_width_accounts_for_wide_characters() {
        assert_eq!(pad_to_width("型", 4), "型  ");
    }

    #[test]
    fn pad_to_width_preserves_already_wide_text() {
        assert_eq!(pad_to_width("abcdef", 3), "abcdef");
    }
}
