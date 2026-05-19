use crate::app::{App, AppMode};
use crate::bookmarks::Bookmark;
use crate::clipboard;

impl App {
    pub(crate) fn bookmark_selected(&mut self) {
        if let Some(result) = self.results.selected_result() {
            let bookmark = Bookmark {
                name: result.name.clone(),
                module: result.module.as_ref().map(|m| m.to_string()),
                package: result.package.as_ref().map(|p| p.to_string()),
                signature: result.signature.clone(),
                doc_url: result.doc_url.clone(),
                added: chrono::Utc::now(),
            };
            self.bookmark_store.add(bookmark);
            self.bookmark_store.save();
            self.show_info("Bookmarked!");
        }
    }

    pub(crate) fn yank_signature(&mut self) {
        if let Some(result) = self.results.selected_result() {
            if let Some(ref sig) = result.signature {
                let text = format!("{} :: {sig}", result.name);
                match clipboard::copy_to_clipboard(&text) {
                    Ok(()) => self.show_info("Copied signature to clipboard"),
                    Err(e) => self.show_error(&e),
                }
            }
        }
    }

    pub(crate) fn yank_import(&mut self) {
        if let Some(result) = self.results.selected_result() {
            let module_str = result.module.as_ref().map(|m| m.to_string());
            if let Some(import) = clipboard::generate_import(&result.name, module_str.as_deref()) {
                match clipboard::copy_to_clipboard(&import) {
                    Ok(()) => self.show_info("Copied import to clipboard"),
                    Err(e) => self.show_error(&e),
                }
            }
        }
    }

    pub(crate) fn yank_url(&mut self) {
        if let Some(result) = self.results.selected_result() {
            if let Some(ref url) = result.doc_url {
                match clipboard::copy_to_clipboard(url.as_str()) {
                    Ok(()) => self.show_info("Copied URL to clipboard"),
                    Err(e) => self.show_error(&e),
                }
            }
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
        let url = match self.mode {
            AppMode::Results => self
                .results
                .selected_result()
                .and_then(|r| r.doc_url.as_ref())
                .map(|u| u.to_string()),
            AppMode::DocView => self
                .doc_state
                .doc
                .as_ref()
                .and_then(|_| self.doc_state.nav_stack.last().map(|u| u.to_string()))
                .or_else(|| {
                    self.results
                        .selected_result()
                        .and_then(|r| r.doc_url.as_ref())
                        .map(|u| u.to_string())
                }),
            _ => None,
        };
        if let Some(url) = url {
            match open::that(&url) {
                Ok(()) => self.show_info("Opened in browser"),
                Err(e) => self.show_error(&format!("Failed to open browser: {e}")),
            }
        } else {
            self.show_info("No URL available");
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
                let module = r.module.as_ref().map(|m| m.to_string())?;
                Some(format!("import {} ({})", module, r.name))
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

        if let Some(ref anchor) = decl.anchor {
            let base = self
                .results
                .selected_result()
                .and_then(|r| r.doc_url.as_ref())
                .map(|u| {
                    let mut u = u.clone();
                    u.set_fragment(Some(anchor));
                    u.to_string()
                });
            if let Some(url) = base {
                match clipboard::copy_to_clipboard(&url) {
                    Ok(()) => self.show_info("Copied deep link to clipboard"),
                    Err(e) => self.show_error(&e),
                }
                return;
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
        let url = format!(
            "https://hackage.haskell.org/package/{}/docs/{}.html#{kind_prefix}:{}",
            doc.package,
            doc.module.replace('.', "-"),
            decl.name
        );
        match clipboard::copy_to_clipboard(&url) {
            Ok(()) => self.show_info("Copied deep link to clipboard"),
            Err(e) => self.show_error(&e),
        }
    }

    pub(crate) fn detect_and_apply_project(&mut self) {
        match crate::project::detect_project() {
            Some(info) => {
                let pkg_count = info.dependencies.len();
                self.package_scope = info.dependencies;
                self.status.package_scope = self.package_scope.clone();
                let proj_type = match info.project_type {
                    crate::project::ProjectType::Cabal => "cabal",
                    crate::project::ProjectType::Stack => "stack",
                };
                self.show_info(&format!(
                    "Detected {proj_type} project: {pkg_count} dependencies scoped"
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
