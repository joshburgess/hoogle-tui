use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::popup_layout::centered_popup;
use super::text::{display_width, truncate_width};
use crate::bookmarks::BookmarkStore;

pub struct BookmarksPopupState {
    pub filter: String,
    pub selected: usize,
    pub filtered_indices: Vec<usize>,
}

impl BookmarksPopupState {
    pub fn new(total: usize) -> Self {
        Self {
            filter: String::new(),
            selected: 0,
            filtered_indices: (0..total).collect(),
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

    pub fn selected_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.selected).copied()
    }

    pub fn add_filter_char(&mut self, c: char, store: &BookmarkStore) {
        self.filter.push(c);
        self.update_filter(store);
    }

    pub fn delete_filter_char(&mut self, store: &BookmarkStore) {
        self.filter.pop();
        self.update_filter(store);
    }

    pub fn update_filter(&mut self, store: &BookmarkStore) {
        self.update_filter_indices(store);
        self.selected = 0;
    }

    pub fn update_filter_preserving_selection(&mut self, store: &BookmarkStore) {
        let selected = self.selected;
        self.update_filter_indices(store);
        if self.filtered_indices.is_empty() {
            self.selected = 0;
        } else {
            self.selected = selected.min(self.filtered_indices.len() - 1);
        }
    }

    fn update_filter_indices(&mut self, store: &BookmarkStore) {
        let query = self.filter.trim().to_lowercase();
        self.filtered_indices = store
            .bookmarks()
            .iter()
            .enumerate()
            .filter(|(_, bookmark)| query.is_empty() || bookmark_matches(bookmark, &query))
            .map(|(i, _)| i)
            .collect();
    }
}

fn bookmark_matches(bookmark: &crate::bookmarks::Bookmark, query: &str) -> bool {
    bookmark.name.to_lowercase().contains(query)
        || bookmark
            .module
            .as_deref()
            .is_some_and(|module| module.to_lowercase().contains(query))
        || bookmark
            .package
            .as_deref()
            .is_some_and(|package| package.to_lowercase().contains(query))
        || bookmark
            .signature
            .as_deref()
            .is_some_and(|signature| signature.to_lowercase().contains(query))
}

pub fn render(
    frame: &mut Frame,
    state: &BookmarksPopupState,
    store: &BookmarkStore,
    theme: &Theme,
) {
    let area = frame.area();
    let popup_width = (area.width * 3 / 4).min(70);
    let popup_height = (area.height * 3 / 4).min(20);
    let popup = centered_popup(area, popup_width, popup_height);
    frame.render_widget(Clear, popup);

    let display_filter = state.filter.trim();
    let raw_title = if display_filter.is_empty() {
        " Bookmarks (d to delete) ".to_string()
    } else {
        format!(" Bookmarks: {display_filter} ")
    };
    let title = truncate_width(&raw_title, popup.width.saturating_sub(2) as usize, "...");

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

    let mut lines: Vec<Line> = Vec::new();
    let bookmarks = store.bookmarks();

    for (vi, &idx) in state
        .filtered_indices
        .iter()
        .enumerate()
        .skip(scroll)
        .take(max_visible)
    {
        let bm = &bookmarks[idx];
        let is_selected = vi == state.selected;
        let marker = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            theme
                .style(SemanticToken::Selected)
                .add_modifier(Modifier::BOLD)
        } else {
            theme.style(SemanticToken::DocText)
        };

        let raw_module = bm
            .module
            .as_ref()
            .map(|m| format!(" ({m})"))
            .unwrap_or_default();
        let raw_signature = bm
            .signature
            .as_ref()
            .map(|s| format!(" :: {s}"))
            .unwrap_or_default();

        let row_width = inner.width as usize;
        let content_width = row_width.saturating_sub(display_width(marker));
        let name_budget = content_width / 3;
        let module_budget = content_width / 3;
        let name = truncate_width(&bm.name, name_budget, "...");
        let module_str = truncate_width(&raw_module, module_budget, "...");
        let sig_width = content_width
            .saturating_sub(display_width(&name))
            .saturating_sub(display_width(&module_str));
        let sig_str = truncate_width(&raw_signature, sig_width, "...");

        let meta_style = if is_selected {
            theme.style(SemanticToken::Selected)
        } else {
            theme.style(SemanticToken::Comment)
        };

        lines.push(Line::from(vec![
            Span::styled(marker.to_string(), style),
            Span::styled(name, style),
            Span::styled(module_str, meta_style),
            Span::styled(sig_str, meta_style),
        ]));
    }

    if bookmarks.is_empty() {
        let message = truncate_width(
            "  No bookmarks. Press m on a result to bookmark it.",
            inner.width as usize,
            "...",
        );
        lines.push(Line::from(Span::styled(
            message,
            theme.style(SemanticToken::Comment),
        )));
    } else if state.filtered_indices.is_empty() {
        let message = truncate_width("  No bookmarks found.", inner.width as usize, "...");
        lines.push(Line::from(Span::styled(
            message,
            theme.style(SemanticToken::Comment),
        )));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bookmarks::Bookmark;

    fn store_with_bookmarks() -> (tempfile::TempDir, BookmarkStore) {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let mut store = BookmarkStore::load_with_status(dir.path().join("bookmarks.json")).0;
        store.add(Bookmark {
            name: "lookup".to_string(),
            module: Some("Data.Map.Strict".to_string()),
            package: Some("containers".to_string()),
            signature: Some("Ord k => k -> Map k a -> Maybe a".to_string()),
            doc_url: None,
            added: chrono::Utc::now(),
        });
        store.add(Bookmark {
            name: "filter".to_string(),
            module: Some("Data.List".to_string()),
            package: Some("base".to_string()),
            signature: Some("(a -> Bool) -> [a] -> [a]".to_string()),
            doc_url: None,
            added: chrono::Utc::now(),
        });
        (dir, store)
    }

    #[test]
    fn filter_matches_name_module_package_and_signature() {
        let (_dir, store) = store_with_bookmarks();
        let mut state = BookmarksPopupState::new(store.bookmarks().len());

        for c in "map".chars() {
            state.add_filter_char(c, &store);
        }
        assert_eq!(state.filtered_indices, vec![1]);

        state.filter.clear();
        for c in "base".chars() {
            state.add_filter_char(c, &store);
        }
        assert_eq!(state.filtered_indices, vec![0]);

        state.filter.clear();
        for c in "bool".chars() {
            state.add_filter_char(c, &store);
        }
        assert_eq!(state.filtered_indices, vec![0]);
    }

    #[test]
    fn filter_ignores_surrounding_whitespace() {
        let (_dir, store) = store_with_bookmarks();
        let mut state = BookmarksPopupState::new(store.bookmarks().len());

        for c in "  containers  ".chars() {
            state.add_filter_char(c, &store);
        }

        assert_eq!(state.filtered_indices, vec![1]);
    }

    #[test]
    fn selected_index_uses_filtered_entries() {
        let (_dir, store) = store_with_bookmarks();
        let mut state = BookmarksPopupState::new(store.bookmarks().len());

        for c in "containers".chars() {
            state.add_filter_char(c, &store);
        }

        assert_eq!(state.selected_index(), Some(1));
    }
}
