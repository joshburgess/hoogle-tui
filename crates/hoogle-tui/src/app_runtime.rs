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
                match response.results {
                    Ok(items) => {
                        let count = items.len();
                        self.has_more_results = count >= self.config.ui.max_results;
                        if response.append {
                            self.all_results.extend(items);
                            self.loading_more = false;
                        } else {
                            self.all_results = items;
                        }
                        self.apply_filter_and_sort();
                        self.results.loading = false;
                        self.status.message = None;
                        self.status.offline = false;
                        if !response.append {
                            self.history.add(&self.last_searched, count);
                            self.history.save();
                        }
                    }
                    Err(e) => {
                        self.results.loading = false;
                        self.loading_more = false;
                        let err_str = format!("{e}");
                        if err_str.contains("network") || err_str.contains("timeout") {
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
            match response.result {
                Ok(doc) => {
                    self.viewed_docs
                        .push((doc.module.clone(), doc.package.clone()));
                    self.doc_state.set_doc(doc, &self.theme, self.last_width);
                    self.status.clear_message();
                    self.status.offline = false;
                }
                Err(e) => {
                    self.doc_state.loading = false;
                    self.doc_state.error = Some(format!("{e}"));
                    self.status.set_error(format!("Doc fetch failed: {e}"));
                    self.message_deadline =
                        Some(Instant::now() + std::time::Duration::from_secs(5));
                }
            }
        }

        while let Ok(response) = self.source_rx.try_recv() {
            match response.result {
                Ok(source) => {
                    self.source_state
                        .set_source(source, &response.decl_name, &self.theme);
                    self.status.clear_message();
                }
                Err(e) => {
                    self.source_state.loading = false;
                    self.source_state.error = Some(format!("{e}"));
                    self.status.set_error(format!("Source fetch failed: {e}"));
                    self.message_deadline =
                        Some(Instant::now() + std::time::Duration::from_secs(5));
                }
            }
        }

        if let Some(deadline) = self.message_deadline {
            if Instant::now() >= deadline {
                self.message_deadline = None;
                self.status.clear_message();
            }
        }
    }

    pub(crate) fn show_info(&mut self, msg: &str) {
        self.status.set_info(msg);
        self.message_deadline = Some(Instant::now() + std::time::Duration::from_secs(2));
    }

    pub(crate) fn show_error(&mut self, msg: &str) {
        self.status.set_error(msg);
        self.message_deadline = Some(Instant::now() + std::time::Duration::from_secs(3));
    }
}
