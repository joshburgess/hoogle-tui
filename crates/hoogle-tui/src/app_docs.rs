use hoogle_core::haddock::types::{Declaration, HaddockDoc};
use url::Url;

use crate::app::{App, AppMode, DocResponse, PopupMode, SourceResponse};
use crate::ui::{status_bar, toc_popup};

impl App {
    pub(crate) fn open_doc_for_selected(&mut self) {
        let Some(result) = self.results.selected_result().cloned() else {
            return;
        };
        let Some(ref url) = result.doc_url else {
            self.show_error("No documentation URL available");
            return;
        };
        self.mode = AppMode::DocView;
        self.doc_state.loading = true;
        self.doc_state.error = None;
        self.status.message = Some(status_bar::StatusMessage::Loading("Loading docs...".into()));
        self.fetch_doc(url.clone());
    }

    pub(crate) fn fetch_doc(&mut self, url: Url) {
        tracing::info!("fetching doc: {url}");
        self.doc_state.loading = true;
        self.doc_state.error = None;

        let fetcher = self.fetcher.clone();
        let tx = self.doc_tx.clone();
        let fetch_url = url.clone();

        tokio::spawn(async move {
            let result = fetcher.fetch_doc(&fetch_url).await;
            let _ = tx.send(DocResponse {
                url: fetch_url,
                result,
            });
        });
    }

    pub(crate) fn open_source_for_current_decl(&mut self) {
        let Some(decl) = self.current_declaration() else {
            self.show_info("No declaration selected");
            return;
        };

        let Some(source_url) = decl.source_url.clone() else {
            self.show_info("Source not available for this declaration");
            return;
        };
        let decl_name = decl.name.clone();

        self.mode = AppMode::SourceView;
        self.source_state.loading = true;
        self.source_state.error = None;
        self.status.message = Some(status_bar::StatusMessage::Loading(
            "Loading source...".into(),
        ));

        let fetcher = self.fetcher.clone();
        let tx = self.source_tx.clone();

        tokio::spawn(async move {
            let result = fetcher.fetch_source(&source_url).await;
            let _ = tx.send(SourceResponse { decl_name, result });
        });
    }

    pub(crate) fn open_toc(&mut self) {
        if let Some(ref doc) = self.doc_state.doc {
            let entries: Vec<toc_popup::TocEntry> = doc
                .declarations
                .iter()
                .zip(self.doc_state.declaration_offsets.iter())
                .map(|(decl, (_, offset))| toc_popup::TocEntry {
                    name: decl.name.clone(),
                    signature: decl.signature.clone(),
                    line_offset: *offset,
                })
                .collect();
            self.toc_state = Some(toc_popup::TocState::new(entries));
            self.popup = Some(PopupMode::Toc);
        }
    }

    pub(crate) fn move_doc_declaration_or_match(&mut self, forward: bool) {
        match (self.doc_state.search_matches.is_empty(), forward) {
            (false, true) => self.doc_state.next_match(),
            (false, false) => self.doc_state.prev_match(),
            (true, true) => self.doc_state.next_declaration(),
            (true, false) => self.doc_state.prev_declaration(),
        }
    }

    pub(crate) fn follow_doc_link(&mut self) {
        if let Some(url) = self.doc_state.focused_link_url().cloned() {
            if url.as_str().contains("/docs/") || url.as_str().contains('#') {
                self.doc_state.push_nav(url.clone());
                self.fetch_doc(url);
            } else {
                self.show_info(&format!("Link: {url}"));
            }
        } else {
            self.doc_state.focus_next_link();
        }
    }

    pub(crate) fn navigate_doc_back(&mut self) {
        if let Some(url) = self.doc_state.pop_nav() {
            self.fetch_doc(url);
        } else {
            self.mode = AppMode::Results;
        }
    }

    pub(crate) fn current_doc_and_declaration(&self) -> Option<(&HaddockDoc, &Declaration)> {
        let doc = self.doc_state.doc.as_ref()?;
        let (name, _) = self
            .doc_state
            .declaration_offsets
            .iter()
            .rev()
            .find(|(_, off)| *off <= self.doc_state.scroll_offset + 2)?;
        let decl = doc.declarations.iter().find(|d| &d.name == name)?;
        Some((doc, decl))
    }

    pub(crate) fn current_declaration(&self) -> Option<&Declaration> {
        self.current_doc_and_declaration().map(|(_, decl)| decl)
    }

    pub(crate) fn current_decl_name(&self) -> Option<String> {
        let (doc, decl) = self.current_doc_and_declaration()?;
        Some(format!("{}.{}", doc.module, decl.name))
    }
}
