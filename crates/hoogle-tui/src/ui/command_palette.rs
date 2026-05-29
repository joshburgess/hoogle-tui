use crate::actions::Action;
use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::popup_layout::centered_popup;
use super::text::{display_width, normalized_filter, truncate_width};

#[derive(Debug, Clone)]
pub struct CommandEntry {
    pub group: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct CommandPaletteState {
    pub filter: String,
    pub selected: usize,
    pub entries: Vec<CommandEntry>,
    pub filtered_indices: Vec<usize>,
}

impl CommandPaletteState {
    pub fn new(entries: Vec<CommandEntry>) -> Self {
        let filtered_indices = (0..entries.len()).collect();
        Self {
            filter: String::new(),
            selected: 0,
            entries,
            filtered_indices,
        }
    }

    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected < self.filtered_indices.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn add_filter_char(&mut self, c: char) {
        self.filter.push(c);
        self.apply_filter();
    }

    pub fn delete_filter_char(&mut self) {
        self.filter.pop();
        self.apply_filter();
    }

    pub fn selected_action(&self) -> Option<Action> {
        let idx = *self.filtered_indices.get(self.selected)?;
        Some(self.entries[idx].action.clone())
    }

    pub fn visible_count(&self) -> usize {
        self.filtered_indices.len()
    }

    fn apply_filter(&mut self) {
        let query = normalized_filter(&self.filter);
        let mut scored: Vec<(usize, usize)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if query.is_empty() {
                    Some((0, index))
                } else {
                    match_score(entry, &query).map(|score| (score, index))
                }
            })
            .collect();
        scored.sort_by_key(|(score, index)| (*score, *index));
        self.filtered_indices = scored.into_iter().map(|(_, index)| index).collect();
        self.selected = 0;
    }
}

fn match_score(entry: &CommandEntry, query: &str) -> Option<usize> {
    let group = entry.group.to_lowercase();
    let label = entry.label.to_lowercase();
    let hint = entry.hint.to_lowercase();
    let combined = format!("{label} {hint} {group}");

    if label.starts_with(query) {
        Some(0)
    } else if label.split_whitespace().any(|word| word.starts_with(query)) {
        Some(1)
    } else if group.starts_with(query) {
        Some(2)
    } else if hint.starts_with(query) {
        Some(3)
    } else if label.contains(query) {
        Some(4)
    } else if group.contains(query) {
        Some(5)
    } else if hint.contains(query) {
        Some(6)
    } else if query.split_whitespace().all(|part| combined.contains(part)) {
        Some(7)
    } else {
        None
    }
}

pub fn render(frame: &mut Frame, state: &CommandPaletteState, theme: &Theme) {
    let area = frame.area();
    let popup_width = (area.width * 3 / 4).min(72);
    let popup_height = (area.height * 3 / 4).min(18);
    let popup = centered_popup(area, popup_width, popup_height);
    frame.render_widget(Clear, popup);

    let count = state.visible_count();
    let display_filter = state.filter.trim();
    let raw_title = if display_filter.is_empty() {
        format!(" Commands ({count}) ")
    } else {
        format!(" Commands: {display_filter} ({count}) ")
    };
    let title_width = popup.width.saturating_sub(2) as usize;
    let title = truncate_width(&raw_title, title_width, "...");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.style(SemanticToken::Border));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let max_visible = inner.height as usize;
    let scroll = if state.selected >= max_visible {
        state.selected - max_visible + 1
    } else {
        0
    };

    let mut lines = Vec::new();
    for (visible_idx, &idx) in state
        .filtered_indices
        .iter()
        .enumerate()
        .skip(scroll)
        .take(max_visible)
    {
        let entry = &state.entries[idx];
        let selected = visible_idx == state.selected;
        let style = if selected {
            theme
                .style(SemanticToken::Selected)
                .add_modifier(Modifier::BOLD)
        } else {
            theme.style(SemanticToken::DocText)
        };
        let hint_style = if selected {
            theme.style(SemanticToken::Selected)
        } else {
            theme.style(SemanticToken::Comment)
        };
        let group_style = if selected {
            theme.style(SemanticToken::Selected)
        } else {
            theme.style(SemanticToken::ModuleName)
        };
        let max_width = inner.width as usize;
        let label_width = max_width.saturating_sub(2);
        let hint_width = label_width
            .saturating_sub(display_width(entry.label))
            .saturating_sub(2);
        let group_width = hint_width
            .saturating_sub(display_width(entry.hint))
            .saturating_sub(2);
        let label = truncate_width(entry.label, label_width, "...");
        let hint = if group_width > 0 {
            truncate_width(entry.hint, hint_width, "...")
        } else {
            String::new()
        };
        let group = truncate_width(entry.group, group_width, "...");

        let mut spans = vec![
            Span::styled(if selected { "> " } else { "  " }, style),
            Span::styled(label, style),
        ];
        if !hint.is_empty() {
            spans.push(Span::styled("  ", style));
            spans.push(Span::styled(hint, hint_style));
        }
        if !group.is_empty() {
            spans.push(Span::styled("  ", style));
            spans.push(Span::styled(group, group_style));
        }
        lines.push(Line::from(spans));
    }

    if state.filtered_indices.is_empty() {
        let message = truncate_width("  No commands found.", inner.width as usize, "...");
        lines.push(Line::from(Span::styled(
            message,
            theme.style(SemanticToken::Comment),
        )));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> CommandPaletteState {
        CommandPaletteState::new(vec![
            CommandEntry {
                group: "Docs",
                label: "Open docs",
                hint: "Enter",
                action: Action::Select,
            },
            CommandEntry {
                group: "Pins",
                label: "Copy pinned imports",
                hint: "pins",
                action: Action::YankPinnedImports,
            },
        ])
    }

    fn type_filter(state: &mut CommandPaletteState, query: &str) {
        for c in query.chars() {
            state.add_filter_char(c);
        }
    }

    #[test]
    fn filter_matches_labels_and_hints() {
        let mut state = state();
        type_filter(&mut state, "pin");
        assert_eq!(state.filtered_indices, vec![1]);
        assert_eq!(state.selected_action(), Some(Action::YankPinnedImports));
    }

    #[test]
    fn filter_matches_groups() {
        let mut state = state();
        type_filter(&mut state, "doc");
        assert_eq!(state.filtered_indices, vec![0]);
    }

    #[test]
    fn movement_clamps() {
        let mut state = state();
        state.move_down();
        state.move_down();
        assert_eq!(state.selected, 1);
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn visible_count_tracks_filter() {
        let mut state = state();
        assert_eq!(state.visible_count(), 2);
        type_filter(&mut state, "doc");
        assert_eq!(state.visible_count(), 1);
    }

    #[test]
    fn stronger_matches_sort_first() {
        let mut state = CommandPaletteState::new(vec![
            CommandEntry {
                group: "Global",
                label: "Show help",
                hint: "F1",
                action: Action::ToggleHelp,
            },
            CommandEntry {
                group: "Pins",
                label: "Copy pinned imports",
                hint: "pins",
                action: Action::YankPinnedImports,
            },
        ]);

        state.add_filter_char('p');

        assert_eq!(state.selected_action(), Some(Action::YankPinnedImports));
    }

    #[test]
    fn multi_word_filter_matches_across_fields() {
        let mut state = CommandPaletteState::new(vec![CommandEntry {
            group: "Project",
            label: "Toggle project scope",
            hint: "project packages",
            action: Action::ToggleProjectScope,
        }]);

        type_filter(&mut state, "scope toggle");

        assert_eq!(state.selected_action(), Some(Action::ToggleProjectScope));
    }

    #[test]
    fn filter_ignores_surrounding_whitespace() {
        let mut state = state();

        type_filter(&mut state, "  pins  ");

        assert_eq!(state.selected_action(), Some(Action::YankPinnedImports));
    }
}
