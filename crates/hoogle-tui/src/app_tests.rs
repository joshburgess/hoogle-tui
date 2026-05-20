use super::app_input::search_textarea_with_query;
use async_trait::async_trait;
use hoogle_core::backend::{BackendError, HoogleBackend};
use hoogle_core::config::Config;
use hoogle_core::haddock::types::{Declaration, HaddockDoc};
use hoogle_core::models::{ResultKind, SearchResult};
use url::Url;

use crate::app::{App, AppMode, DocResponse, PopupMode};
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
    app.all_results = vec![result("map")];
    app.results.set_items(app.all_results.clone());
    app.status.result_count = 1;
    app.has_more_results = true;
    app.loading_more = true;

    app.clear_search_state();

    assert!(app.query_text().is_empty());
    assert!(app.last_searched.is_empty());
    assert!(app.all_results.is_empty());
    assert!(app.results.items.is_empty());
    assert_eq!(app.status.result_count, 0);
    assert!(!app.has_more_results);
    assert!(!app.loading_more);
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

    app.toggle_compact_results();
    assert!(app.results.compact);
}

#[test]
fn popup_helpers_open_expected_popup_state() {
    let mut app = test_app();

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
fn pin_helpers_update_pinned_state() {
    let mut app = test_app();
    app.results.set_items(vec![result("map")]);

    app.pin_selected_result();
    assert!(!app.pinned.is_empty());

    app.clear_pinned_results();
    assert!(app.pinned.is_empty());
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
fn doc_back_returns_to_results_when_history_is_empty() {
    let mut app = app_with_doc();

    app.navigate_doc_back();

    assert_eq!(app.mode, AppMode::Results);
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
    app.history.add("map", 2);
    app.history.add("lookup", 1);
    app.history_popup = Some(history_popup::HistoryPopupState::new(
        app.history.entries().len(),
    ));
    app.popup = Some(PopupMode::History);

    app.add_history_filter_char('m');
    let popup = app.history_popup.as_ref().unwrap();
    assert_eq!(popup.filter, "m");
    assert_eq!(popup.filtered_indices, vec![1]);

    app.delete_history_filter_char();
    let popup = app.history_popup.as_ref().unwrap();
    assert!(popup.filter.is_empty());
    assert_eq!(popup.filtered_indices, vec![0, 1]);
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
