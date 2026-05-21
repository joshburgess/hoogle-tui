use super::app_input::search_textarea_with_query;
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use hoogle_core::backend::{BackendError, HoogleBackend};
use hoogle_core::config::Config;
use hoogle_core::haddock::types::{Declaration, HaddockDoc};
use hoogle_core::models::{ModulePath, ResultKind, SearchResult};
use ratatui::layout::Rect;
use url::Url;

use crate::actions::Action;
use crate::app::{App, AppMode, DocResponse, PopupMode, SearchResponse, SourceResponse};
use crate::bookmarks::{Bookmark, BookmarkStore};
use crate::history::SearchHistory;
use crate::keymap::Keymap;
use crate::ui::status_bar::StatusMessage;
use crate::ui::{history_popup, toc_popup};

#[derive(Debug)]
struct TestBackend;

#[async_trait]
impl HoogleBackend for TestBackend {
    async fn search(
        &self,
        _query: &str,
        _offset: usize,
        _count: usize,
    ) -> Result<Vec<SearchResult>, BackendError> {
        Ok(Vec::new())
    }

    fn name(&self) -> &str {
        "test"
    }
}

fn test_app() -> App {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.cache.dir = Some(temp_dir.path().join("cache"));
    App::new(config, Box::new(TestBackend)).unwrap()
}

fn result(name: &str) -> SearchResult {
    SearchResult {
        name: name.to_string(),
        module: None,
        package: None,
        signature: None,
        doc_url: None,
        short_doc: None,
        result_kind: ResultKind::Function,
    }
}

fn module_result(name: &str, module: &[&str]) -> SearchResult {
    SearchResult {
        module: Some(ModulePath(
            module.iter().map(|part| (*part).to_string()).collect(),
        )),
        ..result(name)
    }
}

fn doc() -> HaddockDoc {
    HaddockDoc {
        module: "Data.Example".to_string(),
        package: "example-0.1.0".to_string(),
        description: Vec::new(),
        declarations: vec![
            Declaration {
                name: "first".to_string(),
                signature: Some("first :: a".to_string()),
                doc: Vec::new(),
                since: None,
                source_url: None,
                anchor: Some("v:first".to_string()),
            },
            Declaration {
                name: "Second".to_string(),
                signature: Some("data Second".to_string()),
                doc: Vec::new(),
                since: None,
                source_url: Url::parse("https://example.com/src/Data/Example.html#Second").ok(),
                anchor: Some("t:Second".to_string()),
            },
        ],
    }
}

fn app_with_doc() -> App {
    let mut app = test_app();
    app.mode = AppMode::DocView;
    app.doc_state.doc = Some(doc());
    app.doc_state.declaration_offsets = vec![("first".to_string(), 2), ("Second".to_string(), 8)];
    app
}

#[test]
fn clear_search_state_resets_query_results_and_pagination() {
    let mut app = test_app();
    app.textarea = search_textarea_with_query("map");
    app.last_searched = "map".to_string();
    let generation = app.search_generation;
    app.all_results = vec![result("map")];
    app.results.set_items(app.all_results.clone());
    app.status.result_count = 1;
    app.status.search_by_type = true;
    app.status.message = Some(StatusMessage::Loading("Searching...".to_string()));
    app.message_deadline = Some(tokio::time::Instant::now());
    app.has_more_results = true;
    app.loading_more = true;
    app.results.loading = true;
    app.completion_candidates = vec!["map".to_string()];
    app.completion_index = 1;

    app.clear_search_state();

    assert!(app.search_generation > generation);
    assert!(app.query_text().is_empty());
    assert!(app.last_searched.is_empty());
    assert!(app.all_results.is_empty());
    assert!(app.results.items.is_empty());
    assert_eq!(app.status.result_count, 0);
    assert!(!app.status.search_by_type);
    assert!(app.status.message.is_none());
    assert_eq!(app.message_deadline, None);
    assert!(!app.has_more_results);
    assert!(!app.loading_more);
    assert!(!app.results.loading);
    assert!(app.completion_candidates.is_empty());
    assert_eq!(app.completion_index, 0);
}

#[test]
fn clear_search_action_resets_query_results_and_pagination() {
    let mut app = test_app();
    app.textarea = search_textarea_with_query("map");
    app.last_searched = "map".to_string();
    app.all_results = vec![result("map")];
    app.results.set_items(app.all_results.clone());
    app.status.result_count = 1;
    app.status.search_by_type = true;
    app.status.message = Some(StatusMessage::Loading("Searching...".to_string()));
    app.message_deadline = Some(tokio::time::Instant::now());
    app.has_more_results = true;
    app.loading_more = true;
    app.results.loading = true;
    app.completion_candidates = vec!["map".to_string()];
    app.completion_index = 1;

    app.handle_action(Action::ClearSearch);

    assert!(app.query_text().is_empty());
    assert!(app.last_searched.is_empty());
    assert!(app.all_results.is_empty());
    assert!(app.results.items.is_empty());
    assert_eq!(app.status.result_count, 0);
    assert!(!app.status.search_by_type);
    assert!(app.status.message.is_none());
    assert_eq!(app.message_deadline, None);
    assert!(!app.has_more_results);
    assert!(!app.loading_more);
    assert!(!app.results.loading);
    assert!(app.completion_candidates.is_empty());
    assert_eq!(app.completion_index, 0);
}

#[tokio::test]
async fn clear_search_state_ignores_in_flight_search_response() {
    let mut app = test_app();
    app.textarea = search_textarea_with_query("map");
    app.trigger_search();
    let stale_generation = app.search_generation;

    app.clear_search_state();
    app.search_tx
        .send(SearchResponse {
            generation: stale_generation,
            append: false,
            results: Ok(vec![result("map")]),
        })
        .unwrap();
    app.on_tick();

    assert!(app.results.items.is_empty());
    assert!(app.all_results.is_empty());
    assert_eq!(app.status.result_count, 0);
}

#[test]
fn result_helpers_update_selection_state() {
    let mut app = test_app();
    app.mode = AppMode::Results;
    app.results.set_items(vec![result("a"), result("b")]);

    app.toggle_multi_select_current();

    assert!(app.results.multi_select_mode);
    assert!(app.results.multi_selected.contains(&0));
    assert_eq!(app.results.selected, 1);

    app.toggle_group_by_module();
    assert!(app.results.group_by_module);
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "Grouped by module"
    ));

    app.toggle_compact_results();
    assert!(app.results.compact);
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "Compact results enabled"
    ));
}

#[test]
fn preview_toggle_reports_new_state() {
    let mut app = test_app();
    app.preview_enabled = false;

    app.handle_action(Action::TogglePreview);

    assert!(app.preview_enabled);
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "Preview enabled"
    ));

    app.handle_action(Action::TogglePreview);

    assert!(!app.preview_enabled);
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "Preview disabled"
    ));
}

#[test]
fn multi_select_without_result_reports_noop() {
    let mut app = test_app();
    app.mode = AppMode::Results;

    app.toggle_multi_select_current();

    assert!(!app.results.multi_select_mode);
    assert!(app.results.multi_selected.is_empty());
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No result selected"
    ));
}

#[test]
fn focus_results_reports_when_results_are_empty() {
    let mut app = test_app();

    app.handle_action(Action::FocusResults);

    assert_eq!(app.mode, AppMode::Search);
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No results to focus"
    ));
}

#[test]
fn fuzzy_filter_enter_reports_when_filter_has_no_matches() {
    let mut app = test_app();
    app.mode = AppMode::Results;
    app.results.set_items(vec![result("map")]);
    app.results.start_fuzzy_filter();
    app.results.fuzzy_add_char('z');
    let keymap = Keymap::new(&Default::default());

    app.handle_fuzzy_filter_input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &keymap);

    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No filtered result selected"
    ));
}

#[test]
fn open_toc_reports_when_document_is_missing() {
    let mut app = test_app();
    app.mode = AppMode::DocView;

    app.handle_action(Action::OpenTOC);

    assert_eq!(app.popup, None);
    assert!(app.toc_state.is_none());
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No document loaded"
    ));
}

#[test]
fn doc_search_reports_when_document_is_missing() {
    let mut app = test_app();
    app.mode = AppMode::DocView;

    app.handle_action(Action::SearchInDoc);

    assert!(!app.doc_state.search_active);
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No document loaded"
    ));
}

#[test]
fn load_more_reports_unavailable_states() {
    let mut app = test_app();

    app.handle_action(Action::LoadMore);

    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No more results"
    ));

    app.loading_more = true;
    app.handle_action(Action::LoadMore);

    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "Already loading more results"
    ));

    app.loading_more = false;
    app.has_more_results = true;
    app.handle_action(Action::LoadMore);

    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No query to load more results"
    ));
}

#[test]
fn tab_complete_rebuilds_candidates_for_narrowed_query() {
    let mut app = test_app();
    app.results.set_items(vec![
        result("map"),
        result("mconcat"),
        result("mapMaybe"),
        result("map"),
    ]);
    app.textarea = search_textarea_with_query("m");
    app.tab_complete();
    assert_eq!(app.query_text(), "map");

    app.textarea = search_textarea_with_query("ma");
    app.tab_complete();

    assert_eq!(app.query_text(), "map");
    assert_eq!(app.completion_candidates, vec!["map", "mapMaybe"]);
}

#[test]
fn tab_complete_reports_unavailable_states() {
    let mut app = test_app();

    app.handle_action(Action::TabComplete);

    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No query to complete"
    ));

    app.status.message = None;
    app.textarea = search_textarea_with_query("zz");
    app.results.set_items(vec![result("map")]);

    app.handle_action(Action::TabComplete);

    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No completions available"
    ));
}

#[test]
fn appended_search_results_preserve_result_view_state() {
    let mut app = test_app();
    app.all_results = vec![result("alpha")];
    app.results.set_items(app.all_results.clone());
    app.results.start_fuzzy_filter();
    app.results.fuzzy_add_char('a');
    app.results.multi_select_mode = true;
    app.results.toggle_select_current();
    app.loading_more = true;

    app.search_tx
        .send(SearchResponse {
            generation: app.search_generation,
            append: true,
            results: Ok(vec![result("beta")]),
        })
        .unwrap();

    app.on_tick();

    assert!(!app.loading_more);
    assert_eq!(app.results.fuzzy_filter.as_deref(), Some("a"));
    assert_eq!(app.results.visible_count(), 2);
    assert!(app.results.multi_select_mode);
    assert!(app.results.multi_selected.contains(&0));
}

#[test]
fn successful_async_responses_clear_status_deadline() {
    let mut app = app_with_doc();
    let doc_url = Url::parse("https://hackage.haskell.org/package/demo/docs/Demo.html").unwrap();

    app.status.message = Some(StatusMessage::Loading("Searching...".to_string()));
    app.message_deadline = Some(tokio::time::Instant::now());
    app.loading_more = true;
    app.search_tx
        .send(SearchResponse {
            generation: app.search_generation,
            append: true,
            results: Ok(vec![result("map")]),
        })
        .unwrap();
    app.on_tick();
    assert!(app.status.message.is_none());
    assert_eq!(app.message_deadline, None);

    app.pending_doc_url = Some(doc_url.clone());
    app.status.message = Some(StatusMessage::Loading("Loading docs...".to_string()));
    app.message_deadline = Some(tokio::time::Instant::now());
    app.doc_tx
        .send(DocResponse {
            url: doc_url,
            result: Ok(doc()),
        })
        .unwrap();
    app.on_tick();
    assert!(app.status.message.is_none());
    assert_eq!(app.message_deadline, None);

    app.pending_source_decl = Some("targetDecl".to_string());
    app.status.message = Some(StatusMessage::Loading("Loading source...".to_string()));
    app.message_deadline = Some(tokio::time::Instant::now());
    app.source_tx
        .send(SourceResponse {
            decl_name: "targetDecl".to_string(),
            result: Ok("targetDecl = 1".to_string()),
        })
        .unwrap();
    app.on_tick();
    assert!(app.status.message.is_none());
    assert_eq!(app.message_deadline, None);
}

#[test]
fn popup_helpers_open_expected_popup_state() {
    let mut app = test_app();
    app.results.set_items(vec![result("map")]);

    app.open_yank_menu();
    assert_eq!(app.popup, Some(PopupMode::YankMenu));
    assert!(app.yank_popup.is_some());

    app.close_popup();
    app.open_package_scope_popup();
    assert_eq!(app.popup, Some(PopupMode::PackageScope));
    assert!(app.package_popup.is_some());

    app.close_popup();
    app.toggle_theme_switcher();
    assert_eq!(app.popup, Some(PopupMode::ThemeSwitcher));
    assert!(app.theme_popup.is_some());

    app.toggle_theme_switcher();
    assert_eq!(app.popup, None);
    assert!(app.theme_popup.is_none());
}

#[test]
fn open_yank_menu_reports_when_no_result_is_selected() {
    let mut app = test_app();
    app.mode = AppMode::Results;

    app.open_yank_menu();

    assert_eq!(app.popup, None);
    assert!(app.yank_popup.is_none());
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No result selected"
    ));
}

#[test]
fn open_module_browser_reports_empty_module_list() {
    let mut app = test_app();
    app.all_results = vec![result("map")];

    app.open_module_browser();

    assert_eq!(app.popup, None);
    assert!(app.module_browser.is_none());
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No modules available"
    ));
}

#[test]
fn open_module_browser_shows_available_modules() {
    let mut app = test_app();
    app.all_results = vec![module_result("map", &["Data", "Map"])];

    app.open_module_browser();

    assert_eq!(app.popup, Some(PopupMode::ModuleBrowser));
    assert!(app.module_browser.is_some());
}

#[test]
fn empty_bookmarks_and_history_report_noop() {
    let mut app = test_app();
    let temp_dir = tempfile::tempdir().unwrap();
    app.bookmark_store = BookmarkStore::load_with_status(temp_dir.path().join("bookmarks.json")).0;
    app.history = SearchHistory::load_with_status(temp_dir.path().join("history.json")).0;

    app.handle_action(Action::OpenBookmarks);

    assert_eq!(app.popup, None);
    assert!(app.bookmarks_popup.is_none());
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No bookmarks saved"
    ));

    app.status.message = None;
    app.handle_action(Action::SearchHistory);

    assert_eq!(app.popup, None);
    assert!(app.history_popup.is_none());
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No search history"
    ));
}

#[test]
fn quit_action_exits_even_when_popup_is_open() {
    let mut app = test_app();
    app.open_package_scope_popup();

    app.handle_action(Action::Quit);

    assert!(app.should_quit);
    assert_eq!(app.popup, Some(PopupMode::PackageScope));
}

#[test]
fn bookmark_delete_keeps_selection_near_deleted_row() {
    let mut app = test_app();
    for name in ["first", "second", "third"] {
        app.bookmark_store.add(Bookmark {
            name: name.to_string(),
            module: None,
            package: None,
            signature: None,
            doc_url: None,
            added: chrono::Utc::now(),
        });
    }
    app.handle_action(Action::OpenBookmarks);
    app.handle_action(Action::MoveDown);

    app.handle_action(Action::DeleteEntry);

    assert_eq!(app.bookmark_store.bookmarks().len(), 2);
    assert_eq!(
        app.bookmarks_popup.as_ref().map(|popup| popup.selected),
        Some(1)
    );
}

#[test]
fn bookmark_select_without_url_reports_noop() {
    let mut app = test_app();
    app.bookmark_store.add(Bookmark {
        name: "map".to_string(),
        module: None,
        package: None,
        signature: None,
        doc_url: None,
        added: chrono::Utc::now(),
    });
    app.handle_action(Action::OpenBookmarks);

    app.handle_action(Action::Select);

    assert_eq!(app.popup, None);
    assert_eq!(app.mode, AppMode::Search);
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No URL available"
    ));
}

#[test]
fn bookmark_delete_without_selection_reports_noop() {
    let mut app = test_app();
    app.bookmarks_popup = Some(crate::ui::bookmarks_popup::BookmarksPopupState {
        selected: app.bookmark_store.bookmarks().len(),
    });
    app.popup = Some(PopupMode::Bookmarks);

    app.handle_action(Action::DeleteEntry);

    assert_eq!(app.popup, Some(PopupMode::Bookmarks));
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No bookmark selected"
    ));
}

#[test]
fn history_delete_keeps_selection_near_deleted_row() {
    let mut app = test_app();
    app.history.add("first", 1);
    app.history.add("second", 2);
    app.history.add("third", 3);
    app.handle_action(Action::SearchHistory);
    app.handle_action(Action::MoveDown);

    app.handle_action(Action::DeleteEntry);

    assert_eq!(app.history.entries().len(), 2);
    assert_eq!(
        app.history_popup.as_ref().map(|popup| popup.selected),
        Some(1)
    );
    assert_eq!(app.history.entries()[1].query, "first");
}

#[test]
fn history_delete_without_selection_reports_noop() {
    let mut app = test_app();
    app.history_popup = Some(history_popup::HistoryPopupState::new(0));
    app.popup = Some(PopupMode::History);

    app.handle_action(Action::DeleteEntry);

    assert_eq!(app.popup, Some(PopupMode::History));
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No history entry selected"
    ));
}

#[test]
fn pin_helpers_update_pinned_state() {
    let mut app = test_app();
    app.results.set_items(vec![result("map")]);

    app.pin_selected_result();
    assert!(!app.pinned.is_empty());

    app.pin_selected_result();
    assert!(app.pinned.is_empty());

    app.pin_selected_result();
    app.clear_pinned_results();
    assert!(app.pinned.is_empty());
}

#[test]
fn clearing_empty_pins_reports_noop() {
    let mut app = test_app();

    app.clear_pinned_results();

    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No pins to clear"
    ));
}

#[test]
fn unavailable_yank_and_bookmark_commands_report_noop() {
    let mut app = test_app();

    app.bookmark_selected();
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No result selected"
    ));

    app.pin_selected_result();
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No result selected"
    ));

    app.yank_qualified_name();
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No result selected"
    ));

    app.open_doc_for_selected();
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No result selected"
    ));

    app.results.set_items(vec![result("map")]);

    app.yank_signature();
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No signature available"
    ));

    app.yank_import();
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No import available"
    ));

    app.yank_url();
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No URL available"
    ));
}

#[test]
fn mouse_wheel_scrolls_pinned_panel_independently() {
    let mut app = test_app();
    app.mode = AppMode::Results;
    for name in ["a", "b", "c", "d"] {
        app.pinned.pin(&result(name));
    }
    app.pinned.viewport_height = 3;
    app.hit_pinned_panel = Some(Rect::new(40, 10, 20, 8));
    app.hit_preview_pane = Some(Rect::new(40, 0, 20, 10));

    app.handle_mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 45,
        row: 12,
        modifiers: KeyModifiers::NONE,
    });

    assert_eq!(app.pinned.scroll_offset, 3);
    assert_eq!(app.preview_state.scroll_offset, 0);
}

#[test]
fn scroll_up_in_results_mode_scrolls_preview_when_enabled() {
    let mut app = test_app();
    app.mode = AppMode::Results;
    app.preview_enabled = true;
    app.results
        .set_items(vec![result("first"), result("second")]);
    app.results.selected = 1;
    app.preview_state.total_lines = 20;
    app.preview_state.viewport_height = 5;
    app.preview_state.scroll_offset = 3;

    app.handle_action(Action::ScrollUp);

    assert_eq!(app.preview_state.scroll_offset, 2);
    assert_eq!(app.results.selected, 1);
}

#[test]
fn page_scroll_actions_target_active_scrollable_view() {
    let mut app = test_app();
    app.mode = AppMode::SourceView;
    app.doc_state.viewport_height = 10;
    app.source_state.viewport_height = 8;
    app.source_state.rendered_lines = vec![ratatui::text::Line::from(""); 20];

    app.handle_action(Action::ScrollPageDown);

    assert_eq!(app.source_state.scroll_offset, 6);
    assert_eq!(app.doc_state.scroll_offset, 0);

    app.mode = AppMode::Help;
    app.help_state.viewport_height = 12;
    app.handle_action(Action::ScrollHalfDown);

    assert_eq!(app.help_state.scroll_offset, 6);
    assert_eq!(app.doc_state.scroll_offset, 0);
}

#[test]
fn page_scroll_actions_target_results_preview_when_enabled() {
    let mut app = test_app();
    app.mode = AppMode::Results;
    app.preview_enabled = true;
    app.preview_state.total_lines = 30;
    app.preview_state.viewport_height = 10;

    app.handle_action(Action::ScrollPageDown);

    assert_eq!(app.preview_state.scroll_offset, 8);
    assert_eq!(app.doc_state.scroll_offset, 0);
}

#[test]
fn help_closes_back_to_previous_mode() {
    let mut app = app_with_doc();

    app.handle_action(Action::ToggleHelp);
    assert_eq!(app.mode, AppMode::Help);

    app.handle_action(Action::Back);
    assert_eq!(app.mode, AppMode::DocView);

    app.mode = AppMode::SourceView;
    app.handle_action(Action::ToggleHelp);
    app.handle_action(Action::ToggleHelp);

    assert_eq!(app.mode, AppMode::SourceView);
}

#[test]
fn move_to_edge_scrolls_help_without_moving_hidden_results() {
    let mut app = test_app();
    app.mode = AppMode::Help;
    app.help_previous_mode = Some(AppMode::DocView);
    app.help_state.viewport_height = 5;
    app.results
        .set_items(vec![result("first"), result("second")]);
    app.results.selected = 1;

    app.handle_action(Action::MoveToBottom);

    assert!(app.help_state.scroll_offset > 0);
    assert_eq!(app.results.selected, 1);

    app.handle_action(Action::MoveToTop);

    assert_eq!(app.help_state.scroll_offset, 0);
    assert_eq!(app.results.selected, 1);
}

#[test]
fn direct_mode_switches_clear_stale_help_restore_mode() {
    let mut app = app_with_doc();

    app.handle_action(Action::ToggleHelp);
    app.handle_action(Action::FocusSearch);

    assert_eq!(app.mode, AppMode::Search);
    assert_eq!(app.help_previous_mode, None);

    app.handle_action(Action::ToggleHelp);
    app.switch_mode(AppMode::DocView);

    assert_eq!(app.help_previous_mode, None);
}

#[test]
fn current_doc_and_declaration_tracks_scroll_position() {
    let mut app = app_with_doc();

    app.doc_state.scroll_offset = 0;
    let (_, decl) = app.current_doc_and_declaration().unwrap();
    assert_eq!(decl.name, "first");

    app.doc_state.scroll_offset = 7;
    let (_, decl) = app.current_doc_and_declaration().unwrap();
    assert_eq!(decl.name, "Second");
    assert_eq!(
        app.current_decl_name().as_deref(),
        Some("Data.Example.Second")
    );
}

#[test]
fn doc_navigation_prefers_search_matches_when_present() {
    let mut app = app_with_doc();
    app.doc_state.search_matches = vec![3, 9];

    app.move_doc_declaration_or_match(true);
    assert_eq!(app.doc_state.scroll_offset, 0);
    assert_eq!(app.doc_state.current_match, Some(0));

    app.move_doc_declaration_or_match(true);
    assert_eq!(app.doc_state.scroll_offset, 6);
    assert_eq!(app.doc_state.current_match, Some(1));
}

#[test]
fn doc_declaration_navigation_reports_when_no_declarations_are_available() {
    let mut app = test_app();
    app.mode = AppMode::DocView;

    app.move_doc_declaration_or_match(true);

    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No declarations available"
    ));
}

#[test]
fn doc_back_returns_to_results_when_history_is_empty() {
    let mut app = app_with_doc();

    app.navigate_doc_back();

    assert_eq!(app.mode, AppMode::Results);
}

#[test]
fn back_from_doc_view_cancels_pending_doc_response() {
    let mut app = app_with_doc();
    let pending_url =
        Url::parse("https://hackage.haskell.org/package/demo/docs/Pending.html").unwrap();
    app.pending_doc_url = Some(pending_url.clone());
    app.doc_state.loading = true;

    app.handle_back();

    assert_eq!(app.mode, AppMode::Results);
    assert_eq!(app.pending_doc_url, None);
    assert!(!app.doc_state.loading);

    app.doc_tx
        .send(DocResponse {
            url: pending_url,
            result: Ok(doc()),
        })
        .unwrap();
    app.on_tick();

    assert_eq!(app.doc_state.current_url, None);
}

#[test]
fn back_from_source_view_cancels_pending_source_response() {
    let mut app = app_with_doc();
    app.mode = AppMode::SourceView;
    app.pending_source_decl = Some("pendingDecl".to_string());
    app.source_state.loading = true;

    app.handle_back();

    assert_eq!(app.mode, AppMode::DocView);
    assert_eq!(app.pending_source_decl, None);
    assert!(!app.source_state.loading);

    app.source_tx
        .send(SourceResponse {
            decl_name: "pendingDecl".to_string(),
            result: Ok("pendingDecl = 1".to_string()),
        })
        .unwrap();
    app.on_tick();

    assert!(app.source_state.source.is_none());
}

#[test]
fn follow_doc_link_focuses_first_visible_link() {
    let mut app = app_with_doc();
    app.doc_state.viewport_height = 10;
    app.doc_state
        .links
        .push((2, Url::parse("https://example.com/external").unwrap()));

    app.follow_doc_link();

    assert_eq!(app.doc_state.focused_link, Some(0));
    assert!(app.doc_state.nav_stack.is_empty());
}

#[test]
fn follow_doc_link_reports_when_no_links_exist() {
    let mut app = app_with_doc();

    app.follow_doc_link();

    assert_eq!(app.doc_state.focused_link, None);
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No links available"
    ));
}

#[test]
fn cycle_doc_link_reports_when_no_links_exist() {
    let mut app = app_with_doc();

    app.handle_action(Action::CycleLink);

    assert_eq!(app.doc_state.focused_link, None);
    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No links available"
    ));
}

#[test]
fn doc_search_match_navigation_reports_when_no_matches_exist() {
    let mut app = app_with_doc();
    app.doc_state.start_search();

    app.handle_doc_search_input(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));

    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No document search matches"
    ));

    app.status.message = None;
    app.handle_doc_search_input(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));

    assert!(matches!(
        app.status.message,
        Some(StatusMessage::Info(ref msg)) if msg == "No document search matches"
    ));
}

#[test]
fn follow_external_doc_link_reports_without_fetching() {
    let mut app = app_with_doc();
    app.doc_state.focused_link = Some(0);
    app.doc_state
        .links
        .push((2, Url::parse("https://example.com/external").unwrap()));

    app.follow_doc_link();

    assert!(!app.doc_state.loading);
    assert!(app.doc_state.nav_stack.is_empty());
    assert!(app.status.message.is_some());
}

#[tokio::test]
async fn follow_internal_doc_link_pushes_current_url() {
    let mut app = app_with_doc();
    let current_url =
        Url::parse("https://hackage.haskell.org/package/demo/docs/Demo.html").unwrap();
    let next_url =
        Url::parse("https://hackage.haskell.org/package/demo/docs/Demo.html#v:next").unwrap();
    app.doc_state.current_url = Some(current_url.clone());
    app.doc_state.focused_link = Some(0);
    app.doc_state.links.push((2, next_url));

    app.follow_doc_link();

    assert_eq!(app.doc_state.nav_stack, vec![current_url]);
    assert!(app.doc_state.loading);
}

#[test]
fn doc_response_records_current_url() {
    let mut app = app_with_doc();
    let url = Url::parse("https://hackage.haskell.org/package/demo/docs/Demo.html").unwrap();
    app.pending_doc_url = Some(url.clone());

    app.doc_tx
        .send(DocResponse {
            url: url.clone(),
            result: Ok(doc()),
        })
        .unwrap();
    app.on_tick();

    assert_eq!(app.doc_state.current_url, Some(url));
}

#[test]
fn stale_doc_response_is_ignored() {
    let mut app = app_with_doc();
    let current_url =
        Url::parse("https://hackage.haskell.org/package/demo/docs/Current.html").unwrap();
    let stale_url = Url::parse("https://hackage.haskell.org/package/demo/docs/Stale.html").unwrap();
    app.pending_doc_url = Some(current_url.clone());
    app.doc_state.current_url = Some(current_url.clone());

    app.doc_tx
        .send(DocResponse {
            url: stale_url,
            result: Ok(doc()),
        })
        .unwrap();
    app.on_tick();

    assert_eq!(app.doc_state.current_url, Some(current_url));
    assert!(app.pending_doc_url.is_some());
}

#[test]
fn toc_popup_filter_methods_update_visible_entries() {
    let mut app = test_app();
    app.toc_state = Some(toc_popup::TocState::new(vec![
        toc_popup::TocEntry {
            name: "lookup".to_string(),
            signature: None,
            line_offset: 2,
        },
        toc_popup::TocEntry {
            name: "insert".to_string(),
            signature: None,
            line_offset: 8,
        },
    ]));
    app.popup = Some(PopupMode::Toc);

    app.add_toc_filter_char('l');
    let toc = app.toc_state.as_ref().unwrap();
    assert_eq!(toc.filter, "l");
    assert_eq!(toc.filtered_indices, vec![0]);

    app.delete_toc_filter_char();
    let toc = app.toc_state.as_ref().unwrap();
    assert!(toc.filter.is_empty());
    assert_eq!(toc.filtered_indices, vec![0, 1]);
}

#[test]
fn history_popup_filter_methods_update_visible_entries() {
    let mut app = test_app();
    app.history.add("codex-history-map-filter", 2);
    app.history.add("codex-history-lookup-filter", 1);
    app.history_popup = Some(history_popup::HistoryPopupState::new(
        app.history.entries().len(),
    ));
    app.popup = Some(PopupMode::History);

    for c in "codex-history-map-filter".chars() {
        app.add_history_filter_char(c);
    }
    let popup = app.history_popup.as_ref().unwrap();
    let expected = app
        .history
        .entries()
        .iter()
        .position(|entry| entry.query == "codex-history-map-filter")
        .map(|idx| vec![idx])
        .unwrap_or_default();
    assert_eq!(popup.filter, "codex-history-map-filter");
    assert_eq!(popup.filtered_indices, expected);

    for _ in "codex-history-map-filter".chars() {
        app.delete_history_filter_char();
    }
    let popup = app.history_popup.as_ref().unwrap();
    assert!(popup.filter.is_empty());
    assert_eq!(popup.filtered_indices.len(), app.history.entries().len());
}

#[tokio::test]
async fn open_source_for_current_decl_sets_source_loading_state() {
    let mut app = app_with_doc();
    app.doc_state.scroll_offset = 7;

    app.open_source_for_current_decl();

    assert_eq!(app.mode, AppMode::SourceView);
    assert!(app.source_state.loading);
    assert_eq!(app.source_state.error, None);
}

#[test]
fn source_response_scrolls_to_declaration_name() {
    let mut app = app_with_doc();
    app.pending_source_decl = Some("targetDecl".to_string());
    app.source_tx
        .send(SourceResponse {
            decl_name: "targetDecl".to_string(),
            result: Ok("before\nmore before\ntargetDecl = 1\nafter".to_string()),
        })
        .unwrap();

    app.on_tick();

    assert_eq!(app.source_state.scroll_offset, 2);
}

#[test]
fn stale_source_response_is_ignored() {
    let mut app = app_with_doc();
    app.pending_source_decl = Some("currentDecl".to_string());

    app.source_tx
        .send(SourceResponse {
            decl_name: "staleDecl".to_string(),
            result: Ok("staleDecl = 1".to_string()),
        })
        .unwrap();
    app.on_tick();

    assert!(app.source_state.source.is_none());
    assert_eq!(app.pending_source_decl.as_deref(), Some("currentDecl"));
}

#[test]
fn source_view_yank_text_uses_loaded_source() {
    let mut app = test_app();
    app.mode = AppMode::SourceView;
    app.source_state.source = Some("targetDecl = 1".to_string());

    let (text, label) = app.yank_signature_text().unwrap();

    assert_eq!(text, "targetDecl = 1");
    assert_eq!(label, "Copied source to clipboard");
}

#[test]
fn doc_view_browser_url_uses_current_doc_url() {
    let mut app = app_with_doc();
    let previous_url =
        Url::parse("https://hackage.haskell.org/package/demo/docs/Previous.html").unwrap();
    let current_url =
        Url::parse("https://hackage.haskell.org/package/demo/docs/Current.html").unwrap();
    app.doc_state.nav_stack.push(previous_url);
    app.doc_state.current_url = Some(current_url.clone());

    assert_eq!(app.browser_url(), Some(current_url.to_string()));
}

#[test]
fn doc_view_deep_link_uses_current_doc_url() {
    let mut app = app_with_doc();
    let stale_result_url =
        Url::parse("https://hackage.haskell.org/package/demo/docs/Stale.html").unwrap();
    let current_url =
        Url::parse("https://hackage.haskell.org/package/demo/docs/Current.html").unwrap();
    let mut stale_result = result("stale");
    stale_result.doc_url = Some(stale_result_url);
    app.results.set_items(vec![stale_result]);
    app.doc_state.current_url = Some(current_url);

    let (doc, decl) = app.current_doc_and_declaration().unwrap();

    assert_eq!(
        app.decl_deep_link_url(doc, decl).as_deref(),
        Some("https://hackage.haskell.org/package/demo/docs/Current.html#v:first")
    );
}
