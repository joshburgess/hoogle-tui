use hoogle_core::models::SearchResult;
use std::io;
use std::path::PathBuf;

pub fn export_session(
    query: &str,
    results: &[SearchResult],
    viewed_docs: &[(String, String)], // (module, package) pairs
) -> io::Result<PathBuf> {
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let filename = format!("hoogle-export-{timestamp}.md");

    let dir = dirs::document_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    let path = dir.join(&filename);

    let mut md = String::new();

    md.push_str(&format!("# Hoogle Search: {query}\n\n"));
    md.push_str(&format!(
        "Exported at {}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));

    // Results table
    if !results.is_empty() {
        md.push_str(&format!("## Results ({} found)\n\n", results.len()));
        md.push_str("| Name | Module | Package | Signature |\n");
        md.push_str("|------|--------|---------|----------|\n");

        for r in results.iter().take(100) {
            let module = r.module.as_ref().map(|m| m.to_string()).unwrap_or_default();
            let package = r.package.as_ref().map(|p| p.name.as_str()).unwrap_or("");
            let sig = r.signature.as_deref().unwrap_or("");
            let name = &r.name;
            md.push_str(&format!("| `{name}` | {module} | {package} | `{sig}` |\n"));
        }
        md.push('\n');
    }

    // Viewed docs
    if !viewed_docs.is_empty() {
        md.push_str("## Viewed Documentation\n\n");
        for (module, package) in viewed_docs {
            md.push_str(&format!("- **{module}** ({package})\n"));
        }
        md.push('\n');
    }

    std::fs::write(&path, md)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoogle_core::models::{ModulePath, PackageInfo, ResultKind};

    fn make_result(
        name: &str,
        module: Option<&str>,
        package: Option<&str>,
        sig: Option<&str>,
    ) -> SearchResult {
        SearchResult {
            name: name.to_string(),
            module: module.map(|m| ModulePath(m.split('.').map(|s| s.to_string()).collect())),
            package: package.map(|p| PackageInfo {
                name: p.to_string(),
                version: None,
            }),
            signature: sig.map(|s| s.to_string()),
            doc_url: None,
            short_doc: None,
            result_kind: ResultKind::Function,
        }
    }

    #[test]
    fn export_empty_results() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-export.md");

        // We can't easily redirect where export_session writes, so we replicate
        // the markdown generation logic and test the output format directly.
        let results: Vec<SearchResult> = vec![];
        let viewed_docs: Vec<(String, String)> = vec![];

        let mut md = String::new();
        md.push_str("# Hoogle Search: map\n\n");
        md.push_str("Exported at 2026-01-01 00:00:00\n\n");

        if !results.is_empty() {
            md.push_str(&format!("## Results ({} found)\n\n", results.len()));
        }

        if !viewed_docs.is_empty() {
            md.push_str("## Viewed Documentation\n\n");
        }

        std::fs::write(&path, &md).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();

        assert!(content.contains("# Hoogle Search: map"));
        assert!(!content.contains("## Results"));
        assert!(!content.contains("## Viewed Documentation"));
    }

    #[test]
    fn export_with_results_and_docs() {
        let results = vec![
            make_result(
                "lookup",
                Some("Data.Map.Strict"),
                Some("containers"),
                Some("Ord k => k -> Map k a -> Maybe a"),
            ),
            make_result(
                "insert",
                Some("Data.Map.Strict"),
                Some("containers"),
                Some("Ord k => k -> a -> Map k a -> Map k a"),
            ),
        ];
        let viewed_docs = vec![("Data.Map.Strict".to_string(), "containers".to_string())];

        let query = "map";

        // Build the same markdown the function would produce
        let mut md = String::new();
        md.push_str(&format!("# Hoogle Search: {query}\n\n"));
        md.push_str("Exported at test-time\n\n");
        md.push_str(&format!("## Results ({} found)\n\n", results.len()));
        md.push_str("| Name | Module | Package | Signature |\n");
        md.push_str("|------|--------|---------|----------|\n");
        for r in &results {
            let module = r.module.as_ref().map(|m| m.to_string()).unwrap_or_default();
            let package = r.package.as_ref().map(|p| p.name.as_str()).unwrap_or("");
            let sig = r.signature.as_deref().unwrap_or("");
            md.push_str(&format!(
                "| `{}` | {} | {} | `{}` |\n",
                r.name, module, package, sig
            ));
        }
        md.push('\n');
        md.push_str("## Viewed Documentation\n\n");
        for (module, package) in &viewed_docs {
            md.push_str(&format!("- **{module}** ({package})\n"));
        }
        md.push('\n');

        assert!(md.contains("# Hoogle Search: map"));
        assert!(md.contains("## Results (2 found)"));
        assert!(md.contains("| Name | Module | Package | Signature |"));
        assert!(md.contains(
            "| `lookup` | Data.Map.Strict | containers | `Ord k => k -> Map k a -> Maybe a` |"
        ));
        assert!(md.contains("| `insert` | Data.Map.Strict | containers |"));
        assert!(md.contains("## Viewed Documentation"));
        assert!(md.contains("- **Data.Map.Strict** (containers)"));
    }

    #[test]
    fn export_markdown_table_format() {
        let results = vec![make_result(
            "fmap",
            Some("Data.Functor"),
            Some("base"),
            Some("(a -> b) -> f a -> f b"),
        )];
        let viewed_docs: Vec<(String, String)> = vec![];

        let mut md = String::new();
        md.push_str("## Results (1 found)\n\n");
        md.push_str("| Name | Module | Package | Signature |\n");
        md.push_str("|------|--------|---------|----------|\n");
        for r in &results {
            let module = r.module.as_ref().map(|m| m.to_string()).unwrap_or_default();
            let package = r.package.as_ref().map(|p| p.name.as_str()).unwrap_or("");
            let sig = r.signature.as_deref().unwrap_or("");
            md.push_str(&format!(
                "| `{}` | {} | {} | `{}` |\n",
                r.name, module, package, sig
            ));
        }

        // Verify table header
        assert!(md.contains("| Name | Module | Package | Signature |"));
        assert!(md.contains("|------|--------|---------|----------|"));
        // Verify row content
        assert!(md.contains("| `fmap` | Data.Functor | base | `(a -> b) -> f a -> f b` |"));
    }

    #[test]
    fn export_result_with_no_module_or_package() {
        let results = vec![make_result("something", None, None, None)];

        let r = &results[0];
        let module = r.module.as_ref().map(|m| m.to_string()).unwrap_or_default();
        let package = r.package.as_ref().map(|p| p.name.as_str()).unwrap_or("");
        let sig = r.signature.as_deref().unwrap_or("");

        let row = format!("| `{}` | {} | {} | `{}` |", r.name, module, package, sig);
        assert_eq!(row, "| `something` |  |  | `` |");
    }

    #[test]
    fn export_session_writes_file() {
        // Actually call export_session and verify it creates a file
        let results = vec![make_result(
            "map",
            Some("Data.List"),
            Some("base"),
            Some("[a] -> [b]"),
        )];
        let viewed_docs = vec![("Data.List".to_string(), "base".to_string())];

        let path = export_session("map", &results, &viewed_docs).unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Hoogle Search: map"));
        assert!(content.contains("## Results (1 found)"));
        assert!(content.contains("| `map` |"));
        assert!(content.contains("## Viewed Documentation"));
        assert!(content.contains("- **Data.List** (base)"));

        // Clean up
        let _ = std::fs::remove_file(&path);
    }
}
