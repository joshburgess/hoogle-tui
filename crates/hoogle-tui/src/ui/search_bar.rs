use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::Span,
    widgets::{Block, Borders},
    Frame,
};
use tui_textarea::TextArea;

use crate::app::AppMode;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    textarea: &mut TextArea,
    mode: AppMode,
    has_query: bool,
    theme: &Theme,
) {
    let focused = mode == AppMode::Search;

    let border_style = if focused {
        theme
            .style(SemanticToken::ModuleName)
            .add_modifier(Modifier::BOLD)
    } else {
        theme.style(SemanticToken::Border)
    };

    let title = if focused {
        " \u{1f50d} Search Hoogle "
    } else {
        " Search (/ to focus) "
    };

    // Bottom hint: context-sensitive
    let bottom_hint = if focused {
        " Enter:results \u{2502} Tab:complete \u{2502} Esc:back \u{2502} F1:help "
    } else if !has_query {
        // Syntax cheatsheet when empty and unfocused
        " name \u{2502} :: a -> b \u{2502} +pkg name \u{2502} module:Data.Map "
    } else {
        " /:focus \u{2502} ?:help "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(Span::styled(
            bottom_hint,
            theme.style(SemanticToken::Comment),
        ))
        .border_style(border_style);

    textarea.set_block(block);
    textarea.set_cursor_line_style(ratatui::style::Style::default());

    if focused {
        textarea.set_style(theme.style(SemanticToken::SearchInput));
    } else {
        textarea.set_style(theme.style(SemanticToken::Border));
    }

    frame.render_widget(&*textarea, area);
}
