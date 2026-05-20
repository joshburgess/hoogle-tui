use hoogle_core::models::SearchResult;
use std::collections::HashSet;

use crate::app::{App, SearchResponse};
use crate::ui::{sort_popup, status_bar};

use super::app_input::{search_textarea, search_textarea_with_query};
impl App {
    pub(crate) fn clear_search_state(&mut self) {
        self.textarea = search_textarea();
        self.results.set_items(Vec::new());
        self.all_results.clear();
        self.status.result_count = 0;
        self.last_searched.clear();
        self.has_more_results = false;
        self.loading_more = false;
    }

    pub(crate) fn trigger_search(&mut self) {
        let query = self.query_text();
        if query.is_empty() {
            self.results.set_items(Vec::new());
            self.status.result_count = 0;
            self.status.search_by_type = false;
            self.last_searched.clear();
            self.has_more_results = false;
            self.loading_more = false;
            return;
        }

        self.status.search_by_type = query.contains("->") || query.contains("=>");
        self.completion_candidates.clear();
        self.completion_index = 0;

        tracing::info!("searching for: {query}");
        self.search_generation += 1;
        let generation = self.search_generation;
        self.last_searched = query.clone();
        self.results.loading = true;
        self.has_more_results = false;
        self.loading_more = false;
        self.status.message = Some(status_bar::StatusMessage::Loading("Searching...".into()));

        let backend = self.backend.clone();
        let max_results = self.config.ui.max_results;
        let tx = self.search_tx.clone();
        let full_query = self.build_scoped_query(&query);

        tokio::spawn(async move {
            let results = backend.search(&full_query, 0, max_results).await;
            let _ = tx.send(SearchResponse {
                generation,
                append: false,
                results,
            });
        });
    }

    pub(crate) fn apply_filter_and_sort(&mut self) {
        self.apply_filter_and_sort_with_view_reset(true);
    }

    pub(crate) fn apply_filter_and_sort_preserving_view(&mut self) {
        self.apply_filter_and_sort_with_view_reset(false);
    }

    fn apply_filter_and_sort_with_view_reset(&mut self, reset_view: bool) {
        let needs_sort = self.sort_state.active_sort != sort_popup::SortMode::Relevance;

        let mut items: Vec<SearchResult> = if let Some(kind) = self.filter_state.active_filter {
            self.all_results
                .iter()
                .filter(|r| r.result_kind == kind)
                .cloned()
                .collect()
        } else if needs_sort {
            self.all_results.clone()
        } else {
            self.status.result_count = self.all_results.len();
            if reset_view {
                self.results.set_items(self.all_results.clone());
            } else {
                self.results
                    .set_items_preserving_view(self.all_results.clone());
            }
            return;
        };

        match self.sort_state.active_sort {
            sort_popup::SortMode::Relevance => {}
            sort_popup::SortMode::Package => {
                items.sort_by(|a, b| {
                    let pa = a.package.as_ref().map(|p| &p.name);
                    let pb = b.package.as_ref().map(|p| &p.name);
                    pa.cmp(&pb)
                });
            }
            sort_popup::SortMode::Module => {
                items.sort_by(|a, b| {
                    let ma = a.module.as_ref().map(|m| m.as_dotted());
                    let mb = b.module.as_ref().map(|m| m.as_dotted());
                    ma.cmp(&mb)
                });
            }
            sort_popup::SortMode::Name => {
                items.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }

        self.status.result_count = items.len();
        if reset_view {
            self.results.set_items(items);
        } else {
            self.results.set_items_preserving_view(items);
        }
    }

    pub(crate) fn load_more_results(&mut self) {
        let query = self.query_text();
        if query.is_empty() {
            return;
        }

        self.loading_more = true;
        self.status.message = Some(status_bar::StatusMessage::Loading(
            "Loading more results...".into(),
        ));

        let offset = self.all_results.len();
        let backend = self.backend.clone();
        let max_results = self.config.ui.max_results;
        let tx = self.search_tx.clone();
        let generation = self.search_generation;
        let full_query = self.build_scoped_query(&query);

        tokio::spawn(async move {
            let results = backend.search(&full_query, offset, max_results).await;
            let _ = tx.send(SearchResponse {
                generation,
                append: true,
                results,
            });
        });
    }

    pub(crate) fn build_scoped_query(&self, query: &str) -> String {
        if self.package_scope.is_empty() {
            query.to_string()
        } else {
            let prefix: String = self
                .package_scope
                .iter()
                .map(|p| format!("+{p} "))
                .collect();
            format!("{prefix}{query}")
        }
    }

    pub(crate) fn tab_complete(&mut self) {
        let partial = self.query_text();
        if partial.is_empty() {
            return;
        }
        let partial_lower = partial.to_lowercase();

        let mut seen = HashSet::new();
        let candidates: Vec<String> = self
            .results
            .items
            .iter()
            .filter_map(|r| {
                if r.name.to_lowercase().starts_with(&partial_lower) && seen.insert(r.name.clone())
                {
                    Some(r.name.clone())
                } else {
                    None
                }
            })
            .collect();
        if candidates != self.completion_candidates {
            self.completion_candidates = candidates;
            self.completion_index = 0;
        } else {
            self.completion_index = self
                .completion_index
                .min(self.completion_candidates.len().saturating_sub(1));
        }

        if self.completion_candidates.is_empty() {
            return;
        }

        let candidate = &self.completion_candidates[self.completion_index];
        self.textarea = search_textarea_with_query(candidate);
        self.completion_index = (self.completion_index + 1) % self.completion_candidates.len();
    }
}
