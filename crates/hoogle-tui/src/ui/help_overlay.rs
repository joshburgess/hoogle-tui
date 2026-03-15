use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

pub struct HelpState {
    pub scroll_offset: usize,
    pub viewport_height: usize,
}

impl HelpState {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            viewport_height: 0,
        }
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = HELP_LINES.len().saturating_sub(self.viewport_height);
        self.scroll_offset = (self.scroll_offset + n).min(max);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }
}

struct HelpEntry {
    key: &'static str,
    desc: &'static str,
}

struct HelpSection {
    title: &'static str,
    entries: &'static [HelpEntry],
}

const SECTIONS: &[HelpSection] = &[
    HelpSection {
        title: "Global",
        entries: &[
            HelpEntry {
                key: "Ctrl-c",
                desc: "Quit immediately",
            },
            HelpEntry {
                key: "Ctrl-l",
                desc: "Force redraw",
            },
            HelpEntry {
                key: "?",
                desc: "Toggle help overlay",
            },
        ],
    },
    HelpSection {
        title: "Search Bar",
        entries: &[
            HelpEntry {
                key: "<typing>",
                desc: "Live search (debounced)",
            },
            HelpEntry {
                key: "Enter",
                desc: "Move focus to results",
            },
            HelpEntry {
                key: "Ctrl-r",
                desc: "Open search history",
            },
            HelpEntry {
                key: "Ctrl-u",
                desc: "Clear search bar",
            },
            HelpEntry {
                key: "Esc",
                desc: "Clear search / quit if empty",
            },
        ],
    },
    HelpSection {
        title: "Result List",
        entries: &[
            HelpEntry {
                key: "j / Down",
                desc: "Move selection down",
            },
            HelpEntry {
                key: "k / Up",
                desc: "Move selection up",
            },
            HelpEntry {
                key: "g",
                desc: "Jump to first result",
            },
            HelpEntry {
                key: "G",
                desc: "Jump to last result",
            },
            HelpEntry {
                key: "Enter",
                desc: "Open Haddock docs",
            },
            HelpEntry {
                key: "Tab",
                desc: "Toggle preview pane",
            },
            HelpEntry {
                key: "/",
                desc: "Focus search bar",
            },
            HelpEntry {
                key: "f",
                desc: "Filter by result kind",
            },
            HelpEntry {
                key: "s",
                desc: "Sort results",
            },
            HelpEntry {
                key: "y",
                desc: "Yank type signature",
            },
            HelpEntry {
                key: "Y",
                desc: "Yank import statement",
            },
            HelpEntry {
                key: "Ctrl-y",
                desc: "Yank Hackage URL",
            },
            HelpEntry {
                key: "m",
                desc: "Bookmark selected result",
            },
            HelpEntry {
                key: "'",
                desc: "Open bookmarks",
            },
            HelpEntry {
                key: "q",
                desc: "Quit",
            },
        ],
    },
    HelpSection {
        title: "Doc Viewer",
        entries: &[
            HelpEntry {
                key: "j / Down",
                desc: "Scroll down one line",
            },
            HelpEntry {
                key: "k / Up",
                desc: "Scroll up one line",
            },
            HelpEntry {
                key: "d / Ctrl-d",
                desc: "Scroll down half page",
            },
            HelpEntry {
                key: "u / Ctrl-u",
                desc: "Scroll up half page",
            },
            HelpEntry {
                key: "f / Ctrl-f",
                desc: "Scroll down full page",
            },
            HelpEntry {
                key: "b / Ctrl-b",
                desc: "Scroll up full page",
            },
            HelpEntry {
                key: "g",
                desc: "Jump to top",
            },
            HelpEntry {
                key: "G",
                desc: "Jump to bottom",
            },
            HelpEntry {
                key: "o",
                desc: "Open table of contents",
            },
            HelpEntry {
                key: "n",
                desc: "Next declaration",
            },
            HelpEntry {
                key: "p",
                desc: "Previous declaration",
            },
            HelpEntry {
                key: "s",
                desc: "View source code",
            },
            HelpEntry {
                key: "Esc",
                desc: "Return to result list",
            },
        ],
    },
    HelpSection {
        title: "Source Viewer",
        entries: &[
            HelpEntry {
                key: "j / k",
                desc: "Scroll up/down",
            },
            HelpEntry {
                key: "g / G",
                desc: "Top / bottom",
            },
            HelpEntry {
                key: "Esc",
                desc: "Return to doc viewer",
            },
            HelpEntry {
                key: "y",
                desc: "Yank source to clipboard",
            },
        ],
    },
    HelpSection {
        title: "Popups (TOC, Filter, History, Bookmarks)",
        entries: &[
            HelpEntry {
                key: "j / k",
                desc: "Navigate items",
            },
            HelpEntry {
                key: "Enter",
                desc: "Select",
            },
            HelpEntry {
                key: "d",
                desc: "Delete entry (history/bookmarks)",
            },
            HelpEntry {
                key: "Esc",
                desc: "Close popup",
            },
        ],
    },
];

// Pre-compute total lines for scroll bounds
const HELP_LINES: &[(); 100] = &[(); 100]; // placeholder; actual count computed at render time

pub fn render(frame: &mut Frame, state: &mut HelpState, theme: &Theme) {
    let area = frame.area();

    let popup_width = (area.width.saturating_sub(4)).min(64);
    let popup_height = area.height.saturating_sub(4);
    let popup = centered_popup(area, popup_width, popup_height);

    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" hoogle-tui Help ")
        .title_bottom(" ? or Esc to close ")
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(popup);
    state.viewport_height = inner.height as usize;
    frame.render_widget(block, popup);

    let key_width = 16;
    let mut lines: Vec<Line> = Vec::new();

    for section in SECTIONS {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", section.title),
            theme
                .style(SemanticToken::DocHeading)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!("  {}", "\u{2500}".repeat(section.title.len() + 4)),
            theme.style(SemanticToken::Border),
        )));

        for entry in section.entries {
            let key_padded = format!("  {:<width$}", entry.key, width = key_width);
            lines.push(Line::from(vec![
                Span::styled(key_padded, theme.style(SemanticToken::ModuleName)),
                Span::styled(entry.desc.to_string(), theme.style(SemanticToken::DocText)),
            ]));
        }
    }

    lines.push(Line::from(""));

    let total = lines.len();
    let max_scroll = total.saturating_sub(state.viewport_height);
    state.scroll_offset = state.scroll_offset.min(max_scroll);

    let start = state.scroll_offset;
    let end = (start + state.viewport_height).min(total);
    let visible: Vec<Line> = lines[start..end].to_vec();

    let paragraph = Paragraph::new(visible);
    frame.render_widget(paragraph, inner);

    if total > state.viewport_height {
        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(state.scroll_offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            popup,
            &mut scrollbar_state,
        );
    }
}

fn centered_popup(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Length((area.height.saturating_sub(height)) / 2),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Length((area.width.saturating_sub(width)) / 2),
        Constraint::Length(width),
        Constraint::Min(0),
    ])
    .split(vertical[1])[1]
}
