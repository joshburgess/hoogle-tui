use crate::actions::Action;
use crate::app::{App, AppMode, PopupMode};
use crate::ui::{bookmarks_popup, history_popup};

impl App {
    pub(crate) fn handle_action(&mut self, action: Action) {
        if action == Action::Quit {
            self.should_quit = true;
            return;
        }

        if let Some(popup) = self.popup {
            self.handle_popup_action(popup, action);
            return;
        }

        match action {
            Action::Quit => self.should_quit = true,
            Action::Back => self.handle_back(),
            Action::FocusSearch => self.switch_mode(AppMode::Search),
            Action::FocusResults if !self.results.items.is_empty() => {
                self.switch_mode(AppMode::Results)
            }
            Action::MoveDown => self.move_selection_or_scroll(1),
            Action::MoveUp => self.move_selection_or_scroll(-1),
            Action::MoveToTop => self.move_selection_or_scroll_to_edge(false),
            Action::MoveToBottom => self.move_selection_or_scroll_to_edge(true),
            Action::Select if self.mode == AppMode::Results => self.open_doc_for_selected(),
            Action::ScrollDown => self.scroll_active_view(1),
            Action::ScrollUp => self.scroll_active_view(-1),
            Action::ScrollHalfDown => self.scroll_active_view_by_fraction(2, true),
            Action::ScrollHalfUp => self.scroll_active_view_by_fraction(2, false),
            Action::ScrollPageDown => self.scroll_active_view_page(true),
            Action::ScrollPageUp => self.scroll_active_view_page(false),
            Action::NextDeclaration => self.move_doc_declaration_or_match(true),
            Action::PrevDeclaration => self.move_doc_declaration_or_match(false),
            Action::OpenTOC if self.mode == AppMode::DocView => self.open_toc(),
            Action::FollowLink if self.mode == AppMode::DocView => self.follow_doc_link(),
            Action::CycleLink if self.mode == AppMode::DocView => self.doc_state.focus_next_link(),
            Action::SearchInDoc if self.mode == AppMode::DocView => self.doc_state.start_search(),
            Action::NavBack if self.mode == AppMode::DocView => self.navigate_doc_back(),
            Action::ViewSource if self.mode == AppMode::DocView => {
                self.open_source_for_current_decl()
            }
            Action::TogglePreview => {
                self.preview_enabled = !self.preview_enabled;
            }
            Action::OpenFilter => {
                self.filter_state.sync_selection();
                self.popup = Some(PopupMode::Filter);
            }
            Action::OpenSort => {
                self.sort_state.sync_selection();
                self.popup = Some(PopupMode::Sort);
            }
            Action::Bookmark => self.bookmark_selected(),
            Action::OpenBookmarks => {
                self.bookmarks_popup = Some(bookmarks_popup::BookmarksPopupState::new());
                self.popup = Some(PopupMode::Bookmarks);
            }
            Action::SearchHistory => {
                let total = self.history.entries().len();
                self.history_popup = Some(history_popup::HistoryPopupState::new(total));
                self.popup = Some(PopupMode::History);
            }
            Action::YankSignature => self.yank_signature(),
            Action::YankImport => self.yank_import(),
            Action::YankUrl => self.yank_url(),
            Action::ToggleHelp => {
                if self.mode == AppMode::Help {
                    self.close_help();
                } else {
                    self.open_help();
                }
            }
            Action::ClearSearch => self.clear_search_state(),
            Action::OpenYankMenu if self.mode == AppMode::Results => self.open_yank_menu(),
            Action::OpenPackageScope => self.open_package_scope_popup(),
            Action::OpenThemeSwitcher => self.toggle_theme_switcher(),
            Action::ToggleCompact if self.mode == AppMode::Results => self.toggle_compact_results(),
            Action::OpenInBrowser => {
                self.open_in_browser();
            }
            Action::ExportSession => self.export_session(),
            Action::TabComplete => {}
            Action::LoadMore => {
                if self.loading_more {
                    self.show_info("Already loading more results");
                } else if self.has_more_results {
                    self.load_more_results();
                } else {
                    self.show_info("No more results");
                }
            }
            Action::OpenModuleBrowser => self.open_module_browser(),
            Action::PinResult => self.pin_selected_result(),
            Action::UnpinAll => self.clear_pinned_results(),
            Action::ToggleMultiSelect if self.mode == AppMode::Results => {
                self.toggle_multi_select_current()
            }
            Action::YankSelectedImports if self.mode == AppMode::Results => {
                self.yank_multi_imports()
            }
            Action::ToggleGroupByModule if self.mode == AppMode::Results => {
                self.toggle_group_by_module()
            }
            Action::YankGhciType => {
                self.yank_ghci_command(":type");
            }
            Action::YankGhciInfo => {
                self.yank_ghci_command(":info");
            }
            Action::YankDeclLink => {
                self.yank_decl_deep_link();
            }
            Action::DetectProject => {
                self.detect_and_apply_project();
            }
            Action::Tick => self.on_tick(),
            _ => {}
        }
    }
}
