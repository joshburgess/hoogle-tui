use tokio::time::Instant;

use crate::app::App;

impl App {
    pub(crate) fn on_tick(&mut self) {
        self.status.tick();

        if let Some(deadline) = self.debounce_deadline {
            if Instant::now() >= deadline {
                self.debounce_deadline = None;
                let query = self.query_text();
                if query != self.last_searched {
                    self.trigger_search();
                }
            }
        }

        while let Ok(response) = self.search_rx.try_recv() {
            if response.generation == self.search_generation {
                let elapsed = response.started_at.elapsed();
                match response.results {
                    Ok(items) => {
                        let count = items.len();
                        tracing::info!(
                            result_count = count,
                            append = response.append,
                            elapsed_ms = elapsed.as_millis(),
                            "search completed"
                        );
                        self.has_more_results = count >= self.config.ui.max_results;
                        if response.append {
                            self.all_results.extend(items);
                            self.loading_more = false;
                            self.apply_filter_and_sort_preserving_view();
                        } else {
                            self.all_results = items;
                            self.apply_filter_and_sort();
                        }
                        self.results.loading = false;
                        self.clear_status_message();
                        self.status.offline = false;
                        if !response.append {
                            self.history.add(&self.last_searched, count);
                            if let Err(e) = self.history.try_save() {
                                self.status
                                    .set_error(format!("Failed to save history: {e}"));
                                self.message_deadline =
                                    Some(Instant::now() + std::time::Duration::from_secs(5));
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            append = response.append,
                            elapsed_ms = elapsed.as_millis(),
                            error = %e,
                            "search failed"
                        );
                        self.results.loading = false;
                        self.loading_more = false;
                        let err_str = format!("{e}");
                        if is_offline_error(&err_str) {
                            self.status.offline = true;
                        }
                        self.status.set_error(format!("Search failed: {e}"));
                        self.message_deadline =
                            Some(Instant::now() + std::time::Duration::from_secs(5));
                    }
                }
            }
        }

        while let Ok(response) = self.doc_rx.try_recv() {
            if self.pending_doc_url.as_ref() != Some(&response.url) {
                continue;
            }
            self.pending_doc_url = None;
            let elapsed = response.started_at.elapsed();
            match response.result {
                Ok(doc) => {
                    tracing::info!(
                        module = %doc.module,
                        elapsed_ms = elapsed.as_millis(),
                        "doc fetch completed"
                    );
                    self.doc_state.current_url = Some(response.url);
                    self.viewed_docs
                        .push((doc.module.clone(), doc.package.clone()));
                    self.doc_state.set_doc(doc, &self.theme, self.last_width);
                    self.clear_status_message();
                    self.status.offline = false;
                }
                Err(e) => {
                    tracing::warn!(
                        url = %response.url,
                        elapsed_ms = elapsed.as_millis(),
                        error = %e,
                        "doc fetch failed"
                    );
                    self.doc_state.loading = false;
                    self.doc_state.error = Some(format!("{e}"));
                    let err_str = format!("{e}");
                    if is_offline_error(&err_str) {
                        self.status.offline = true;
                    }
                    self.status.set_error(format!("Doc fetch failed: {e}"));
                    self.message_deadline =
                        Some(Instant::now() + std::time::Duration::from_secs(5));
                }
            }
        }

        while let Ok(response) = self.source_rx.try_recv() {
            if self.pending_source_decl.as_deref() != Some(response.decl_name.as_str()) {
                continue;
            }
            self.pending_source_decl = None;
            let elapsed = response.started_at.elapsed();
            match response.result {
                Ok(source) => {
                    tracing::info!(
                        decl = %response.decl_name,
                        bytes = source.len(),
                        elapsed_ms = elapsed.as_millis(),
                        "source fetch completed"
                    );
                    self.source_state
                        .set_source(source, &response.decl_name, &self.theme);
                    self.source_state.scroll_to_first_match(&response.decl_name);
                    self.clear_status_message();
                    self.status.offline = false;
                }
                Err(e) => {
                    tracing::warn!(
                        decl = %response.decl_name,
                        elapsed_ms = elapsed.as_millis(),
                        error = %e,
                        "source fetch failed"
                    );
                    self.source_state.loading = false;
                    self.source_state.error = Some(format!("{e}"));
                    let err_str = format!("{e}");
                    if is_offline_error(&err_str) {
                        self.status.offline = true;
                    }
                    self.status.set_error(format!("Source fetch failed: {e}"));
                    self.message_deadline =
                        Some(Instant::now() + std::time::Duration::from_secs(5));
                }
            }
        }

        if let Some(deadline) = self.message_deadline {
            if Instant::now() >= deadline {
                self.message_deadline = None;
                self.clear_status_message();
            }
        }
    }

    pub(crate) fn clear_status_message(&mut self) {
        self.status.clear_message();
        self.message_deadline = None;
    }

    pub(crate) fn show_info(&mut self, msg: &str) {
        self.status.set_info(msg);
        self.message_deadline = Some(Instant::now() + std::time::Duration::from_secs(2));
    }

    pub(crate) fn show_loading(&mut self, msg: &str) {
        self.status.set_loading(msg);
        self.message_deadline = None;
    }

    pub(crate) fn show_error(&mut self, msg: &str) {
        self.status.set_error(msg);
        self.message_deadline = Some(Instant::now() + std::time::Duration::from_secs(3));
    }
}

fn is_offline_error(message: &str) -> bool {
    message.contains("network") || message.contains("timeout")
}
