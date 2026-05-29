use hoogle_core::{
    backend::{BackendError, HoogleBackend},
    config::Config,
    haddock::types::{DocBlock, HaddockDoc, Inline},
    models::{ModulePath, PackageInfo, ResultKind, SearchResult},
};
use hoogle_syntax::theme::Theme;
use ratatui::{backend::TestBackend, layout::Rect, Terminal};
use std::path::PathBuf;

use crate::app::{App, AppMode, PopupMode};
use crate::bookmarks::{Bookmark, BookmarkStore};
use crate::history::SearchHistory;
use crate::ui::{
    bookmarks_popup, command_palette, doc_viewer, filter_popup, help_overlay, history_popup,
    module_browser, package_popup, preview_pane, result_list, search_bar, sort_popup,
    source_viewer, status_bar, theme_popup, toc_popup, yank_popup,
};

#[derive(Debug)]
struct RenderBackend;

#[async_trait::async_trait]
impl HoogleBackend for RenderBackend {
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

fn buffer_text(backend: &TestBackend) -> String {
    backend
        .buffer()
        .content
        .chunks(backend.buffer().area.width as usize)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_to_text<F>(width: u16, height: u16, render: F) -> String
where
    F: FnOnce(&mut ratatui::Frame),
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("failed to create test terminal");
    terminal.draw(render).expect("failed to draw test frame");
    buffer_text(terminal.backend())
}

fn assert_snapshot(name: &str, actual: &str) {
    let path = snapshot_path(name);
    let actual = format!("{}\n", actual.trim_end());

    if std::env::var_os("UPDATE_RENDER_GOLDENS").is_some() {
        std::fs::create_dir_all(path.parent().expect("snapshot path has a parent"))
            .expect("failed to create snapshot directory");
        std::fs::write(&path, actual).expect("failed to write render snapshot");
        return;
    }

    let expected = std::fs::read_to_string(&path).expect("failed to read render snapshot");
    assert_eq!(
        actual,
        expected,
        "render snapshot changed: {}",
        path.display()
    );
}

fn assert_lines_fit(actual: &str, width: usize) {
    for line in actual.lines() {
        assert!(
            line.chars().count() <= width,
            "rendered row has more than {width} cells: {line:?}"
        );
    }
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(format!("{name}.snap"))
}

fn search_result_with_module(name: &str, signature: &str, module: ModulePath) -> SearchResult {
    SearchResult {
        name: name.to_string(),
        module: Some(module),
        package: Some(PackageInfo {
            name: "containers".to_string(),
            version: Some("0.6.7".to_string()),
        }),
        signature: Some(signature.to_string()),
        doc_url: None,
        short_doc: Some("Look up a key in the map.".to_string()),
        result_kind: ResultKind::Function,
    }
}

fn search_result(name: &str, signature: &str) -> SearchResult {
    search_result_with_module(
        name,
        signature,
        ModulePath(vec![
            "Data".to_string(),
            "Map".to_string(),
            "Strict".to_string(),
        ]),
    )
}

fn render_app_to_text(mut app: App, width: u16, height: u16) -> String {
    render_to_text(width, height, |frame| app.draw(frame))
}

fn test_app() -> App {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let mut config = Config::default();
    config.cache.dir = Some(dir.path().join("cache"));
    App::new(config, Box::new(RenderBackend)).expect("failed to create app")
}

fn demo_doc() -> HaddockDoc {
    HaddockDoc {
        module: "Data.Map.Strict".to_string(),
        package: "containers-0.6.7".to_string(),
        description: vec![DocBlock::Paragraph(vec![Inline::Text(
            "Finite maps with strict values.".to_string(),
        )])],
        declarations: vec![hoogle_core::haddock::types::Declaration {
            name: "lookup".to_string(),
            signature: Some("lookup :: Ord k => k -> Map k a -> Maybe a".to_string()),
            doc: vec![DocBlock::Paragraph(vec![Inline::Text(
                "Lookup the value at a key in the map.".to_string(),
            )])],
            since: None,
            source_url: None,
            anchor: Some("v:lookup".to_string()),
        }],
    }
}

#[test]
fn result_list_render_includes_selected_result_metadata() {
    let theme = Theme::dracula();
    let mut state = result_list::ResultListState::new();
    state.set_items(vec![search_result(
        "lookup",
        "Ord k => k -> Map k a -> Maybe a",
    )]);

    let output = render_to_text(80, 12, |frame| {
        result_list::render(frame, Rect::new(0, 0, 80, 12), &mut state, &theme);
    });

    assert!(output.contains("Results"));
    assert!(output.contains("Data.Map.Strict"));
    assert!(output.contains("containers-0.6.7"));
    assert!(output.contains("Map k a -> Maybe a"));
    assert!(output.contains("Look up a key in the map."));
    assert_snapshot("result_list_selected", &output);
}

#[test]
fn full_app_results_snapshot_with_preview_and_pins() {
    let mut app = test_app();
    let lookup = search_result("lookup", "Ord k => k -> Map k a -> Maybe a");
    let insert = search_result("insert", "Ord k => k -> a -> Map k a -> Map k a");
    app.mode = AppMode::Results;
    app.last_searched = "map".to_string();
    app.all_results = vec![lookup.clone(), insert.clone()];
    app.results.set_items(app.all_results.clone());
    app.pinned.pin(&lookup);
    app.status.result_count = app.all_results.len();

    let output = render_app_to_text(app, 100, 28);

    assert!(output.contains("lookup"), "{output}");
    assert!(output.contains("Pinned (1)"), "{output}");
    assert_snapshot("full_app_results_preview_pins", &output);
}

#[test]
fn full_app_doc_snapshot() {
    let mut app = test_app();
    app.mode = AppMode::DocView;
    app.doc_state.set_doc(demo_doc(), &app.theme, 90);

    let output = render_app_to_text(app, 90, 24);

    assert!(output.contains("Data.Map.Strict"), "{output}");
    assert!(output.contains("lookup"), "{output}");
    assert_snapshot("full_app_doc_view", &output);
}

#[test]
fn command_palette_snapshot() {
    let mut app = test_app();
    app.mode = AppMode::Results;
    app.open_command_palette();
    if let Some(ref mut palette) = app.command_palette {
        palette.add_filter_char('p');
        palette.add_filter_char('i');
        palette.add_filter_char('n');
        palette.add_filter_char('n');
        palette.add_filter_char('e');
        palette.add_filter_char('d');
        palette.add_filter_char(' ');
        palette.add_filter_char('i');
        palette.add_filter_char('m');
        palette.add_filter_char('p');
        palette.add_filter_char('o');
        palette.add_filter_char('r');
        palette.add_filter_char('t');
        palette.add_filter_char('s');
    }
    app.popup = Some(PopupMode::CommandPalette);

    let output = render_app_to_text(app, 90, 24);

    assert!(output.contains("Commands: pinned imports"), "{output}");
    assert!(output.contains("Copy pinned imports"), "{output}");
    assert_snapshot("command_palette", &output);
}

#[test]
fn command_palette_long_filter_title_fits_render_width() {
    let theme = Theme::dracula();
    let mut state =
        command_palette::CommandPaletteState::new(vec![command_palette::CommandEntry {
            group: "Project",
            label: "Toggle project scope",
            hint: "project packages",
            action: crate::actions::Action::ToggleProjectScope,
        }]);
    for c in "scope toggle with a deliberately very long trailing query".chars() {
        state.add_filter_char(c);
    }

    let output = render_to_text(36, 10, |frame| {
        command_palette::render(frame, &state, &theme);
    });

    assert_lines_fit(&output, 36);
}

#[test]
fn command_palette_long_row_fits_render_width() {
    let theme = Theme::dracula();
    let state = command_palette::CommandPaletteState::new(vec![command_palette::CommandEntry {
        group: "VeryLongCommandGroupName",
        label: "Toggle an exceptionally long command palette action label",
        hint: "Ctrl-Shift-Alt-Super-P",
        action: crate::actions::Action::ToggleProjectScope,
    }]);

    let output = render_to_text(36, 10, |frame| {
        command_palette::render(frame, &state, &theme);
    });

    assert_lines_fit(&output, 36);
}

#[test]
fn command_palette_title_trims_filter_whitespace() {
    let theme = Theme::dracula();
    let mut state =
        command_palette::CommandPaletteState::new(vec![command_palette::CommandEntry {
            group: "Pins",
            label: "Copy pinned imports",
            hint: "pins",
            action: crate::actions::Action::YankPinnedImports,
        }]);
    for c in "  pins  ".chars() {
        state.add_filter_char(c);
    }

    let output = render_to_text(60, 10, |frame| {
        command_palette::render(frame, &state, &theme);
    });

    assert!(output.contains("Commands: pins (1)"), "{output}");
    assert!(!output.contains("Commands:   pins"), "{output}");
}

#[test]
fn command_palette_truncates_empty_state_on_narrow_terminal() {
    let theme = Theme::dracula();
    let mut state =
        command_palette::CommandPaletteState::new(vec![command_palette::CommandEntry {
            group: "Docs",
            label: "Open docs",
            hint: "Enter",
            action: crate::actions::Action::Select,
        }]);
    for c in "does-not-match-any-command".chars() {
        state.add_filter_char(c);
    }

    let output = render_to_text(24, 8, |frame| {
        command_palette::render(frame, &state, &theme);
    });

    assert!(output.contains("No commands..."), "{output}");
    assert_lines_fit(&output, 24);
}

#[test]
fn result_list_compact_wide_text_keeps_module_visible() {
    let theme = Theme::dracula();
    let mut state = result_list::ResultListState::new();
    state.compact = true;
    state.set_items(vec![search_result_with_module(
        "型型lookup",
        "型型型型型型型型型型型型型型型型 -> Maybe 型",
        ModulePath(vec!["Data".to_string(), "型型型".to_string()]),
    )]);

    let output = render_to_text(42, 5, |frame| {
        result_list::render(frame, Rect::new(0, 0, 42, 5), &mut state, &theme);
    });

    assert!(output.contains("Data."), "{output}");
    assert!(output.contains("型 型 lookup"), "{output}");
    assert!(output.contains("\u{2026}"), "{output}");
    assert_lines_fit(&output, 42);
}

#[test]
fn result_list_long_filter_title_fits_render_width() {
    let theme = Theme::dracula();
    let mut state = result_list::ResultListState::new();
    state.set_items(vec![search_result(
        "map",
        "Ord k => k -> Map k a -> Maybe a",
    )]);
    state.start_fuzzy_filter();
    for c in "  map with a deliberately very long trailing query  ".chars() {
        state.fuzzy_add_char(c);
    }

    let output = render_to_text(36, 5, |frame| {
        result_list::render(frame, Rect::new(0, 0, 36, 5), &mut state, &theme);
    });

    assert!(output.contains("Filter: map"), "{output}");
    assert!(!output.contains("Filter:   map"), "{output}");
    assert!(output.contains("..."), "{output}");
    assert_lines_fit(&output, 36);
}

#[test]
fn result_list_truncates_narrow_state_messages() {
    let theme = Theme::dracula();

    let mut loading = result_list::ResultListState::new();
    loading.loading = true;
    let loading_output = render_to_text(12, 5, |frame| {
        result_list::render(frame, Rect::new(0, 0, 12, 5), &mut loading, &theme);
    });
    assert!(loading_output.contains("Searc..."), "{loading_output}");
    assert_lines_fit(&loading_output, 12);

    let mut empty = result_list::ResultListState::new();
    let empty_output = render_to_text(24, 8, |frame| {
        result_list::render(frame, Rect::new(0, 0, 24, 8), &mut empty, &theme);
    });
    assert!(
        empty_output.contains("Start typing to s..."),
        "{empty_output}"
    );
    assert!(empty_output.contains("Try: map"), "{empty_output}");
    assert!(empty_output.contains("Press ?"), "{empty_output}");
    assert_lines_fit(&empty_output, 24);

    let tiny_empty_output = render_to_text(8, 8, |frame| {
        result_list::render(frame, Rect::new(0, 0, 8, 8), &mut empty, &theme);
    });
    assert_lines_fit(&tiny_empty_output, 8);

    let mut filtered = result_list::ResultListState::new();
    filtered.set_items(vec![search_result(
        "map",
        "Ord k => k -> Map k a -> Maybe a",
    )]);
    filtered.start_fuzzy_filter();
    for c in "zipper".chars() {
        filtered.fuzzy_add_char(c);
    }
    let no_match_output = render_to_text(24, 5, |frame| {
        result_list::render(frame, Rect::new(0, 0, 24, 5), &mut filtered, &theme);
    });
    assert!(
        no_match_output.contains("No matches. Press..."),
        "{no_match_output}"
    );
    assert_lines_fit(&no_match_output, 24);
}

#[test]
fn preview_pane_truncates_metadata_and_url() {
    let theme = Theme::dracula();
    let mut state = preview_pane::PreviewState::new();
    let mut result = search_result_with_module(
        "lookup",
        "Ord k => k -> Map k a -> Maybe a",
        ModulePath(vec![
            "Data".to_string(),
            "Map".to_string(),
            "Strict".to_string(),
            "VeryLongNestedModule".to_string(),
        ]),
    );
    result.package = Some(PackageInfo {
        name: "containers-with-a-long-package-name".to_string(),
        version: None,
    });
    result.doc_url = Some(
        url::Url::parse(
            "https://hackage.haskell.org/package/containers/docs/Data-Map-Strict.html#v:lookup",
        )
        .expect("fixture URL should parse"),
    );

    let output = render_to_text(38, 14, |frame| {
        preview_pane::render(
            frame,
            Rect::new(0, 0, 38, 14),
            Some(&result),
            &mut state,
            &theme,
        );
    });

    assert!(
        output.contains("Data.Map.Strict.Ver...  container..."),
        "{output}"
    );
    assert!(
        output.contains("https://hackage.haskell.org/packa..."),
        "{output}"
    );
    assert_lines_fit(&output, 38);
}

#[test]
fn preview_pane_truncates_state_messages_and_signature() {
    let theme = Theme::dracula();
    let mut empty_state = preview_pane::PreviewState::new();

    let narrow_empty_output = render_to_text(8, 5, |frame| {
        preview_pane::render(frame, Rect::new(0, 0, 8, 5), None, &mut empty_state, &theme);
    });
    assert!(
        narrow_empty_output.contains(" Pr..."),
        "{narrow_empty_output}"
    );
    assert!(
        narrow_empty_output.contains("  S..."),
        "{narrow_empty_output}"
    );
    assert_lines_fit(&narrow_empty_output, 8);

    let empty_output = render_to_text(18, 5, |frame| {
        preview_pane::render(
            frame,
            Rect::new(0, 0, 18, 5),
            None,
            &mut empty_state,
            &theme,
        );
    });
    assert!(empty_output.contains("Select a re..."), "{empty_output}");
    assert_lines_fit(&empty_output, 18);

    let mut state = preview_pane::PreviewState::new();
    let mut result = search_result(
        "lookupWithAnIntentionallyLongName",
        "Ord k => k -> Map k a -> Maybe a",
    );
    result.short_doc = None;

    let output = render_to_text(24, 12, |frame| {
        preview_pane::render(
            frame,
            Rect::new(0, 0, 24, 12),
            Some(&result),
            &mut state,
            &theme,
        );
    });

    assert!(output.contains("\u{2026}"), "{output}");
    assert!(output.contains("No documentation av..."), "{output}");
    assert_lines_fit(&output, 24);
}

#[test]
fn preview_pane_truncates_long_doc_words() {
    let theme = Theme::dracula();
    let mut state = preview_pane::PreviewState::new();
    let mut result = search_result("map", "(a -> b) -> [a] -> [b]");
    result.short_doc =
        Some("SupercalifragilisticexpialidociousIdentifierWithoutSpaces".to_string());

    let output = render_to_text(24, 12, |frame| {
        preview_pane::render(
            frame,
            Rect::new(0, 0, 24, 12),
            Some(&result),
            &mut state,
            &theme,
        );
    });

    assert!(output.contains("\u{2026}"), "{output}");
    assert_lines_fit(&output, 24);
}

#[test]
fn preview_pane_truncates_highlighted_doc_code() {
    let theme = Theme::dracula();
    let mut state = preview_pane::PreviewState::new();
    let mut result = search_result("example", "a -> a");
    result.short_doc =
        Some(">>> veryLongIdentifierWithoutSpaces = veryLongIdentifierWithoutSpaces".to_string());

    let output = render_to_text(28, 12, |frame| {
        preview_pane::render(
            frame,
            Rect::new(0, 0, 28, 12),
            Some(&result),
            &mut state,
            &theme,
        );
    });

    assert!(output.contains("\u{2026}"), "{output}");
    assert_lines_fit(&output, 28);
}

#[test]
fn doc_viewer_table_wide_text_fits_render_width() {
    let theme = Theme::dracula();
    let doc = HaddockDoc {
        module: "Demo.Wide".to_string(),
        package: "demo-0.1.0".to_string(),
        description: vec![DocBlock::Table {
            headers: vec![
                vec![Inline::Text("名前".to_string())],
                vec![Inline::Text("型".to_string())],
            ],
            rows: vec![vec![
                vec![Inline::Text("型型lookup".to_string())],
                vec![Inline::Text(
                    "型型型型型型型型型型型型型型 -> Maybe 型".to_string(),
                )],
            ]],
        }],
        declarations: Vec::new(),
    };
    let mut state = doc_viewer::DocViewState::new();
    state.set_doc(doc, &theme, 44);

    let output = render_to_text(44, 10, |frame| {
        doc_viewer::render(frame, Rect::new(0, 0, 44, 10), &mut state, &theme);
    });

    assert!(output.contains("名 前"), "{output}");
    assert!(output.contains("型 型 lookup"), "{output}");
    assert!(output.contains("\u{2026}"), "{output}");
    assert_lines_fit(&output, 44);
}

#[test]
fn doc_viewer_truncates_long_inline_words() {
    let theme = Theme::dracula();
    let doc = HaddockDoc {
        module: "Demo.LongWords".to_string(),
        package: "demo-0.1.0".to_string(),
        description: vec![DocBlock::Paragraph(vec![Inline::Text(
            "SupercalifragilisticexpialidociousIdentifierWithoutSpaces".to_string(),
        )])],
        declarations: Vec::new(),
    };
    let mut state = doc_viewer::DocViewState::new();
    state.set_doc(doc, &theme, 24);

    let output = render_to_text(24, 8, |frame| {
        doc_viewer::render(frame, Rect::new(0, 0, 24, 8), &mut state, &theme);
    });

    assert!(output.contains("\u{2026}"), "{output}");
    assert_lines_fit(&output, 24);
}

#[test]
fn doc_viewer_truncates_wide_code_blocks() {
    let theme = Theme::dracula();
    let doc = HaddockDoc {
        module: "Demo.Code".to_string(),
        package: "demo-0.1.0".to_string(),
        description: vec![DocBlock::CodeBlock {
            language: Some("haskell".to_string()),
            code: "veryLongIdentifierWithoutSpaces = veryLongIdentifierWithoutSpaces".to_string(),
        }],
        declarations: Vec::new(),
    };
    let mut state = doc_viewer::DocViewState::new();
    state.set_doc(doc, &theme, 28);

    let output = render_to_text(28, 8, |frame| {
        doc_viewer::render(frame, Rect::new(0, 0, 28, 8), &mut state, &theme);
    });

    assert!(output.contains("\u{2026}"), "{output}");
    assert_lines_fit(&output, 28);
}

#[test]
fn doc_viewer_truncates_long_headers_and_declarations() {
    let theme = Theme::dracula();
    let doc = HaddockDoc {
        module: "Demo.Really.Long.Module.Name.With.Many.Segments".to_string(),
        package: "demo-package-with-a-very-long-version-0.1.0".to_string(),
        description: vec![DocBlock::Header {
            level: 1,
            content: vec![Inline::Text(
                "A very long documentation heading without useful breakpoints".to_string(),
            )],
        }],
        declarations: vec![hoogle_core::haddock::types::Declaration {
            name: "veryLongDeclarationName".to_string(),
            signature: Some(
                "veryLongDeclarationName :: VeryLongConstraintName a => a -> a".to_string(),
            ),
            doc: Vec::new(),
            source_url: None,
            anchor: None,
            since: Some("since-a-deliberately-long-release-label".to_string()),
        }],
    };
    let mut state = doc_viewer::DocViewState::new();
    state.set_doc(doc, &theme, 30);

    let output = render_to_text(30, 16, |frame| {
        doc_viewer::render(frame, Rect::new(0, 0, 30, 16), &mut state, &theme);
    });

    assert!(output.contains("\u{2026}"), "{output}");
    assert_lines_fit(&output, 30);
}

#[test]
fn toc_popup_wide_signature_fits_render_width() {
    let theme = Theme::dracula();
    let state = toc_popup::TocState::new(vec![toc_popup::TocEntry {
        name: "型型lookup".to_string(),
        signature: Some("型型型型型型型型型型型型型型 -> Maybe 型".to_string()),
        line_offset: 0,
        level: 1,
    }]);

    let output = render_to_text(42, 8, |frame| {
        toc_popup::render(frame, &state, &theme);
    });

    assert!(output.contains("型 型 lookup"), "{output}");
    assert!(output.contains("..."), "{output}");
    assert_lines_fit(&output, 42);
}

#[test]
fn toc_popup_long_filter_title_fits_render_width() {
    let theme = Theme::dracula();
    let mut state = toc_popup::TocState::new(vec![toc_popup::TocEntry {
        name: "lookup".to_string(),
        signature: None,
        line_offset: 0,
        level: 1,
    }]);
    for c in "  lookup with a deliberately very long trailing query  ".chars() {
        state.add_filter_char(c);
    }

    let output = render_to_text(36, 8, |frame| {
        toc_popup::render(frame, &state, &theme);
    });

    assert!(output.contains("TOC: lookup"), "{output}");
    assert!(!output.contains("TOC:   lookup"), "{output}");
    assert_lines_fit(&output, 36);
}

#[test]
fn toc_popup_truncates_empty_state_and_long_rows() {
    let theme = Theme::dracula();
    let empty = toc_popup::TocState::new(Vec::new());

    let empty_output = render_to_text(18, 8, |frame| {
        toc_popup::render(frame, &empty, &theme);
    });

    assert!(empty_output.contains("No dec..."), "{empty_output}");
    assert_lines_fit(&empty_output, 18);

    let state = toc_popup::TocState::new(vec![toc_popup::TocEntry {
        name: "lookupWithAnIntentionallyLongDeclarationName".to_string(),
        signature: Some("Ord k => k -> Map k a -> Maybe a".to_string()),
        line_offset: 0,
        level: 1,
    }]);

    let output = render_to_text(24, 8, |frame| {
        toc_popup::render(frame, &state, &theme);
    });

    assert!(output.contains("lookupW..."), "{output}");
    assert_lines_fit(&output, 24);
}

#[test]
fn bookmarks_popup_wide_signature_fits_render_width() {
    let theme = Theme::dracula();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let mut store = BookmarkStore::load_with_status(dir.path().join("bookmarks.json")).0;
    store.add(Bookmark {
        name: "型型lookup".to_string(),
        module: Some("Data.型型型".to_string()),
        package: Some("containers".to_string()),
        signature: Some("型型型型型型型型型型型型型型 -> Maybe 型".to_string()),
        doc_url: None,
        added: chrono::Utc::now(),
    });
    let state = bookmarks_popup::BookmarksPopupState::new(store.bookmarks().len());

    let output = render_to_text(60, 8, |frame| {
        bookmarks_popup::render(frame, &state, &store, &theme);
    });

    assert!(output.contains("Data.型 ..."), "{output}");
    assert!(output.contains("..."), "{output}");
    assert_lines_fit(&output, 60);
}

#[test]
fn bookmarks_popup_long_filter_title_fits_render_width() {
    let theme = Theme::dracula();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let mut store = BookmarkStore::load_with_status(dir.path().join("bookmarks.json")).0;
    store.add(Bookmark {
        name: "map".to_string(),
        module: Some("Data.Map".to_string()),
        package: Some("containers".to_string()),
        signature: None,
        doc_url: None,
        added: chrono::Utc::now(),
    });
    let mut state = bookmarks_popup::BookmarksPopupState::new(store.bookmarks().len());
    for c in "  map with a deliberately very long trailing query  ".chars() {
        state.add_filter_char(c, &store);
    }

    let output = render_to_text(36, 8, |frame| {
        bookmarks_popup::render(frame, &state, &store, &theme);
    });

    assert!(output.contains("Bookmarks: map"), "{output}");
    assert!(!output.contains("Bookmarks:   map"), "{output}");
    assert_lines_fit(&output, 36);
}

#[test]
fn bookmarks_popup_truncates_rows_and_empty_states() {
    let theme = Theme::dracula();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let empty_store = BookmarkStore::load_with_status(dir.path().join("empty.json")).0;
    let empty_state = bookmarks_popup::BookmarksPopupState::new(empty_store.bookmarks().len());

    let empty_output = render_to_text(30, 8, |frame| {
        bookmarks_popup::render(frame, &empty_state, &empty_store, &theme);
    });

    assert!(
        empty_output.contains("No bookmarks. P..."),
        "{empty_output}"
    );
    assert_lines_fit(&empty_output, 30);

    let mut store = BookmarkStore::load_with_status(dir.path().join("bookmarks.json")).0;
    store.add(Bookmark {
        name: "lookupWithAnExcessivelyLongName".to_string(),
        module: Some("Data.Map.Strict.Deeply.Nested.Module".to_string()),
        package: Some("containers".to_string()),
        signature: Some("Ord k => k -> Map k a -> Maybe a".to_string()),
        doc_url: None,
        added: chrono::Utc::now(),
    });
    let state = bookmarks_popup::BookmarksPopupState::new(store.bookmarks().len());

    let output = render_to_text(36, 8, |frame| {
        bookmarks_popup::render(frame, &state, &store, &theme);
    });

    assert!(output.contains("look..."), "{output}");
    assert!(output.contains("(Da..."), "{output}");
    assert_lines_fit(&output, 36);
}

#[test]
fn history_popup_long_filter_title_fits_render_width() {
    let theme = Theme::dracula();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let mut history = SearchHistory::load_with_status(dir.path().join("history.json")).0;
    history.add("map", 12);
    let mut state = history_popup::HistoryPopupState::new(history.entries().len());
    state.filter = "  map with a deliberately very long trailing query  ".to_string();
    state.update_filter(&history);

    let output = render_to_text(36, 8, |frame| {
        history_popup::render(frame, &state, &history, &theme);
    });

    assert!(output.contains("History: map"), "{output}");
    assert!(!output.contains("History:   map"), "{output}");
    assert_lines_fit(&output, 36);
}

#[test]
fn history_popup_truncates_long_query_rows() {
    let theme = Theme::dracula();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let mut history = SearchHistory::load_with_status(dir.path().join("history.json")).0;
    history.add(
        "lookup with a deliberately long search history query that should fit",
        12345,
    );
    let state = history_popup::HistoryPopupState::new(history.entries().len());

    let output = render_to_text(38, 8, |frame| {
        history_popup::render(frame, &state, &history, &theme);
    });

    assert!(output.contains("look...  (12345 results)"), "{output}");
    assert_lines_fit(&output, 38);
}

#[test]
fn history_popup_truncates_empty_state_and_large_counts() {
    let theme = Theme::dracula();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let empty_history = SearchHistory::load_with_status(dir.path().join("empty-history.json")).0;
    let empty_state = history_popup::HistoryPopupState::new(empty_history.entries().len());

    let empty_output = render_to_text(14, 8, |frame| {
        history_popup::render(frame, &empty_state, &empty_history, &theme);
    });

    assert!(empty_output.contains("No ..."), "{empty_output}");
    assert_lines_fit(&empty_output, 14);

    let mut history = SearchHistory::load_with_status(dir.path().join("history.json")).0;
    history.add("lookup with a deliberately long query", usize::MAX);
    let state = history_popup::HistoryPopupState::new(history.entries().len());

    let output = render_to_text(22, 8, |frame| {
        history_popup::render(frame, &state, &history, &theme);
    });

    assert!(output.contains("..."), "{output}");
    assert_lines_fit(&output, 22);
}

#[test]
fn module_browser_long_filter_line_fits_render_width() {
    let theme = Theme::dracula();
    let results = vec![search_result("lookup", "Ord k => k -> Map k a -> Maybe a")];
    let mut state = module_browser::ModuleBrowserState::new(&results);
    for c in "  Data.Map.Strict.With.A.Deliberately.Long.Filter.Query  ".chars() {
        state.add_filter_char(c);
    }

    let output = render_to_text(36, 8, |frame| {
        module_browser::render(frame, &mut state, &theme);
    });

    assert!(output.contains("Data.Map"), "{output}");
    assert!(output.contains("..."), "{output}");
    assert!(!output.contains("  Data.Map.Strict.With.A"), "{output}");
    assert_lines_fit(&output, 36);
}

#[test]
fn module_browser_truncates_chrome_empty_state_and_rows() {
    let theme = Theme::dracula();
    let empty_results: Vec<SearchResult> = Vec::new();
    let mut empty_state = module_browser::ModuleBrowserState::new(&empty_results);

    let empty_output = render_to_text(22, 12, |frame| {
        module_browser::render(frame, &mut empty_state, &theme);
    });

    assert!(empty_output.contains("Module B..."), "{empty_output}");
    assert!(empty_output.contains("Enter:se..."), "{empty_output}");
    assert!(empty_output.contains("No modu..."), "{empty_output}");
    assert_lines_fit(&empty_output, 22);

    let results = vec![search_result_with_module(
        "lookup",
        "Ord k => k -> Map k a -> Maybe a",
        ModulePath(vec![
            "Data".to_string(),
            "ReallyLongModuleSegmentName".to_string(),
        ]),
    )];
    let mut state = module_browser::ModuleBrowserState::new(&results);
    state.move_down();

    let output = render_to_text(22, 12, |frame| {
        module_browser::render(frame, &mut state, &theme);
    });

    assert!(output.contains("R... (1)"), "{output}");
    assert_lines_fit(&output, 22);
}

#[test]
fn package_popup_long_input_fits_render_width() {
    let theme = Theme::dracula();
    let mut state = package_popup::PackageScopeState::new(&[]);
    state.input = "base, containers, text, bytestring, vector, unordered-containers, transformers"
        .to_string();

    let output = render_to_text(36, 8, |frame| {
        package_popup::render(frame, &state, &theme);
    });

    assert!(output.contains("base, containers"), "{output}");
    assert!(output.contains("..."), "{output}");
    assert_lines_fit(&output, 36);
}

#[test]
fn package_popup_truncates_prompt_and_footer_hints() {
    let theme = Theme::dracula();
    let state = package_popup::PackageScopeState::new(&[]);

    let output = render_to_text(24, 7, |frame| {
        package_popup::render(frame, &state, &theme);
    });

    assert!(output.contains("Package Scope"), "{output}");
    assert!(output.contains("Comma-separated..."), "{output}");
    assert!(output.contains("e.g.: base, con..."), "{output}");
    assert!(output.contains("Enter:confirm ..."), "{output}");
    assert_lines_fit(&output, 24);
}

#[test]
fn help_overlay_truncates_narrow_rows() {
    let theme = Theme::dracula();
    let mut state = help_overlay::HelpState::new();

    let output = render_to_text(28, 12, |frame| {
        help_overlay::render(frame, &mut state, &theme);
    });

    assert!(output.contains("Sea..."), "{output}");
    assert!(output.contains("Enter         Go ..."), "{output}");
    assert_lines_fit(&output, 28);
}

#[test]
fn help_overlay_truncates_narrow_chrome() {
    let theme = Theme::dracula();
    let mut state = help_overlay::HelpState::new();

    let output = render_to_text(12, 8, |frame| {
        help_overlay::render(frame, &mut state, &theme);
    });

    assert!(output.contains(" ho..."), "{output}");
    assert!(output.contains(" ? ..."), "{output}");
    assert_lines_fit(&output, 12);
}

#[test]
fn help_overlay_clamps_tiny_key_column() {
    let theme = Theme::dracula();
    let mut state = help_overlay::HelpState::new();

    let output = render_to_text(8, 8, |frame| {
        help_overlay::render(frame, &mut state, &theme);
    });

    assert_lines_fit(&output, 8);
}

#[test]
fn fixed_list_popups_truncate_labels_on_narrow_terminal() {
    let theme = Theme::dracula();

    let filter_output = render_to_text(12, 10, |frame| {
        filter_popup::render(frame, &filter_popup::FilterState::new(), &theme);
    });
    let sort_output = render_to_text(12, 8, |frame| {
        sort_popup::render(frame, &sort_popup::SortState::new(), &theme);
    });
    let theme_output = render_to_text(12, 8, |frame| {
        theme_popup::render(
            frame,
            &theme_popup::ThemePopupState::new("catppuccin_mocha"),
            &theme,
        );
    });
    let yank_output = render_to_text(12, 9, |frame| {
        yank_popup::render(frame, &yank_popup::YankPopupState::new(), &theme);
    });

    for output in [&filter_output, &sort_output, &theme_output, &yank_output] {
        assert!(output.contains("..."), "{output}");
        assert_lines_fit(output, 12);
    }
}

#[test]
fn fixed_list_popups_truncate_titles_on_narrow_terminal() {
    let theme = Theme::dracula();

    let filter_output = render_to_text(10, 10, |frame| {
        filter_popup::render(frame, &filter_popup::FilterState::new(), &theme);
    });
    let sort_output = render_to_text(10, 8, |frame| {
        sort_popup::render(frame, &sort_popup::SortState::new(), &theme);
    });
    let theme_output = render_to_text(10, 8, |frame| {
        theme_popup::render(frame, &theme_popup::ThemePopupState::new("dracula"), &theme);
    });
    let yank_output = render_to_text(10, 9, |frame| {
        yank_popup::render(frame, &yank_popup::YankPopupState::new(), &theme);
    });

    assert!(filter_output.contains(" Filt..."), "{filter_output}");
    assert!(sort_output.contains(" Sort..."), "{sort_output}");
    assert!(theme_output.contains(" Swit..."), "{theme_output}");
    assert!(yank_output.contains(" Copy..."), "{yank_output}");
    for output in [&filter_output, &sort_output, &theme_output, &yank_output] {
        assert_lines_fit(output, 10);
    }
}

#[test]
fn popups_render_on_tiny_terminal() {
    let theme = Theme::dracula();
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let history = SearchHistory::load_with_status(dir.path().join("history.json")).0;
    let bookmarks = BookmarkStore::load_with_status(dir.path().join("bookmarks.json")).0;
    let results = vec![search_result("lookup", "Ord k => k -> Map k a -> Maybe a")];
    let mut module_state = module_browser::ModuleBrowserState::new(&results);
    let mut help_state = help_overlay::HelpState::new();
    let command_state =
        command_palette::CommandPaletteState::new(vec![command_palette::CommandEntry {
            group: "Global",
            label: "Focus search",
            hint: "/",
            action: crate::actions::Action::FocusSearch,
        }]);

    let renders = [
        render_to_text(8, 4, |frame| {
            filter_popup::render(frame, &filter_popup::FilterState::new(), &theme);
        }),
        render_to_text(8, 4, |frame| {
            sort_popup::render(frame, &sort_popup::SortState::new(), &theme);
        }),
        render_to_text(8, 4, |frame| {
            theme_popup::render(frame, &theme_popup::ThemePopupState::new("dracula"), &theme);
        }),
        render_to_text(8, 4, |frame| {
            yank_popup::render(frame, &yank_popup::YankPopupState::new(), &theme);
        }),
        render_to_text(8, 4, |frame| {
            package_popup::render(
                frame,
                &package_popup::PackageScopeState::new(&["base".to_string()]),
                &theme,
            );
        }),
        render_to_text(8, 4, |frame| {
            history_popup::render(
                frame,
                &history_popup::HistoryPopupState::new(0),
                &history,
                &theme,
            );
        }),
        render_to_text(8, 4, |frame| {
            bookmarks_popup::render(
                frame,
                &bookmarks_popup::BookmarksPopupState::new(bookmarks.bookmarks().len()),
                &bookmarks,
                &theme,
            );
        }),
        render_to_text(8, 4, |frame| {
            toc_popup::render(frame, &toc_popup::TocState::new(Vec::new()), &theme);
        }),
        render_to_text(8, 4, |frame| {
            module_browser::render(frame, &mut module_state, &theme);
        }),
        render_to_text(8, 4, |frame| {
            command_palette::render(frame, &command_state, &theme);
        }),
        render_to_text(8, 4, |frame| {
            help_overlay::render(frame, &mut help_state, &theme);
        }),
    ];

    for output in renders {
        assert_lines_fit(&output, 8);
    }
}

#[test]
fn app_truncates_tiny_terminal_message() {
    let output = render_app_to_text(test_app(), 8, 5);

    assert!(output.contains("Termi..."), "{output}");
    assert!(output.contains("Need ..."), "{output}");
    assert!(output.contains("Press..."), "{output}");
    assert_lines_fit(&output, 8);
}

#[test]
fn status_bar_render_includes_mode_backend_badges_and_message() {
    let theme = Theme::dracula();
    let mut state = status_bar::StatusState::new("local".to_string());
    state.result_count = 42;
    state.search_by_type = true;
    state.package_scope = vec!["base".to_string(), "containers".to_string()];
    state.set_info("Ready");

    let output = render_to_text(100, 1, |frame| {
        status_bar::render(
            frame,
            Rect::new(0, 0, 100, 1),
            &state,
            AppMode::Results,
            &theme,
        );
    });

    assert!(output.contains("RESULTS"));
    assert!(output.contains("local"));
    assert!(output.contains("[type]"));
    assert!(output.contains("[base,containers]"));
    assert!(output.contains("Ready"));
    assert_snapshot("status_bar_results_ready", &output);
}

#[test]
fn status_bar_source_view_includes_help_and_quit_hints() {
    let theme = Theme::dracula();
    let state = status_bar::StatusState::new("web".to_string());

    let output = render_to_text(100, 1, |frame| {
        status_bar::render(
            frame,
            Rect::new(0, 0, 100, 1),
            &state,
            AppMode::SourceView,
            &theme,
        );
    });

    assert!(output.contains("SOURCE"));
    assert!(output.contains("? help"));
    assert!(output.contains("q quit"));
}

#[test]
fn status_bar_truncates_long_scope_and_message() {
    let theme = Theme::dracula();
    let mut state = status_bar::StatusState::new("local".to_string());
    state.package_scope = vec![
        "base".to_string(),
        "containers".to_string(),
        "unordered-containers".to_string(),
        "transformers".to_string(),
    ];
    state.set_error("Could not load documentation because the cached response was unreadable");

    let output = render_to_text(54, 1, |frame| {
        status_bar::render(
            frame,
            Rect::new(0, 0, 54, 1),
            &state,
            AppMode::Results,
            &theme,
        );
    });

    assert!(output.contains("[base,conta...]"), "{output}");
    assert!(output.contains("Could not load ..."), "{output}");
    assert_lines_fit(&output, 54);
}

#[test]
fn status_bar_truncates_backend_and_large_result_count() {
    let theme = Theme::dracula();
    let mut state =
        status_bar::StatusState::new("local-backend-with-a-deliberately-long-name".to_string());
    state.result_count = usize::MAX;

    let output = render_to_text(32, 1, |frame| {
        status_bar::render(
            frame,
            Rect::new(0, 0, 32, 1),
            &state,
            AppMode::Search,
            &theme,
        );
    });

    assert!(output.contains("loc..."), "{output}");
    assert!(output.contains("res"), "{output}");
    assert!(output.contains("..."), "{output}");
    assert_lines_fit(&output, 32);
}

#[test]
fn status_bar_omits_hints_when_space_is_exhausted() {
    let theme = Theme::dracula();
    let mut state = status_bar::StatusState::new("local".to_string());
    state.set_info("Ready");

    let output = render_to_text(18, 1, |frame| {
        status_bar::render(
            frame,
            Rect::new(0, 0, 18, 1),
            &state,
            AppMode::Search,
            &theme,
        );
    });

    assert!(output.contains("SEARCH"), "{output}");
    assert!(!output.contains("Ctrl-k"), "{output}");
    assert_lines_fit(&output, 18);
}

#[test]
fn source_viewer_render_includes_title_line_numbers_and_code() {
    let theme = Theme::dracula();
    let mut state = source_viewer::SourceViewState::new();
    state.set_source(
        "module Demo where\nanswer = 42\n".to_string(),
        "answer",
        &theme,
    );

    let output = render_to_text(80, 8, |frame| {
        source_viewer::render(frame, Rect::new(0, 0, 80, 8), &mut state, &theme);
    });

    assert!(output.contains("Source: answer"));
    assert!(output.contains("1"));
    assert!(output.contains("module Demo where"));
    assert!(output.contains("answer"));
    assert_snapshot("source_viewer_basic", &output);
}

#[test]
fn source_viewer_truncates_narrow_title_and_error() {
    let theme = Theme::dracula();
    let mut state = source_viewer::SourceViewState::new();
    state.title = "Data.Map.Strict.Internal.lookupWithAReallyLongDeclarationName".to_string();
    state.error =
        Some("Could not fetch source because the remote package index timed out".to_string());

    let output = render_to_text(34, 8, |frame| {
        source_viewer::render(frame, Rect::new(0, 0, 34, 8), &mut state, &theme);
    });

    assert!(
        output.contains("Source: Data.Map.Strict.Inte..."),
        "{output}"
    );
    assert!(
        output.contains("Could not fetch source beca..."),
        "{output}"
    );
    assert_lines_fit(&output, 34);
}

#[test]
fn source_viewer_truncates_wide_code_lines() {
    let theme = Theme::dracula();
    let mut state = source_viewer::SourceViewState::new();
    state.set_source(
        "module Demo where\nlookupWithAnIntentionallyLongImplementationName = mapMaybe lookup values\n"
            .to_string(),
        "lookupWithAnIntentionallyLongImplementationName",
        &theme,
    );

    let output = render_to_text(34, 8, |frame| {
        source_viewer::render(frame, Rect::new(0, 0, 34, 8), &mut state, &theme);
    });

    assert!(output.contains("\u{2026}"), "{output}");
    assert_lines_fit(&output, 34);
}

#[test]
fn pinned_panel_truncates_wide_rows() {
    let theme = Theme::dracula();
    let mut state = crate::ui::pinned_panel::PinnedState::new();
    let mut result = search_result_with_module(
        "型型lookupWithALongName",
        "型型型型型型型型型型型型型型 -> Maybe 型",
        ModulePath(vec![
            "Data".to_string(),
            "型型型".to_string(),
            "DeeplyNestedModuleName".to_string(),
        ]),
    );
    result.package = Some(PackageInfo {
        name: "containers-with-a-long-package-name".to_string(),
        version: None,
    });
    state.pin(&result);

    let output = render_to_text(34, 8, |frame| {
        crate::ui::pinned_panel::render(frame, Rect::new(0, 0, 34, 8), &mut state, &theme);
    });

    assert!(output.contains("\u{2026}"), "{output}");
    assert!(output.contains("Data."), "{output}");
    assert!(output.contains("contain..."), "{output}");
    assert_lines_fit(&output, 34);
}

#[test]
fn pinned_panel_truncates_narrow_chrome_and_empty_state() {
    let theme = Theme::dracula();
    let mut state = crate::ui::pinned_panel::PinnedState::new();

    let output = render_to_text(18, 5, |frame| {
        crate::ui::pinned_panel::render(frame, Rect::new(0, 0, 18, 5), &mut state, &theme);
    });

    assert!(output.contains("Pinned (0)"), "{output}");
    assert!(output.contains("Ctrl-x:unpin..."), "{output}");
    assert!(output.contains("Toggle pins..."), "{output}");
    assert_lines_fit(&output, 18);
}

#[test]
fn search_bar_truncates_narrow_hints() {
    let theme = Theme::dracula();
    let mut textarea = tui_textarea::TextArea::default();

    let output = render_to_text(28, 3, |frame| {
        search_bar::render(
            frame,
            Rect::new(0, 0, 28, 3),
            &mut textarea,
            AppMode::Results,
            false,
            &theme,
        );
    });

    assert!(output.contains("name │ :: a -> b │ +pk..."), "{output}");
    assert_lines_fit(&output, 28);
}

#[test]
fn doc_viewer_search_bar_truncates_long_query() {
    let theme = Theme::dracula();
    let mut state = doc_viewer::DocViewState::new();
    state.search_active = true;
    state.search_query = "lookup with an intentionally long in-document query".to_string();
    state.search_matches = vec![0, 3, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610];
    state.current_match = Some(8);

    let output = render_to_text(38, 8, |frame| {
        doc_viewer::render(frame, Rect::new(0, 0, 38, 8), &mut state, &theme);
    });

    assert!(
        output.contains("/lookup with an intentiona...█ (9/12)"),
        "{output}"
    );
    assert_lines_fit(&output, 38);
}

#[test]
fn doc_viewer_truncates_narrow_state_messages() {
    let theme = Theme::dracula();
    let mut loading = doc_viewer::DocViewState::new();
    loading.loading = true;

    let loading_output = render_to_text(24, 5, |frame| {
        doc_viewer::render(frame, Rect::new(0, 0, 24, 5), &mut loading, &theme);
    });

    assert!(
        loading_output.contains("Loading documenta..."),
        "{loading_output}"
    );
    assert_lines_fit(&loading_output, 24);

    let mut error = doc_viewer::DocViewState::new();
    error.error = Some("documentation request timed out after following redirect".to_string());

    let error_output = render_to_text(30, 7, |frame| {
        doc_viewer::render(frame, Rect::new(0, 0, 30, 7), &mut error, &theme);
    });

    assert!(
        error_output.contains("Error: documentation re..."),
        "{error_output}"
    );
    assert!(
        error_output.contains("Press Esc to go back."),
        "{error_output}"
    );
    assert_lines_fit(&error_output, 30);

    let mut empty = doc_viewer::DocViewState::new();
    let empty_output = render_to_text(20, 5, |frame| {
        doc_viewer::render(frame, Rect::new(0, 0, 20, 5), &mut empty, &theme);
    });

    assert!(empty_output.contains("No documentat..."), "{empty_output}");
    assert_lines_fit(&empty_output, 20);
}

#[test]
fn filter_popup_render_includes_all_options() {
    let theme = Theme::dracula();
    let state = filter_popup::FilterState::new();

    let output = render_to_text(80, 24, |frame| {
        filter_popup::render(frame, &state, &theme);
    });

    assert!(output.contains("Filter Results"));
    assert!(output.contains("All"));
    assert!(output.contains("Functions"));
    assert!(output.contains("Data Types"));
    assert!(output.contains("Packages"));
    assert_snapshot("filter_popup_default", &output);
}
