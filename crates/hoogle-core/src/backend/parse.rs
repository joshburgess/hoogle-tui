use crate::models::{ModulePath, PackageInfo, ResultKind, SearchResult};
use regex::Regex;
use std::sync::LazyLock;
use url::Url;

/// Strip HTML tags from a string.
fn strip_html(html: &str) -> String {
    static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").unwrap());
    let text = TAG_RE.replace_all(html, "");
    // Decode common HTML entities
    text.replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// Determine the ResultKind from already-stripped item text.
fn detect_result_kind(stripped_text: &str) -> ResultKind {
    if stripped_text.starts_with("data ") {
        ResultKind::DataType
    } else if stripped_text.starts_with("newtype ") {
        ResultKind::Newtype
    } else if stripped_text.starts_with("type ") {
        ResultKind::TypeAlias
    } else if stripped_text.starts_with("class ") {
        ResultKind::Class
    } else if stripped_text.starts_with("module ") {
        ResultKind::Module
    } else if stripped_text.starts_with("package ") {
        ResultKind::Package
    } else {
        ResultKind::Function
    }
}

/// Strip a keyword prefix from a trimmed string, if present.
fn strip_keyword_prefix(trimmed: &str) -> Option<&str> {
    for prefix in [
        "data ", "newtype ", "type ", "class ", "module ", "package ",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return Some(rest.trim());
        }
    }
    None
}

/// Extract the name from the `item` HTML. The name is typically inside
/// `<span class=name>...</span>` or `<s0>...</s0>` tags.
fn extract_name(item_html: &str) -> String {
    // Try to find name in <span class=name> or <s0> tags
    static NAME_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<span class=name>([^<]*(?:<[^>]*>[^<]*)*)</span>"#).unwrap()
    });

    if let Some(cap) = NAME_RE.captures(item_html) {
        let inner = strip_html(&cap[1]);
        let trimmed = inner.trim();
        if let Some(rest) = strip_keyword_prefix(trimmed) {
            return rest.to_string();
        }
        return trimmed.to_string();
    }

    // Fallback: strip all HTML tags and take the first word
    let stripped = strip_html(item_html);
    let trimmed = stripped.trim();
    if let Some(rest) = strip_keyword_prefix(trimmed) {
        return rest.split_whitespace().next().unwrap_or(rest).to_string();
    }
    trimmed
        .split(|c: char| c == ':' || c.is_whitespace())
        .next()
        .unwrap_or(trimmed)
        .to_string()
}

/// Extract the type signature from the already-stripped item text.
/// For functions, the signature follows the name (after ` :: `).
/// For types/classes, the full item text is the signature.
fn extract_signature(stripped_trimmed: &str, kind: ResultKind) -> Option<String> {
    match kind {
        ResultKind::Function => stripped_trimmed
            .find(" :: ")
            .map(|pos| stripped_trimmed[pos + 4..].trim().to_string()),
        ResultKind::Module | ResultKind::Package => None,
        _ => {
            // For data/newtype/type/class, the whole item is the signature
            Some(stripped_trimmed.to_string())
        }
    }
}

/// Extract the first paragraph from the docs string as a short doc.
fn extract_short_doc(docs: &str) -> Option<String> {
    let trimmed = docs.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Take the first paragraph (up to first blank line or first \n\n)
    let first_para = trimmed.split("\n\n").next().unwrap_or(trimmed);
    // Collapse whitespace
    let collapsed: String = first_para.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        None
    } else {
        Some(collapsed)
    }
}

/// Extract package version from the package URL or name.
/// Hackage URLs look like: https://hackage.haskell.org/package/containers-0.6.7
fn extract_version_from_url(url_str: &str) -> Option<String> {
    static VERSION_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"/package/[a-zA-Z][\w-]*?-([\d]+(?:\.[\d]+)*)/?$").unwrap());
    VERSION_RE.captures(url_str).map(|c| c[1].to_string())
}

/// Parse a single Hoogle JSON result into a SearchResult.
pub fn parse_hoogle_json(value: &serde_json::Value) -> Result<SearchResult, String> {
    let item_html = value["item"].as_str().unwrap_or("");
    let stripped_item = strip_html(item_html);
    let stripped_trimmed = stripped_item.trim();

    let kind = detect_result_kind(stripped_trimmed);
    let name = extract_name(item_html);
    let signature = extract_signature(stripped_trimmed, kind);

    let doc_url = value["url"].as_str().and_then(|s| Url::parse(s).ok());

    let module = value["module"]["name"]
        .as_str()
        .map(|name| ModulePath(name.split('.').map(String::from).collect()));

    let package = value["package"]["name"].as_str().map(|pkg_name| {
        let version = value["package"]["url"]
            .as_str()
            .and_then(extract_version_from_url);
        PackageInfo {
            name: pkg_name.to_string(),
            version,
        }
    });

    let short_doc = value["docs"].as_str().and_then(extract_short_doc);

    Ok(SearchResult {
        name,
        module,
        package,
        signature,
        doc_url,
        short_doc,
        result_kind: kind,
    })
}

/// Parse a Hoogle JSON output string (array or newline-delimited) into results.
pub fn parse_hoogle_output(output: &str) -> Result<Vec<SearchResult>, String> {
    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "No results found" {
        return Ok(vec![]);
    }

    // If it doesn't start with '[' or '{', it's probably a non-JSON message
    if !trimmed.starts_with('[') && !trimmed.starts_with('{') {
        return Ok(vec![]);
    }

    // Try parsing as a JSON array first
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed) {
        return arr.iter().map(parse_hoogle_json).collect();
    }

    // Fall back to newline-delimited JSON
    trimmed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let value: serde_json::Value =
                serde_json::from_str(line).map_err(|e| format!("invalid JSON line: {e}"))?;
            parse_hoogle_json(&value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_basic() {
        assert_eq!(
            strip_html("<span class=name><s0>lookup</s0></span> :: <a>Ord</a> k =&gt; k"),
            "lookup :: Ord k => k"
        );
    }

    #[test]
    fn strip_html_entities() {
        assert_eq!(strip_html("a &gt; b &amp; c &lt; d"), "a > b & c < d");
    }

    #[test]
    fn detect_kind_function() {
        let item = r#"<span class=name><s0>lookup</s0></span> :: <a>Ord</a> k =&gt; k -&gt; <a>Map</a> k a -&gt; <a>Maybe</a> a"#;
        let stripped = strip_html(item);
        assert_eq!(detect_result_kind(stripped.trim()), ResultKind::Function);
    }

    #[test]
    fn detect_kind_data() {
        let item = r#"<span class=name><s0>data</s0> <s0>Map</s0></span> k a"#;
        let stripped = strip_html(item);
        assert_eq!(detect_result_kind(stripped.trim()), ResultKind::DataType);
    }

    #[test]
    fn detect_kind_class() {
        let item =
            r#"<span class=name><s0>class</s0> <a>Applicative</a> m =&gt; <s0>Monad</s0></span> m"#;
        let stripped = strip_html(item);
        assert_eq!(detect_result_kind(stripped.trim()), ResultKind::Class);
    }

    #[test]
    fn detect_kind_newtype() {
        let item = r#"<span class=name>newtype Identity</span> a"#;
        let stripped = strip_html(item);
        assert_eq!(detect_result_kind(stripped.trim()), ResultKind::Newtype);
    }

    #[test]
    fn detect_kind_type_alias() {
        let item = r#"<span class=name>type String</span> = [Char]"#;
        let stripped = strip_html(item);
        assert_eq!(detect_result_kind(stripped.trim()), ResultKind::TypeAlias);
    }

    #[test]
    fn extract_name_function() {
        let item = r#"<span class=name><s0>lookup</s0></span> :: <a>Ord</a> k =&gt; k"#;
        assert_eq!(extract_name(item), "lookup");
    }

    #[test]
    fn extract_name_data() {
        let item = r#"<span class=name><s0>data</s0> <s0>Map</s0></span> k a"#;
        assert_eq!(extract_name(item), "Map");
    }

    #[test]
    fn extract_name_class() {
        let item =
            r#"<span class=name><s0>class</s0> <a>Applicative</a> m =&gt; <s0>Monad</s0></span> m"#;
        assert_eq!(extract_name(item), "Applicative m => Monad");
    }

    #[test]
    fn extract_signature_function() {
        let item = r#"<span class=name><s0>lookup</s0></span> :: <a>Ord</a> k =&gt; k -&gt; <a>Map</a> k a -&gt; <a>Maybe</a> a"#;
        let stripped = strip_html(item);
        let sig = extract_signature(stripped.trim(), ResultKind::Function);
        assert_eq!(sig.unwrap(), "Ord k => k -> Map k a -> Maybe a");
    }

    #[test]
    fn extract_signature_data() {
        let item = r#"<span class=name><s0>data</s0> <s0>Map</s0></span> k a"#;
        let stripped = strip_html(item);
        let sig = extract_signature(stripped.trim(), ResultKind::DataType);
        assert_eq!(sig.unwrap(), "data Map k a");
    }

    #[test]
    fn extract_signature_module() {
        let item = r#"module Data.Map"#;
        let stripped = strip_html(item);
        assert_eq!(extract_signature(stripped.trim(), ResultKind::Module), None);
    }

    #[test]
    fn extract_short_doc_basic() {
        let docs = "O(log n). Look up the value at a key in the map.\nThe function will return the corresponding value.";
        assert_eq!(
            extract_short_doc(docs).unwrap(),
            "O(log n). Look up the value at a key in the map. The function will return the corresponding value."
        );
    }

    #[test]
    fn extract_short_doc_multipar() {
        let docs = "First paragraph.\n\nSecond paragraph.";
        assert_eq!(extract_short_doc(docs).unwrap(), "First paragraph.");
    }

    #[test]
    fn extract_short_doc_empty() {
        assert_eq!(extract_short_doc(""), None);
        assert_eq!(extract_short_doc("   "), None);
    }

    #[test]
    fn extract_version_from_url_basic() {
        assert_eq!(
            extract_version_from_url("https://hackage.haskell.org/package/containers-0.6.7"),
            Some("0.6.7".into())
        );
    }

    #[test]
    fn extract_version_from_url_no_version() {
        assert_eq!(
            extract_version_from_url("https://hackage.haskell.org/package/containers"),
            None
        );
    }

    #[test]
    fn parse_function_result() {
        let json = serde_json::json!({
            "url": "https://hackage.haskell.org/package/containers-0.6.7/docs/Data-Map-Strict.html#v:lookup",
            "module": {"name": "Data.Map.Strict", "url": "https://hackage.haskell.org/package/containers-0.6.7/docs/Data-Map-Strict.html"},
            "package": {"name": "containers", "url": "https://hackage.haskell.org/package/containers-0.6.7"},
            "item": "<span class=name><s0>lookup</s0></span> :: <a>Ord</a> k =&gt; k -&gt; <a>Map</a> k a -&gt; <a>Maybe</a> a",
            "type": "",
            "docs": "O(log n). Look up the value at a key in the map.\nThe function will return the corresponding value as (Just value),\nor Nothing if the key isn't in the map."
        });

        let result = parse_hoogle_json(&json).unwrap();
        assert_eq!(result.name, "lookup");
        assert_eq!(result.result_kind, ResultKind::Function);
        assert_eq!(
            result.signature.unwrap(),
            "Ord k => k -> Map k a -> Maybe a"
        );
        assert_eq!(result.module.unwrap().to_string(), "Data.Map.Strict");
        assert_eq!(result.package.as_ref().unwrap().name, "containers");
        assert_eq!(
            result.package.as_ref().unwrap().version.as_deref(),
            Some("0.6.7")
        );
        assert!(result.short_doc.unwrap().starts_with("O(log n)"));
    }

    #[test]
    fn parse_data_result() {
        let json = serde_json::json!({
            "url": "https://hackage.haskell.org/package/containers-0.6.7/docs/Data-Map-Strict.html#t:Map",
            "module": {"name": "Data.Map.Strict", "url": ""},
            "package": {"name": "containers", "url": "https://hackage.haskell.org/package/containers-0.6.7"},
            "item": "<span class=name><s0>data</s0> <s0>Map</s0></span> k a",
            "type": "",
            "docs": "A Map from keys k to values a.\n..."
        });

        let result = parse_hoogle_json(&json).unwrap();
        assert_eq!(result.name, "Map");
        assert_eq!(result.result_kind, ResultKind::DataType);
        assert!(result.signature.unwrap().contains("data Map"));
    }

    #[test]
    fn parse_class_result() {
        let json = serde_json::json!({
            "url": "https://hackage.haskell.org/package/base-4.18.0.0/docs/Control-Monad.html#t:Monad",
            "module": {"name": "Control.Monad", "url": ""},
            "package": {"name": "base", "url": "https://hackage.haskell.org/package/base-4.18.0.0"},
            "item": "<span class=name><s0>class</s0> <a>Applicative</a> m =&gt; <s0>Monad</s0></span> m",
            "type": "",
            "docs": "The Monad class defines the basic operations over a monad..."
        });

        let result = parse_hoogle_json(&json).unwrap();
        assert_eq!(result.result_kind, ResultKind::Class);
        assert!(result.signature.is_some());
    }

    #[test]
    fn parse_result_missing_fields() {
        let json = serde_json::json!({
            "item": "something",
            "type": "",
            "docs": ""
        });

        let result = parse_hoogle_json(&json).unwrap();
        assert_eq!(result.name, "something");
        assert!(result.module.is_none());
        assert!(result.package.is_none());
        assert!(result.doc_url.is_none());
        assert!(result.short_doc.is_none());
    }

    #[test]
    fn parse_no_docs() {
        let json = serde_json::json!({
            "url": "https://example.com",
            "module": {"name": "Foo.Bar", "url": ""},
            "package": {"name": "foo", "url": ""},
            "item": "<span class=name>baz</span> :: Int -> Int",
            "type": "",
            "docs": ""
        });

        let result = parse_hoogle_json(&json).unwrap();
        assert_eq!(result.name, "baz");
        assert!(result.short_doc.is_none());
    }

    #[test]
    fn parse_complex_signature() {
        let json = serde_json::json!({
            "url": "https://example.com",
            "module": {"name": "Data.Foldable", "url": ""},
            "package": {"name": "base", "url": ""},
            "item": "<span class=name>foldMap</span> :: (<a>Foldable</a> t, <a>Monoid</a> m) =&gt; (a -&gt; m) -&gt; t a -&gt; m",
            "type": "",
            "docs": "Map each element to a monoid and combine."
        });

        let result = parse_hoogle_json(&json).unwrap();
        assert_eq!(result.name, "foldMap");
        assert_eq!(
            result.signature.unwrap(),
            "(Foldable t, Monoid m) => (a -> m) -> t a -> m"
        );
    }

    #[test]
    fn parse_output_array_format() {
        let output = r#"[
            {"url":"https://example.com","module":{"name":"A","url":""},"package":{"name":"p","url":""},"item":"<span class=name>foo</span> :: Int","type":"","docs":""},
            {"url":"https://example.com","module":{"name":"B","url":""},"package":{"name":"q","url":""},"item":"<span class=name>bar</span> :: Bool","type":"","docs":""}
        ]"#;

        let results = parse_hoogle_output(output).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "foo");
        assert_eq!(results[1].name, "bar");
    }

    #[test]
    fn parse_output_ndjson_format() {
        let output = r#"{"url":"https://example.com","module":{"name":"A","url":""},"package":{"name":"p","url":""},"item":"<span class=name>foo</span> :: Int","type":"","docs":""}
{"url":"https://example.com","module":{"name":"B","url":""},"package":{"name":"q","url":""},"item":"<span class=name>bar</span> :: Bool","type":"","docs":""}"#;

        let results = parse_hoogle_output(output).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn parse_output_empty() {
        assert!(parse_hoogle_output("").unwrap().is_empty());
        assert!(parse_hoogle_output("  \n  ").unwrap().is_empty());
    }
}
