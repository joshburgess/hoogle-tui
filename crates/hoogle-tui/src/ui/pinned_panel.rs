use hoogle_core::models::SearchResult;
use hoogle_syntax::theme::{SemanticToken, Theme};
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

pub struct PinnedState {
    pub pins: Vec<SearchResult>,
    pub scroll_offset: usize,
    pub viewport_height: usize,
}

#[allow(dead_code)]
impl PinnedState {
    pub fn new() -> Self {
        Self {
            pins: Vec::new(),
            scroll_offset: 0,
            viewport_height: 0,
        }
    }

    pub fn pin(&mut self, result: &SearchResult) {
        // Don't duplicate
        if self
            .pins
            .iter()
            .any(|p| p.name == result.name && p.module == result.module)
        {
            return;
        }
        self.pins.push(result.clone());
    }

    pub fn unpin(&mut self, index: usize) {
        if index < self.pins.len() {
            self.pins.remove(index);
        }
    }

    pub fn clear(&mut self) {
        self.pins.clear();
        self.scroll_offset = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.pins.is_empty()
    }

    pub fn scroll_down(&mut self, n: usize) {
        let total = self.total_lines();
        let max = total.saturating_sub(self.viewport_height);
        self.scroll_offset = (self.scroll_offset + n).min(max);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    fn total_lines(&self) -> usize {
        // 2 lines per pin (sig + module), plus separators
        if self.pins.is_empty() {
            1
        } else {
            self.pins.len() * 3
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoogle_core::models::{ModulePath, PackageInfo, ResultKind};

    fn make_result(name: &str, module: &str) -> SearchResult {
        SearchResult {
            name: name.to_string(),
            module: Some(ModulePath(
                module.split('.').map(|s| s.to_string()).collect(),
            )),
            package: Some(PackageInfo {
                name: "pkg".to_string(),
                version: None,
            }),
            signature: Some("Int -> Int".to_string()),
            doc_url: None,
            short_doc: None,
            result_kind: ResultKind::Function,
        }
    }

    #[test]
    fn new_is_empty() {
        let state = PinnedState::new();
        assert!(state.is_empty());
        assert_eq!(state.pins.len(), 0);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn pin_adds_result() {
        let mut state = PinnedState::new();
        let r = make_result("lookup", "Data.Map");
        state.pin(&r);
        assert_eq!(state.pins.len(), 1);
        assert!(!state.is_empty());
        assert_eq!(state.pins[0].name, "lookup");
    }

    #[test]
    fn pin_dedup_same_name_and_module() {
        let mut state = PinnedState::new();
        let r = make_result("lookup", "Data.Map");
        state.pin(&r);
        state.pin(&r);
        assert_eq!(state.pins.len(), 1);
    }

    #[test]
    fn pin_allows_different_names() {
        let mut state = PinnedState::new();
        state.pin(&make_result("lookup", "Data.Map"));
        state.pin(&make_result("insert", "Data.Map"));
        assert_eq!(state.pins.len(), 2);
    }

    #[test]
    fn pin_allows_same_name_different_module() {
        let mut state = PinnedState::new();
        state.pin(&make_result("lookup", "Data.Map.Strict"));
        state.pin(&make_result("lookup", "Data.Map.Lazy"));
        assert_eq!(state.pins.len(), 2);
    }

    #[test]
    fn unpin_removes_by_index() {
        let mut state = PinnedState::new();
        state.pin(&make_result("a", "M.A"));
        state.pin(&make_result("b", "M.B"));
        state.pin(&make_result("c", "M.C"));
        state.unpin(1);
        assert_eq!(state.pins.len(), 2);
        assert_eq!(state.pins[0].name, "a");
        assert_eq!(state.pins[1].name, "c");
    }

    #[test]
    fn unpin_out_of_bounds_no_panic() {
        let mut state = PinnedState::new();
        state.pin(&make_result("a", "M.A"));
        state.unpin(5);
        assert_eq!(state.pins.len(), 1);
    }

    #[test]
    fn unpin_empty_no_panic() {
        let mut state = PinnedState::new();
        state.unpin(0);
        assert!(state.is_empty());
    }

    #[test]
    fn clear_removes_all_and_resets_scroll() {
        let mut state = PinnedState::new();
        state.pin(&make_result("a", "M.A"));
        state.pin(&make_result("b", "M.B"));
        state.scroll_offset = 5;
        state.clear();
        assert!(state.is_empty());
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn is_empty_reflects_state() {
        let mut state = PinnedState::new();
        assert!(state.is_empty());
        state.pin(&make_result("a", "M.A"));
        assert!(!state.is_empty());
        state.clear();
        assert!(state.is_empty());
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &mut PinnedState, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Pinned ({}) ", state.pins.len()))
        .title_bottom(Span::styled(
            " Ctrl-x:unpin all ",
            theme.style(SemanticToken::Comment),
        ))
        .border_style(theme.style(SemanticToken::Border));

    let inner = block.inner(area);
    state.viewport_height = inner.height as usize;
    frame.render_widget(block, area);

    if state.pins.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  Pin results with 'P' to compare",
            theme.style(SemanticToken::Comment),
        )));
        frame.render_widget(empty, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for (i, pin) in state.pins.iter().enumerate() {
        if i > 0 {
            lines.push(Line::from(Span::styled(
                "\u{2500}".repeat(inner.width as usize),
                theme.style(SemanticToken::Border),
            )));
        }

        // Signature line
        if let Some(ref sig) = pin.signature {
            let full = format!("{} :: {sig}", pin.name);
            let highlighted = hoogle_syntax::highlight_signature(&full, theme);
            lines.push(highlighted);
        } else {
            lines.push(Line::from(Span::styled(
                pin.name.as_str(),
                theme
                    .style(SemanticToken::TypeConstructor)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        // Module line
        let module = pin
            .module
            .as_ref()
            .map(|m| m.to_string())
            .unwrap_or_default();
        let pkg = pin.package.as_ref().map(|p| p.name.as_str()).unwrap_or("");
        lines.push(Line::from(vec![
            Span::styled("  ", theme.style(SemanticToken::DocText)),
            Span::styled(module, theme.style(SemanticToken::ModuleName)),
            Span::styled("  ", theme.style(SemanticToken::DocText)),
            Span::styled(pkg, theme.style(SemanticToken::PackageName)),
        ]));
    }

    let total = lines.len();
    let max_scroll = total.saturating_sub(state.viewport_height);
    state.scroll_offset = state.scroll_offset.min(max_scroll);

    let paragraph = Paragraph::new(lines).scroll((state.scroll_offset as u16, 0));
    frame.render_widget(paragraph, inner);

    if total > state.viewport_height {
        let mut sb_state = ScrollbarState::new(max_scroll).position(state.scroll_offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area,
            &mut sb_state,
        );
    }
}
