use hoogle_core::haddock::types::{Declaration, HaddockDoc};

use crate::app::{App, AppMode};
use crate::bookmarks::Bookmark;
use crate::clipboard;

impl App {
    pub(crate) fn bookmark_selected(&mut self) {
        let Some(result) = self.results.selected_result() else {
            self.show_info("No result selected");
            return;
        };

        let bookmark = Bookmark {
            name: result.name.clone(),
            module: result.module.as_ref().map(|m| m.to_string()),
            package: result.package.as_ref().map(|p| p.to_string()),
            signature: result.signature.clone(),
            doc_url: result.doc_url.clone(),
            added: chrono::Utc::now(),
        };
        self.bookmark_store.add(bookmark);
        match self.bookmark_store.try_save() {
            Ok(()) => self.show_info("Bookmarked!"),
            Err(e) => self.show_error(&format!("Failed to save bookmark: {e}")),
        }
    }

    pub(crate) fn yank_signature(&mut self) {
        match self.yank_signature_text() {
            Some((text, label)) => match clipboard::copy_to_clipboard(&text) {
                Ok(()) => self.show_info(label),
                Err(e) => self.show_error(&e),
            },
            None => {
                if self.mode == AppMode::SourceView {
                    self.show_info("No source loaded");
                } else {
                    self.show_info("No signature available");
                }
            }
        }
    }

    pub(crate) fn yank_signature_text(&self) -> Option<(String, &'static str)> {
        match self.mode {
            AppMode::SourceView => self
                .source_state
                .source
                .as_ref()
                .map(|source| (source.clone(), "Copied source to clipboard")),
            _ => self.results.selected_result().and_then(|result| {
                result.signature.as_ref().map(|sig| {
                    (
                        format!("{} :: {sig}", result.name),
                        "Copied signature to clipboard",
                    )
                })
            }),
        }
    }

    pub(crate) fn yank_import(&mut self) {
        let Some(result) = self.results.selected_result() else {
            self.show_info("No result selected");
            return;
        };

        let module_str = result.module.as_ref().map(|m| m.to_string());
        let Some(import) = clipboard::generate_import(&result.name, module_str.as_deref()) else {
            self.show_info("No import available");
            return;
        };

        match clipboard::copy_to_clipboard(&import) {
            Ok(()) => self.show_info("Copied import to clipboard"),
            Err(e) => self.show_error(&e),
        }
    }

    pub(crate) fn yank_url(&mut self) {
        let Some(result) = self.results.selected_result() else {
            self.show_info("No result selected");
            return;
        };

        let Some(ref url) = result.doc_url else {
            self.show_info("No URL available");
            return;
        };

        match clipboard::copy_to_clipboard(url.as_str()) {
            Ok(()) => self.show_info("Copied URL to clipboard"),
            Err(e) => self.show_error(&e),
        }
    }

    pub(crate) fn yank_qualified_name(&mut self) {
        if let Some(result) = self.results.selected_result() {
            let module = result.module.as_ref().map(|m| m.to_string());
            let qualified = if let Some(module) = module {
                format!("{module}.{}", result.name)
            } else {
                result.name.clone()
            };
            match clipboard::copy_to_clipboard(&qualified) {
                Ok(()) => self.show_info("Copied qualified name to clipboard"),
                Err(e) => self.show_error(&e),
            }
        }
    }

    pub(crate) fn open_in_browser(&mut self) {
        let url = self.browser_url();
        if let Some(url) = url {
            match open::that(&url) {
                Ok(()) => self.show_info("Opened in browser"),
                Err(e) => self.show_error(&format!("Failed to open browser: {e}")),
            }
        } else {
            self.show_info("No URL available");
        }
    }

    pub(crate) fn browser_url(&self) -> Option<String> {
        match self.mode {
            AppMode::Results => self
                .results
                .selected_result()
                .and_then(|r| r.doc_url.as_ref())
                .map(|u| u.to_string()),
            AppMode::DocView => self
                .doc_state
                .current_url
                .as_ref()
                .map(|u| u.to_string())
                .or_else(|| {
                    self.results
                        .selected_result()
                        .and_then(|r| r.doc_url.as_ref())
                        .map(|u| u.to_string())
                }),
            _ => None,
        }
    }

    pub(crate) fn yank_multi_imports(&mut self) {
        let results = self.results.selected_results();
        if results.is_empty() {
            self.show_info("No results selected (use 'x' to multi-select)");
            return;
        }
        let imports: Vec<String> = results
            .iter()
            .filter_map(|r| {
                let module = r.module.as_ref().map(|m| m.to_string());
                clipboard::generate_import(&r.name, module.as_deref())
            })
            .collect();

        if imports.is_empty() {
            self.show_info("No importable results selected");
            return;
        }

        let text = imports.join("\n");
        match clipboard::copy_to_clipboard(&text) {
            Ok(()) => self.show_info(&format!("Copied {} imports to clipboard", imports.len())),
            Err(e) => self.show_error(&e),
        }
        self.results.multi_select_mode = false;
        self.results.multi_selected.clear();
    }

    pub(crate) fn yank_ghci_command(&mut self, cmd: &str) {
        let name = match self.mode {
            AppMode::Results => self.results.selected_result().map(|r| {
                let module = r.module.as_ref().map(|m| m.to_string());
                if let Some(module) = module {
                    format!("{module}.{}", r.name)
                } else {
                    r.name.clone()
                }
            }),
            AppMode::DocView => self.current_decl_name(),
            _ => None,
        };

        if let Some(name) = name {
            let text = format!("{cmd} {name}");
            match clipboard::copy_to_clipboard(&text) {
                Ok(()) => self.show_info(&format!("Copied: {text}")),
                Err(e) => self.show_error(&e),
            }
        } else {
            self.show_info("No declaration selected");
        }
    }

    pub(crate) fn yank_decl_deep_link(&mut self) {
        if self.mode != AppMode::DocView {
            self.yank_url();
            return;
        }

        let Some((doc, decl)) = self.current_doc_and_declaration() else {
            self.show_info("No declaration at cursor");
            return;
        };

        if let Some(url) = self.decl_deep_link_url(doc, decl) {
            match clipboard::copy_to_clipboard(&url) {
                Ok(()) => self.show_info("Copied deep link to clipboard"),
                Err(e) => self.show_error(&e),
            }
            return;
        }

        self.show_info("No declaration at cursor");
    }

    pub(crate) fn decl_deep_link_url(
        &self,
        doc: &HaddockDoc,
        decl: &Declaration,
    ) -> Option<String> {
        if let Some(ref anchor) = decl.anchor {
            if let Some(url) = self.doc_state.current_url.as_ref().or_else(|| {
                self.results
                    .selected_result()
                    .and_then(|r| r.doc_url.as_ref())
            }) {
                let mut url = url.clone();
                url.set_fragment(Some(anchor));
                return Some(url.to_string());
            }
        }

        let kind_prefix = if decl.signature.as_ref().is_some_and(|s| {
            s.starts_with("data ")
                || s.starts_with("type ")
                || s.starts_with("class ")
                || s.starts_with("newtype ")
        }) {
            "t"
        } else {
            "v"
        };
        Some(format!(
            "https://hackage.haskell.org/package/{}/docs/{}.html#{kind_prefix}:{}",
            doc.package,
            doc.module.replace('.', "-"),
            decl.name
        ))
    }

    pub(crate) fn detect_and_apply_project(&mut self) {
        match crate::project::detect_project() {
            Some(info) => {
                let pkg_count = info.dependencies.len();
                let root = info.root.display().to_string();
                self.package_scope = info.dependencies;
                self.status.package_scope = self.package_scope.clone();
                let proj_type = match info.project_type {
                    crate::project::ProjectType::Cabal => "cabal",
                    crate::project::ProjectType::Stack => "stack",
                };
                self.show_info(&format!(
                    "Detected {proj_type} project at {root}: {pkg_count} dependencies scoped"
                ));
                if !self.last_searched.is_empty() {
                    self.trigger_search();
                }
            }
            None => {
                self.show_info("No Haskell project detected in current directory");
            }
        }
    }
}
