use arboard::Clipboard;

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("failed to copy: {e}"))
}

/// Generate an import statement for a search result.
pub fn generate_import(name: &str, module: Option<&str>) -> Option<String> {
    let module = module?;
    Some(format!("import {module} ({name})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_import_with_module_and_name() {
        let result = generate_import("lookup", Some("Data.Map.Strict"));
        assert_eq!(result, Some("import Data.Map.Strict (lookup)".to_string()));
    }

    #[test]
    fn generate_import_with_none_module() {
        let result = generate_import("lookup", None);
        assert_eq!(result, None);
    }

    #[test]
    fn generate_import_with_empty_name() {
        let result = generate_import("", Some("Data.Map"));
        assert_eq!(result, Some("import Data.Map ()".to_string()));
    }

    #[test]
    fn generate_import_operator() {
        let result = generate_import("<$>", Some("Data.Functor"));
        assert_eq!(result, Some("import Data.Functor (<$>)".to_string()));
    }

    #[test]
    fn generate_import_single_segment_module() {
        let result = generate_import("putStrLn", Some("Prelude"));
        assert_eq!(result, Some("import Prelude (putStrLn)".to_string()));
    }
}
