use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders},
    Frame,
};
use tui_textarea::TextArea;

use crate::app::AppMode;

pub fn render(frame: &mut Frame, area: Rect, textarea: &TextArea, mode: AppMode, theme: &Theme) {
    let focused = mode == AppMode::Search;

    let border_style = if focused {
        theme.style(SemanticToken::ModuleName)
    } else {
        theme.style(SemanticToken::Border)
    };

    let title = if focused {
        " Search (Esc to cancel) "
    } else {
        " Search "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    // We render the textarea with its block
    let mut ta = textarea.clone();
    ta.set_block(block);
    ta.set_cursor_line_style(ratatui::style::Style::default());

    if focused {
        ta.set_style(theme.style(SemanticToken::SearchInput));
    } else {
        ta.set_style(theme.style(SemanticToken::Border));
    }

    frame.render_widget(&ta, area);
}
