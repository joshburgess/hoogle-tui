use crate::actions::Action;
use crate::app::{App, AppMode, PopupMode};
use crate::ui::{bookmarks_popup, history_popup};

impl App {
    pub(crate) fn open_bookmarks_popup(&mut self) {
        if self.bookmark_store.bookmarks().is_empty() {
            self.show_info("No bookmarks saved");
            return;
        }
        self.bookmarks_popup = Some(bookmarks_popup::BookmarksPopupState::new(
            self.bookmark_store.bookmarks().len(),
        ));
        self.popup = Some(PopupMode::Bookmarks);
    }

    pub(crate) fn open_history_popup(&mut self) {
        let total = self.history.entries().len();
        if total == 0 {
            self.show_info("No search history");
            return;
        }
        self.history_popup = Some(history_popup::HistoryPopupState::new(total));
        self.popup = Some(PopupMode::History);
    }

    pub(crate) fn add_toc_filter_char(&mut self, c: char) {
        if let Some(ref mut toc) = self.toc_state {
            toc.add_filter_char(c);
        }
    }

    pub(crate) fn delete_toc_filter_char(&mut self) {
        if let Some(ref mut toc) = self.toc_state {
            toc.delete_filter_char();
        }
    }

    pub(crate) fn add_history_filter_char(&mut self, c: char) {
        if let Some(ref mut hp) = self.history_popup {
            hp.filter.push(c);
            hp.update_filter(&self.history);
        }
    }

    pub(crate) fn delete_history_filter_char(&mut self) {
        if let Some(ref mut hp) = self.history_popup {
            hp.filter.pop();
            hp.update_filter(&self.history);
        }
    }

    pub(crate) fn add_bookmark_filter_char(&mut self, c: char) {
        if let Some(ref mut bp) = self.bookmarks_popup {
            bp.add_filter_char(c, &self.bookmark_store);
        }
    }

    pub(crate) fn delete_bookmark_filter_char(&mut self) {
        if let Some(ref mut bp) = self.bookmarks_popup {
            bp.delete_filter_char(&self.bookmark_store);
        }
    }

    pub(crate) fn handle_popup_action(&mut self, popup: PopupMode, action: Action) {
        match popup {
            PopupMode::Filter => match action {
                Action::MoveDown => self.filter_state.move_down(),
                Action::MoveUp => self.filter_state.move_up(),
                Action::Select => {
                    self.filter_state.confirm();
                    self.close_popup();
                    self.apply_filter_and_sort();
                }
                Action::Back | Action::Quit => self.close_popup(),
                Action::Tick => self.on_tick(),
                _ => {}
            },
            PopupMode::Sort => match action {
                Action::MoveDown => self.sort_state.move_down(),
                Action::MoveUp => self.sort_state.move_up(),
                Action::Select => {
                    self.sort_state.confirm();
                    self.close_popup();
                    self.apply_filter_and_sort();
                }
                Action::Back | Action::Quit => self.close_popup(),
                Action::Tick => self.on_tick(),
                _ => {}
            },
            PopupMode::Toc => match action {
                Action::MoveDown => {
                    if let Some(ref mut toc) = self.toc_state {
                        toc.move_down();
                    }
                }
                Action::MoveUp => {
                    if let Some(ref mut toc) = self.toc_state {
                        toc.move_up();
                    }
                }
                Action::Select => {
                    if let Some(ref toc) = self.toc_state {
                        if let Some(offset) = toc.selected_offset() {
                            self.doc_state.scroll_offset = offset.saturating_sub(1);
                            self.close_popup();
                            return;
                        }
                    }
                    self.close_popup();
                    self.show_info("No declaration selected");
                }
                Action::Back | Action::Quit => self.close_popup(),
                Action::Tick => self.on_tick(),
                _ => {}
            },
            PopupMode::History => match action {
                Action::MoveDown => {
                    if let Some(ref mut hp) = self.history_popup {
                        hp.move_down();
                    }
                }
                Action::MoveUp => {
                    if let Some(ref mut hp) = self.history_popup {
                        hp.move_up();
                    }
                }
                Action::Select => {
                    if let Some(ref hp) = self.history_popup {
                        if let Some(idx) = hp.selected_index() {
                            if let Some(entry) = self.history.entries().get(idx) {
                                let query = entry.query.clone();
                                self.close_popup();
                                self.set_initial_query(&query);
                                return;
                            }
                        }
                    }
                    self.close_popup();
                    self.show_info("No history entry selected");
                }
                Action::DeleteEntry => {
                    let selected = self
                        .history_popup
                        .as_ref()
                        .and_then(|hp| hp.selected_index());
                    let Some(idx) = selected else {
                        self.show_info("No history entry selected");
                        return;
                    };
                    self.history.remove(idx);
                    if let Err(e) = self.history.try_save() {
                        self.show_error(&format!("Failed to save history: {e}"));
                    }
                    if let Some(ref mut hp) = self.history_popup {
                        hp.update_filter_preserving_selection(&self.history);
                    } else {
                        let total = self.history.entries().len();
                        self.history_popup = Some(history_popup::HistoryPopupState::new(total));
                    }
                }
                Action::Back | Action::Quit => self.close_popup(),
                Action::Tick => self.on_tick(),
                _ => {}
            },
            PopupMode::Bookmarks => match action {
                Action::MoveDown => {
                    if let Some(ref mut bp) = self.bookmarks_popup {
                        bp.move_down();
                    }
                }
                Action::MoveUp => {
                    if let Some(ref mut bp) = self.bookmarks_popup {
                        bp.move_up();
                    }
                }
                Action::Select => {
                    if let Some(ref bp) = self.bookmarks_popup {
                        if let Some(idx) = bp.selected_index() {
                            if let Some(bm) = self.bookmark_store.bookmarks().get(idx) {
                                if let Some(ref url) = bm.doc_url {
                                    let url = url.clone();
                                    self.close_popup();
                                    self.switch_mode(AppMode::DocView);
                                    self.doc_state.loading = true;
                                    self.doc_state.current_url = None;
                                    self.doc_state.nav_stack.clear();
                                    self.fetch_doc(url);
                                    return;
                                }
                                self.close_popup();
                                self.show_info("No URL available");
                                return;
                            }
                        }
                    }
                    self.close_popup();
                }
                Action::DeleteEntry => {
                    if let Some(selected) = self
                        .bookmarks_popup
                        .as_ref()
                        .and_then(|bp| bp.selected_index())
                    {
                        if selected >= self.bookmark_store.bookmarks().len() {
                            self.show_info("No bookmark selected");
                            return;
                        }
                        self.bookmark_store.remove(selected);
                        if let Err(e) = self.bookmark_store.try_save() {
                            self.show_error(&format!("Failed to save bookmarks: {e}"));
                        }
                        if let Some(ref mut bp) = self.bookmarks_popup {
                            bp.update_filter_preserving_selection(&self.bookmark_store);
                        }
                    } else {
                        self.show_info("No bookmark selected");
                    }
                }
                Action::Back | Action::Quit => self.close_popup(),
                Action::Tick => self.on_tick(),
                _ => {}
            },
            PopupMode::YankMenu => match action {
                Action::MoveDown => {
                    if let Some(ref mut yp) = self.yank_popup {
                        yp.move_down();
                    }
                }
                Action::MoveUp => {
                    if let Some(ref mut yp) = self.yank_popup {
                        yp.move_up();
                    }
                }
                Action::Select => {
                    if let Some(ref yp) = self.yank_popup {
                        let idx = yp.selected;
                        self.close_popup();
                        match idx {
                            0 => self.yank_signature(),
                            1 => self.yank_qualified_name(),
                            2 => self.yank_import(),
                            3 => self.yank_url(),
                            4 => self.yank_ghci_command(":type"),
                            5 => self.yank_ghci_command(":info"),
                            6 => self.yank_decl_deep_link(),
                            _ => {}
                        }
                        return;
                    }
                    self.close_popup();
                }
                Action::Back | Action::Quit => self.close_popup(),
                Action::Tick => self.on_tick(),
                _ => {}
            },
            PopupMode::PackageScope => match action {
                Action::Select => {
                    if let Some(ref mut pp) = self.package_popup {
                        self.package_scope = pp.confirm();
                        self.project_scope_enabled = true;
                        self.status.package_scope = self.package_scope.clone();
                    }
                    self.close_popup();
                    if self.query_text().is_empty() {
                        self.show_info("No query to search with package scope");
                    } else {
                        self.trigger_search();
                    }
                }
                Action::ClearSearch => {
                    if let Some(ref mut pp) = self.package_popup {
                        pp.clear();
                    }
                }
                Action::Back | Action::Quit => self.close_popup(),
                Action::Tick => self.on_tick(),
                _ => {}
            },
            PopupMode::ThemeSwitcher => match action {
                Action::MoveDown => {
                    if let Some(ref mut tp) = self.theme_popup {
                        tp.move_down();
                    }
                }
                Action::MoveUp => {
                    if let Some(ref mut tp) = self.theme_popup {
                        tp.move_up();
                    }
                }
                Action::Select => {
                    if let Some(ref mut tp) = self.theme_popup {
                        let name = tp.confirm();
                        self.theme = hoogle_syntax::theme::Theme::by_name(name);
                        self.doc_state
                            .rerender_current_doc(&self.theme, self.last_width);
                    }
                    self.close_popup();
                }
                Action::Back | Action::Quit | Action::OpenThemeSwitcher => self.close_popup(),
                Action::Tick => self.on_tick(),
                _ => {}
            },
            PopupMode::ModuleBrowser => match action {
                Action::MoveDown => {
                    if let Some(ref mut mb) = self.module_browser {
                        mb.move_down();
                    }
                }
                Action::MoveUp => {
                    if let Some(ref mut mb) = self.module_browser {
                        mb.move_up();
                    }
                }
                Action::ScrollDown => {
                    if let Some(ref mut mb) = self.module_browser {
                        mb.toggle_expand();
                    }
                }
                Action::Select => {
                    if let Some(ref mb) = self.module_browser {
                        if let Some(module) = mb.selected_module() {
                            let query = format!("module:{module}");
                            self.close_popup();
                            self.set_initial_query(&query);
                            return;
                        }
                    }
                    self.close_popup();
                    self.show_info("No module selected");
                }
                Action::Back | Action::Quit => self.close_popup(),
                Action::Tick => self.on_tick(),
                _ => {}
            },
            PopupMode::CommandPalette => match action {
                Action::MoveDown => {
                    if let Some(ref mut cp) = self.command_palette {
                        cp.move_down();
                    }
                }
                Action::MoveUp => {
                    if let Some(ref mut cp) = self.command_palette {
                        cp.move_up();
                    }
                }
                Action::Select => {
                    let selected = self
                        .command_palette
                        .as_ref()
                        .and_then(|cp| cp.selected_action());
                    self.close_popup();
                    if let Some(action) = selected {
                        self.handle_action(action);
                    } else {
                        self.show_info("No command selected");
                    }
                }
                Action::Back | Action::Quit => self.close_popup(),
                Action::Tick => self.on_tick(),
                _ => {}
            },
        }
    }

    pub(crate) fn close_popup(&mut self) {
        match self.popup {
            Some(PopupMode::Toc) => self.toc_state = None,
            Some(PopupMode::History) => self.history_popup = None,
            Some(PopupMode::Bookmarks) => self.bookmarks_popup = None,
            Some(PopupMode::YankMenu) => self.yank_popup = None,
            Some(PopupMode::PackageScope) => self.package_popup = None,
            Some(PopupMode::ThemeSwitcher) => self.theme_popup = None,
            Some(PopupMode::ModuleBrowser) => self.module_browser = None,
            Some(PopupMode::CommandPalette) => self.command_palette = None,
            Some(PopupMode::Filter | PopupMode::Sort) | None => {}
        }
        self.popup = None;
    }
}
