use hoogle_core::{
    haddock::types::{DocBlock, HaddockDoc, Inline},
    models::{ModulePath, PackageInfo, ResultKind, SearchResult},
};
use hoogle_syntax::theme::Theme;
use ratatui::{backend::TestBackend, layout::Rect, Terminal};
use std::path::PathBuf;

use crate::app::AppMode;
use crate::bookmarks::{Bookmark, BookmarkStore};
use crate::ui::{
    bookmarks_popup, doc_viewer, filter_popup, result_list, source_viewer, status_bar, toc_popup,
};

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
fn toc_popup_wide_signature_fits_render_width() {
    let theme = Theme::dracula();
    let state = toc_popup::TocState::new(vec![toc_popup::TocEntry {
        name: "型型lookup".to_string(),
        signature: Some("型型型型型型型型型型型型型型 -> Maybe 型".to_string()),
        line_offset: 0,
    }]);

    let output = render_to_text(42, 8, |frame| {
        toc_popup::render(frame, &state, &theme);
    });

    assert!(output.contains("型 型 lookup"), "{output}");
    assert!(output.contains("..."), "{output}");
    assert_lines_fit(&output, 42);
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
    let state = bookmarks_popup::BookmarksPopupState::new();

    let output = render_to_text(60, 8, |frame| {
        bookmarks_popup::render(frame, &state, &store, &theme);
    });

    assert!(output.contains("Data.型 型 型"), "{output}");
    assert!(output.contains("..."), "{output}");
    assert_lines_fit(&output, 60);
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
