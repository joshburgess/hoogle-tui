use hoogle_core::haddock::types::{Declaration, DocBlock, HaddockDoc, Inline};
use url::Url;

use crate::app::{App, AppMode, DocResponse, PopupMode, SourceResponse};
use crate::ui::toc_popup;

impl App {
    pub(crate) fn open_doc_for_selected(&mut self) {
        let Some(result) = self.results.selected_result().cloned() else {
            self.show_info("No result selected");
            return;
        };
        let Some(ref url) = result.doc_url else {
            self.show_error("No documentation URL available");
            return;
        };
        self.switch_mode(AppMode::DocView);
        self.doc_state.loading = true;
        self.doc_state.error = None;
        self.doc_state.current_url = None;
        self.doc_state.nav_stack.clear();
        self.fetch_doc(url.clone());
    }

    pub(crate) fn fetch_doc(&mut self, url: Url) {
        tracing::info!("fetching doc: {url}");
        self.doc_state.loading = true;
        self.doc_state.error = None;
        self.doc_state.clear_search();
        self.pending_doc_url = Some(url.clone());
        self.show_loading("Loading docs...");

        let fetcher = self.fetcher.clone();
        let tx = self.doc_tx.clone();
        let fetch_url = url.clone();
        let started_at = tokio::time::Instant::now();

        tokio::spawn(async move {
            let result = fetcher.fetch_doc(&fetch_url).await;
            let _ = tx.send(DocResponse {
                url: fetch_url,
                started_at,
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

        self.switch_mode(AppMode::SourceView);
        self.source_state.loading = true;
        self.source_state.error = None;
        self.pending_source_decl = Some(decl_name.clone());
        self.show_loading("Loading source...");

        let fetcher = self.fetcher.clone();
        let tx = self.source_tx.clone();
        let started_at = tokio::time::Instant::now();

        tokio::spawn(async move {
            let result = fetcher.fetch_source(&source_url).await;
            let _ = tx.send(SourceResponse {
                decl_name,
                started_at,
                result,
            });
        });
    }

    pub(crate) fn open_toc(&mut self) {
        let Some(ref doc) = self.doc_state.doc else {
            self.show_info("No document loaded");
            return;
        };

        let mut entries = Vec::new();
        for (decl, (_, offset)) in doc
            .declarations
            .iter()
            .zip(self.doc_state.declaration_offsets.iter())
        {
            entries.push(toc_popup::TocEntry {
                name: decl.name.clone(),
                signature: decl.signature.clone(),
                line_offset: *offset,
                level: 1,
            });
            for block in &decl.doc {
                if let DocBlock::Header { content, .. } = block {
                    let name = inline_plain_text(content);
                    if !name.is_empty() {
                        let line_offset =
                            rendered_line_offset(&self.doc_state.rendered_lines, *offset, &name)
                                .unwrap_or(*offset);
                        entries.push(toc_popup::TocEntry {
                            name,
                            signature: None,
                            line_offset,
                            level: 2,
                        });
                    }
                }
            }
        }
        self.toc_state = Some(toc_popup::TocState::new(entries));
        self.popup = Some(PopupMode::Toc);
    }

    pub(crate) fn start_doc_search(&mut self) {
        if self.doc_state.doc.is_none() {
            self.show_info("No document loaded");
            return;
        }
        self.doc_state.start_search();
    }

    pub(crate) fn move_doc_declaration_or_match(&mut self, forward: bool) {
        match (self.doc_state.search_matches.is_empty(), forward) {
            (false, true) => self.doc_state.next_match(),
            (false, false) => self.doc_state.prev_match(),
            (true, _) if self.doc_state.declaration_offsets.is_empty() => {
                self.show_info("No declarations available");
            }
            (true, true) => self.doc_state.next_declaration(),
            (true, false) => self.doc_state.prev_declaration(),
        }
    }

    pub(crate) fn follow_doc_link(&mut self) {
        if let Some(url) = self.doc_state.focused_link_url().cloned() {
            if url.as_str().contains("/docs/") || url.as_str().contains('#') {
                if let Some(current_url) = self.doc_state.current_url.clone() {
                    self.doc_state.push_nav(current_url);
                }
                self.fetch_doc(url);
            } else {
                self.show_info(&format!("Link: {url}"));
            }
        } else {
            if self.doc_state.links.is_empty() {
                self.show_info("No links available");
                return;
            }
            self.doc_state.focus_next_link();
        }
    }

    pub(crate) fn cycle_doc_link(&mut self) {
        if self.doc_state.links.is_empty() {
            self.show_info("No links available");
            return;
        }
        self.doc_state.focus_next_link();
        if self.doc_state.focused_link_url().is_none() {
            self.show_info("No visible links");
        }
    }

    pub(crate) fn navigate_doc_back(&mut self) {
        if let Some(url) = self.doc_state.pop_nav() {
            self.fetch_doc(url);
        } else {
            self.switch_mode(AppMode::Results);
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

fn inline_plain_text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|inline| match inline {
            Inline::Text(text)
            | Inline::Code(text)
            | Inline::ModuleLink(text)
            | Inline::Emphasis(text)
            | Inline::Bold(text)
            | Inline::Math(text) => text.as_str(),
            Inline::Link { text, .. } => text.as_str(),
        })
        .collect()
}

fn rendered_line_offset(
    lines: &[ratatui::text::Line<'static>],
    start: usize,
    needle: &str,
) -> Option<usize> {
    let needle = needle.to_lowercase();
    lines
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, line)| {
            let text: String = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();
            text.to_lowercase().contains(&needle)
        })
        .map(|(index, _)| index)
}
