use hoogle_core::models::SearchResult;
use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, result: Option<&SearchResult>, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Preview ")
        .border_style(theme.style(SemanticToken::Border));

    let Some(result) = result else {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  Select a result to preview",
            theme.style(SemanticToken::Comment),
        )))
        .block(block);
        frame.render_widget(empty, area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    // Module + package header
    let module_str = result
        .module
        .as_ref()
        .map(|m| m.to_string())
        .unwrap_or_default();
    let pkg_str = result
        .package
        .as_ref()
        .map(|p| p.to_string())
        .unwrap_or_default();

    if !module_str.is_empty() || !pkg_str.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(&module_str, theme.style(SemanticToken::ModuleName)),
            Span::styled("  ", theme.style(SemanticToken::DocText)),
            Span::styled(&pkg_str, theme.style(SemanticToken::PackageName)),
        ]));
        lines.push(Line::from(""));
    }

    // Type signature (syntax-highlighted)
    if let Some(ref sig) = result.signature {
        let full_sig = format!("{} :: {sig}", result.name);
        let highlighted = hoogle_syntax::highlight_signature(&full_sig, theme);
        lines.push(highlighted);
        lines.push(Line::from(""));
    } else {
        lines.push(Line::from(Span::styled(
            &result.name,
            theme
                .style(SemanticToken::TypeConstructor)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }

    // Separator
    let inner_width = area.width.saturating_sub(2) as usize;
    lines.push(Line::from(Span::styled(
        "\u{2500}".repeat(inner_width),
        theme.style(SemanticToken::Border),
    )));
    lines.push(Line::from(""));

    // Kind
    lines.push(Line::from(vec![
        Span::styled("Kind: ", theme.style(SemanticToken::Comment)),
        Span::styled(
            result.result_kind.to_string(),
            theme.style(SemanticToken::Keyword),
        ),
    ]));
    lines.push(Line::from(""));

    // Documentation
    if let Some(ref doc) = result.short_doc {
        // Word-wrap manually by splitting into lines
        for line in wrap_text(doc, inner_width) {
            lines.push(Line::from(Span::styled(
                line,
                theme.style(SemanticToken::DocText),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No documentation available.",
            theme.style(SemanticToken::Comment),
        )));
    }

    // URL
    if let Some(ref url) = result.doc_url {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            url.as_str().to_string(),
            theme.style(SemanticToken::DocLink),
        )));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
