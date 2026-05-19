use scraper::{ElementRef, Html, Selector};
use url::Url;

use super::types::{Declaration, DocBlock, HaddockDoc, Inline};

/// Parse a Haddock HTML module page into a structured HaddockDoc.
pub fn parse_haddock_html(html: &str, page_url: &Url) -> Result<HaddockDoc, String> {
    let document = Html::parse_document(html);

    let module = extract_module_name(&document).unwrap_or_default();
    let package = extract_package_name(&document, page_url);
    let description = extract_description(&document, page_url);
    let declarations = extract_declarations(&document, page_url);

    Ok(HaddockDoc {
        module,
        package,
        description,
        declarations,
    })
}

/// Extract raw Haskell source from a Hackage source HTML page.
pub fn parse_source_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let Some(sel) = sel("pre") else {
        return String::new();
    };
    // Source pages typically have a single <pre> with the code
    // Try the largest <pre> block
    let mut best = String::new();
    for el in document.select(&sel) {
        let text = el.text().collect::<String>();
        if text.len() > best.len() {
            best = text;
        }
    }
    best
}

// --- Module / Package extraction ---

fn extract_module_name(doc: &Html) -> Option<String> {
    if let Some(header_sel) = sel("#module-header .caption") {
        if let Some(el) = doc.select(&header_sel).next() {
            let text = el.text().collect::<String>();
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }

    let title_sel = sel("title")?;
    if let Some(el) = doc.select(&title_sel).next() {
        let text = el.text().collect::<String>();
        // Title is often "Module.Name" or "package-name-ver: Module.Name"
        if let Some(colon_pos) = text.find(':') {
            let after = text[colon_pos + 1..].trim();
            if !after.is_empty() {
                return Some(after.to_string());
            }
        }
        let trimmed = text.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }

    None
}

fn extract_package_name(_doc: &Html, page_url: &Url) -> String {
    // Extract from URL: /package/containers-0.6.7/docs/...
    let path = page_url.path();
    if let Some(start) = path.find("/package/") {
        let rest = &path[start + 9..];
        if let Some(end) = rest.find('/') {
            return rest[..end].to_string();
        }
        return rest.to_string();
    }
    String::new()
}

// --- Description extraction ---

fn extract_description(doc: &Html, base_url: &Url) -> Vec<DocBlock> {
    let Some(desc_sel) = sel("#description") else {
        return Vec::new();
    };
    if let Some(desc_el) = doc.select(&desc_sel).next() {
        if let Some(inner_sel) = sel(".doc") {
            if let Some(doc_el) = desc_el.select(&inner_sel).next() {
                return parse_doc_blocks(doc_el, base_url);
            }
        }
        return parse_doc_blocks(desc_el, base_url);
    }
    Vec::new()
}

// --- Declaration extraction ---

fn extract_declarations(doc: &Html, base_url: &Url) -> Vec<Declaration> {
    let mut declarations = Vec::new();

    let Some(top_sel) = sel(".top") else {
        return declarations;
    };
    for top_el in doc.select(&top_sel) {
        if let Some(decl) = parse_declaration(top_el, base_url) {
            declarations.push(decl);
        }
    }

    declarations
}

fn parse_declaration(el: ElementRef, base_url: &Url) -> Option<Declaration> {
    // Extract name and anchor from a.def or p.src a[id]
    let (name, anchor) = extract_decl_name_and_anchor(el)?;

    // Extract type signature from p.src
    let signature = extract_decl_signature(el);

    // Extract source URL
    let source_url = extract_source_url(el, base_url);

    // Extract since annotation
    let since = extract_since(el);

    let doc = sel(".doc")
        .and_then(|doc_sel| el.select(&doc_sel).next())
        .map(|doc_el| parse_doc_blocks(doc_el, base_url))
        .unwrap_or_default();

    Some(Declaration {
        name,
        signature,
        doc,
        since,
        source_url,
        anchor: Some(anchor),
    })
}

fn extract_decl_name_and_anchor(el: ElementRef) -> Option<(String, String)> {
    if let Some(def_sel) = sel("a.def") {
        for a in el.select(&def_sel) {
            let name = a.text().collect::<String>().trim().to_string();
            let anchor = a
                .value()
                .attr("id")
                .or_else(|| a.value().attr("name"))
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                return Some((name, anchor));
            }
        }
    }

    let src_sel = sel("p.src")?;
    if let Some(src_el) = el.select(&src_sel).next() {
        let a_sel = sel("a[id]")?;
        for a in src_el.select(&a_sel) {
            let text = a.text().collect::<String>().trim().to_string();
            let id = a.value().attr("id").unwrap_or("").to_string();
            if !text.is_empty() {
                return Some((text, id));
            }
        }
    }

    None
}

fn extract_decl_signature(el: ElementRef) -> Option<String> {
    let src_sel = sel("p.src")?;
    let src_el = el.select(&src_sel).next()?;

    // Get the full text, but strip the "Source" link text
    let mut text = String::new();
    for child in src_el.children() {
        if let Some(el_ref) = ElementRef::wrap(child) {
            // Skip "Source" / "#" links
            let tag = el_ref.value().name();
            if tag == "a" {
                let classes = el_ref.value().attr("class").unwrap_or("");
                if classes.contains("link") {
                    continue; // Skip source link
                }
            }
            text.push_str(&el_ref.text().collect::<String>());
        } else if let Some(text_node) = child.value().as_text() {
            text.push_str(text_node);
        }
    }

    let cleaned = text
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&");
    let trimmed = cleaned.trim().to_string();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn extract_source_url(el: ElementRef, base_url: &Url) -> Option<Url> {
    if let Some(link_sel) = sel("a.link") {
        for a in el.select(&link_sel) {
            if let Some(href) = a.value().attr("href") {
                let text = a.text().collect::<String>();
                if text.contains("Source") || text.contains("#") {
                    return base_url.join(href).ok();
                }
            }
        }
    }

    let a_sel = sel("p.src a[href]")?;
    for a in el.select(&a_sel) {
        let text = a.text().collect::<String>();
        if text.trim() == "Source" || text.trim() == "#" {
            if let Some(href) = a.value().attr("href") {
                return base_url.join(href).ok();
            }
        }
    }

    None
}

fn extract_since(el: ElementRef) -> Option<String> {
    let since_sel = sel("p.since, .since, span.since")?;
    for since_el in el.select(&since_sel) {
        let text = since_el.text().collect::<String>();
        let trimmed = text.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    None
}

// --- DocBlock parsing ---

fn parse_doc_blocks(el: ElementRef, base_url: &Url) -> Vec<DocBlock> {
    let mut blocks = Vec::new();

    for child in el.children() {
        let Some(child_el) = ElementRef::wrap(child) else {
            // Text node at top level: treat as paragraph if non-empty
            if let Some(text) = child.value().as_text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    blocks.push(DocBlock::Paragraph(vec![Inline::Text(trimmed.to_string())]));
                }
            }
            continue;
        };

        append_doc_element(&mut blocks, child_el, base_url);
    }

    blocks
}

fn append_doc_element(blocks: &mut Vec<DocBlock>, el: ElementRef, base_url: &Url) {
    let tag = el.value().name();
    match tag {
        "p" => {
            let classes = el.value().attr("class").unwrap_or("");
            if classes.contains("since") {
                return;
            }
            let inlines = parse_inlines(el, base_url);
            if !inlines.is_empty() {
                blocks.push(DocBlock::Paragraph(inlines));
            }
        }
        "pre" => {
            let code = el.text().collect::<String>();
            blocks.push(DocBlock::CodeBlock {
                language: Some("haskell".into()),
                code,
            });
        }
        "ul" => {
            let items = parse_list_items(el, base_url);
            if !items.is_empty() {
                blocks.push(DocBlock::UnorderedList(items));
            }
        }
        "ol" => {
            let items = parse_list_items(el, base_url);
            if !items.is_empty() {
                blocks.push(DocBlock::OrderedList(items));
            }
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let level = tag
                .as_bytes()
                .get(1)
                .and_then(|byte| char::from(*byte).to_digit(10))
                .map_or(1, |digit| digit as u8);
            let content = parse_inlines(el, base_url);
            blocks.push(DocBlock::Header { level, content });
        }
        "hr" => {
            blocks.push(DocBlock::HorizontalRule);
        }
        "div" => {
            let classes = el.value().attr("class").unwrap_or("");
            if classes.contains("warning") || classes.contains("note") {
                let inlines = parse_inlines(el, base_url);
                blocks.push(DocBlock::Note(inlines));
            } else {
                blocks.extend(parse_doc_blocks(el, base_url));
            }
        }
        "table" => {
            blocks.push(parse_table(el, base_url));
        }
        "dl" => {
            let items = parse_dl_items(el, base_url);
            if !items.is_empty() {
                blocks.push(DocBlock::UnorderedList(items));
            }
        }
        "blockquote" => {
            let inlines = parse_inlines(el, base_url);
            if !inlines.is_empty() {
                blocks.push(DocBlock::Note(inlines));
            }
        }
        "details" => {
            if let Some(summary_sel) = sel("summary") {
                if let Some(summary) = el.select(&summary_sel).next() {
                    let content = parse_inlines(summary, base_url);
                    if !content.is_empty() {
                        blocks.push(DocBlock::Header { level: 4, content });
                    }
                }
            }
            blocks.extend(parse_details_body(el, base_url));
        }
        _ => {
            let inlines = parse_inlines(el, base_url);
            if !inlines.is_empty() {
                blocks.push(DocBlock::Paragraph(inlines));
            }
        }
    }
}

fn parse_details_body(el: ElementRef, base_url: &Url) -> Vec<DocBlock> {
    let mut blocks = Vec::new();

    for child in el.children() {
        let Some(child_el) = ElementRef::wrap(child) else {
            continue;
        };
        if child_el.value().name() == "summary" {
            continue;
        }
        append_doc_element(&mut blocks, child_el, base_url);
    }

    blocks
}

fn parse_list_items(el: ElementRef, base_url: &Url) -> Vec<Vec<Inline>> {
    let Some(li_sel) = sel("li") else {
        return Vec::new();
    };
    el.select(&li_sel)
        .map(|li| parse_inlines(li, base_url))
        .filter(|inlines| !inlines.is_empty())
        .collect()
}

fn parse_dl_items(el: ElementRef, base_url: &Url) -> Vec<Vec<Inline>> {
    let mut items = Vec::new();
    let Some(dt_sel) = sel("dt") else {
        return items;
    };
    let Some(dd_sel) = sel("dd") else {
        return items;
    };

    let dts: Vec<_> = el.select(&dt_sel).collect();
    let dds: Vec<_> = el.select(&dd_sel).collect();

    for (i, dt) in dts.iter().enumerate() {
        let mut inlines = parse_inlines(*dt, base_url);
        if let Some(dd) = dds.get(i) {
            inlines.push(Inline::Text(" - ".into()));
            inlines.extend(parse_inlines(*dd, base_url));
        }
        if !inlines.is_empty() {
            items.push(inlines);
        }
    }

    items
}

fn parse_table(el: ElementRef, base_url: &Url) -> DocBlock {
    let Some(thead_sel) = sel("thead tr") else {
        return DocBlock::Paragraph(Vec::new());
    };
    let Some(tbody_sel) = sel("tbody tr") else {
        return DocBlock::Paragraph(Vec::new());
    };
    let Some(tr_sel) = sel("tr") else {
        return DocBlock::Paragraph(Vec::new());
    };
    let Some(th_sel) = sel("th") else {
        return DocBlock::Paragraph(Vec::new());
    };
    let Some(td_sel) = sel("td") else {
        return DocBlock::Paragraph(Vec::new());
    };

    // Extract headers from <thead> or first row with <th>
    let mut headers: Vec<Vec<Inline>> = Vec::new();
    if let Some(header_row) = el.select(&thead_sel).next() {
        headers = header_row
            .select(&th_sel)
            .map(|cell| parse_inlines(cell, base_url))
            .collect();
    }

    // Extract body rows
    let mut rows: Vec<Vec<Vec<Inline>>> = Vec::new();
    let body_rows = el.select(&tbody_sel);
    let all_rows_iter: Box<dyn Iterator<Item = ElementRef>> = if body_rows.clone().next().is_some()
    {
        Box::new(body_rows)
    } else {
        // No <tbody>, use all <tr> directly
        Box::new(el.select(&tr_sel))
    };

    for row_el in all_rows_iter {
        // If this row has <th> and we have no headers yet, use it as headers
        let ths: Vec<_> = row_el.select(&th_sel).collect();
        if !ths.is_empty() && headers.is_empty() {
            headers = ths
                .into_iter()
                .map(|cell| parse_inlines(cell, base_url))
                .collect();
            continue;
        }

        let cells: Vec<Vec<Inline>> = row_el
            .select(&td_sel)
            .map(|cell| parse_inlines(cell, base_url))
            .collect();
        if !cells.is_empty() {
            rows.push(cells);
        }
    }

    // If we got nothing useful, fall back to text
    if headers.is_empty() && rows.is_empty() {
        let text = el.text().collect::<String>();
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return DocBlock::Paragraph(vec![Inline::Text(trimmed.to_string())]);
        }
    }

    DocBlock::Table { headers, rows }
}

// --- Inline parsing ---

fn parse_inlines(el: ElementRef, base_url: &Url) -> Vec<Inline> {
    let mut inlines = Vec::new();

    for child in el.children() {
        if let Some(text_node) = child.value().as_text() {
            let text = text_node.to_string();
            if !text.is_empty() {
                inlines.push(Inline::Text(text));
            }
        } else if let Some(child_el) = ElementRef::wrap(child) {
            let tag = child_el.value().name();
            match tag {
                "code" | "tt" => {
                    let text = child_el.text().collect::<String>();
                    inlines.push(Inline::Code(text));
                }
                "a" => {
                    let text = child_el.text().collect::<String>();
                    if let Some(href) = child_el.value().attr("href") {
                        if is_module_link(href) {
                            // Module link
                            let module_name = text.trim().to_string();
                            inlines.push(Inline::ModuleLink(module_name));
                        } else if let Ok(url) = base_url.join(href) {
                            inlines.push(Inline::Link {
                                text: text.trim().to_string(),
                                url,
                            });
                        } else {
                            inlines.push(Inline::Text(text));
                        }
                    } else {
                        inlines.push(Inline::Text(text));
                    }
                }
                "em" | "i" => {
                    let text = child_el.text().collect::<String>();
                    inlines.push(Inline::Emphasis(text));
                }
                "strong" | "b" => {
                    let text = child_el.text().collect::<String>();
                    inlines.push(Inline::Bold(text));
                }
                "span" => {
                    let classes = child_el.value().attr("class").unwrap_or("");
                    if classes.contains("math") {
                        let text = child_el.text().collect::<String>();
                        inlines.push(Inline::Math(text));
                    } else {
                        // Recurse into span
                        inlines.extend(parse_inlines(child_el, base_url));
                    }
                }
                "pre" | "kbd" | "samp" | "var" => {
                    let text = child_el.text().collect::<String>();
                    inlines.push(Inline::Code(text));
                }
                "sub" => {
                    let text = child_el.text().collect::<String>();
                    inlines.push(Inline::Text(format!("_{text}")));
                }
                "sup" => {
                    let text = child_el.text().collect::<String>();
                    inlines.push(Inline::Text(format!("^{text}")));
                }
                "br" => {
                    inlines.push(Inline::Text("\n".to_string()));
                }
                "img" => {
                    let alt = child_el.value().attr("alt").unwrap_or("[image]");
                    inlines.push(Inline::Text(format!("[{alt}]")));
                }
                _ => {
                    // Recurse for unknown elements (details, summary, div, etc.)
                    inlines.extend(parse_inlines(child_el, base_url));
                }
            }
        }
    }

    inlines
}

/// Detect if a href points to a module page (e.g., "Data-Map-Strict.html")
fn is_module_link(href: &str) -> bool {
    // Module links look like: "Module-Name.html" or "../Module-Name.html"
    // But NOT "src/Module-Name.html" (source links)
    let path = href.split('#').next().unwrap_or(href);
    if path.contains("src/") || path.contains("src\\") {
        return false;
    }
    let filename = path.rsplit('/').next().unwrap_or(path);
    if !filename.ends_with(".html") {
        return false;
    }
    let name = &filename[..filename.len() - 5];
    // Module names: hyphen-separated components starting with uppercase
    // e.g. "Data-Map-Strict"
    name.split('-')
        .all(|part| part.starts_with(|c: char| c.is_ascii_uppercase()) || part.is_empty())
        && !name.is_empty()
}

fn sel(s: &str) -> Option<Selector> {
    Selector::parse(s).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_url() -> Url {
        Url::parse("https://hackage.haskell.org/package/base-4.18.0.0/docs/Data-Maybe.html")
            .unwrap()
    }

    #[test]
    fn is_module_link_basic() {
        assert!(is_module_link("Data-Map-Strict.html"));
        assert!(is_module_link("Control-Monad.html"));
        assert!(is_module_link("../Data-Map.html"));
        assert!(!is_module_link("https://example.com"));
        assert!(!is_module_link("src/Data-Map.html")); // src pages have lowercase
        assert!(!is_module_link(""));
    }

    #[test]
    fn parse_empty_html() {
        let result = parse_haddock_html("<html><body></body></html>", &test_url()).unwrap();
        assert!(result.declarations.is_empty());
    }

    #[test]
    fn parse_module_name_from_header() {
        let html = r#"<html><body>
            <div id="module-header"><p class="caption">Data.Maybe</p></div>
        </body></html>"#;
        let result = parse_haddock_html(html, &test_url()).unwrap();
        assert_eq!(result.module, "Data.Maybe");
    }

    #[test]
    fn parse_package_from_url() {
        let url = Url::parse(
            "https://hackage.haskell.org/package/containers-0.6.7/docs/Data-Map-Strict.html",
        )
        .unwrap();
        let result = parse_haddock_html("<html></html>", &url).unwrap();
        assert_eq!(result.package, "containers-0.6.7");
    }

    #[test]
    fn parse_hackage_like_fixture() {
        let html = r#"
            <html>
              <head><title>containers-0.6.7: Data.Map.Strict</title></head>
              <body>
                <div id="module-header"><p class="caption">Data.Map.Strict</p></div>
                <div id="description">
                  <div class="doc">
                    <p>Strict maps from keys to values.</p>
                    <table>
                      <thead><tr><th>Operation</th><th>Cost</th></tr></thead>
                      <tbody><tr><td><code>lookup</code></td><td>O(log n)</td></tr></tbody>
                    </table>
                  </div>
                </div>
                <div class="top">
                  <p class="src">
                    <a id="v:lookup" class="def">lookup</a> :: Ord k => k -> Map k a -> Maybe a
                    <a href="src/Data-Map-Strict.html#lookup" class="link">Source</a>
                  </p>
                  <p class="since">Since: containers-0.1</p>
                  <div class="doc">
                    <p>Look up a key in the map. See <a href="Data-Map-Lazy.html">Data.Map.Lazy</a>.</p>
                    <details>
                      <summary>Examples</summary>
                      <pre>>>> lookup 1 empty
Nothing</pre>
                    </details>
                    <dl>
                      <dt>Warning</dt>
                      <dd>Keys must be ordered.</dd>
                    </dl>
                  </div>
                </div>
              </body>
            </html>
        "#;

        let url = Url::parse(
            "https://hackage.haskell.org/package/containers-0.6.7/docs/Data-Map-Strict.html",
        )
        .unwrap();
        let doc = parse_haddock_html(html, &url).unwrap();

        assert_eq!(doc.module, "Data.Map.Strict");
        assert_eq!(doc.package, "containers-0.6.7");
        assert!(doc
            .description
            .iter()
            .any(|block| matches!(block, DocBlock::Table { headers, rows } if headers.len() == 2 && rows.len() == 1)));

        let decl = &doc.declarations[0];
        assert_eq!(decl.name, "lookup");
        assert_eq!(decl.anchor.as_deref(), Some("v:lookup"));
        assert_eq!(decl.since.as_deref(), Some("Since: containers-0.1"));
        assert!(decl
            .signature
            .as_deref()
            .is_some_and(|signature| signature.contains("Ord k => k -> Map k a -> Maybe a")));
        assert!(decl.source_url.as_ref().is_some_and(|source_url| {
            source_url
                .as_str()
                .ends_with("src/Data-Map-Strict.html#lookup")
        }));
        assert!(decl
            .doc
            .iter()
            .any(|block| matches!(block, DocBlock::Header { level: 4, .. })));
        assert!(decl.doc.iter().any(
            |block| matches!(block, DocBlock::CodeBlock { code, .. } if code.contains("Nothing"))
        ));
        assert!(decl
            .doc
            .iter()
            .any(|block| matches!(block, DocBlock::UnorderedList(items) if items.len() == 1)));
    }

    #[test]
    fn parse_hackage_like_type_fixture_with_warning_and_rich_inlines() {
        let html = include_str!("../../tests/fixtures/haddock/data_maybe_type.html");

        let doc = parse_haddock_html(html, &test_url()).unwrap();
        assert_eq!(doc.module, "Data.Maybe");
        assert_eq!(doc.package, "base-4.18.0.0");
        assert!(doc
            .description
            .iter()
            .any(|block| matches!(block, DocBlock::Paragraph(inlines) if inlines.iter().any(|inline| matches!(inline, Inline::Text(text) if text.contains("optional value"))))));

        let decl = &doc.declarations[0];
        assert_eq!(decl.name, "data Maybe");
        assert_eq!(decl.anchor.as_deref(), Some("t:Maybe"));
        assert_eq!(decl.signature.as_deref(), Some("data Maybe a"));
        assert!(decl.source_url.as_ref().is_some_and(|source_url| {
            source_url.as_str().ends_with("src/Data-Maybe.html#Maybe")
        }));

        assert!(decl.doc.iter().any(|block| {
            matches!(
                block,
                DocBlock::Note(inlines)
                    if inlines.iter().any(|inline| {
                        matches!(inline, Inline::Text(text) if text.contains("Deprecated"))
                    })
            )
        }));

        assert!(decl.doc.iter().any(|block| {
            matches!(
                block,
                DocBlock::Paragraph(inlines)
                    if inlines.iter().any(|inline| {
                        matches!(inline, Inline::ModuleLink(module) if module == "Control.Applicative")
                    }) && inlines.iter().any(|inline| {
                        matches!(
                            inline,
                            Inline::Link { text, url }
                                if text == "maybe" && url.as_str().ends_with("Data-Maybe.html#v:maybe")
                        )
                    })
            )
        }));

        assert!(decl.doc.iter().any(|block| {
            matches!(
                block,
                DocBlock::Paragraph(inlines)
                    if inlines.iter().any(|inline| matches!(inline, Inline::Emphasis(text) if text == "emphasis"))
                        && inlines.iter().any(|inline| matches!(inline, Inline::Bold(text) if text == "strength"))
                        && inlines.iter().any(|inline| matches!(inline, Inline::Math(text) if text == "x^2"))
                        && inlines.iter().any(|inline| matches!(inline, Inline::Text(text) if text.contains("_2")))
                        && inlines.iter().any(|inline| matches!(inline, Inline::Text(text) if text.contains("^n")))
            )
        }));
    }

    #[test]
    fn parse_simple_declaration() {
        let html = r#"<html><body>
            <div class="top">
                <p class="src">
                    <a id="v:fromJust" class="def">fromJust</a> :: Maybe a -> a
                </p>
                <div class="doc">
                    <p>Extract the element from a Just.</p>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        assert_eq!(result.declarations.len(), 1);

        let decl = &result.declarations[0];
        assert_eq!(decl.name, "fromJust");
        assert!(decl.signature.is_some());
        assert!(decl.signature.as_ref().unwrap().contains("Maybe a -> a"));
        assert_eq!(decl.anchor.as_deref(), Some("v:fromJust"));
        assert!(!decl.doc.is_empty());
    }

    #[test]
    fn parse_declaration_with_source_link() {
        let html = r#"<html><body>
            <div class="top">
                <p class="src">
                    <a id="v:isJust" class="def">isJust</a> :: Maybe a -> Bool
                    <a href="src/Data-Maybe.html#isJust" class="link">Source</a>
                </p>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        let decl = &result.declarations[0];
        assert_eq!(decl.name, "isJust");
        assert!(decl.source_url.is_some());
        // Signature should NOT contain "Source"
        let sig = decl.signature.as_ref().unwrap();
        assert!(!sig.contains("Source"));
    }

    #[test]
    fn parse_doc_with_code_block() {
        let html = r#"<html><body>
            <div class="top">
                <p class="src"><a id="v:foo" class="def">foo</a> :: Int</p>
                <div class="doc">
                    <p>Example:</p>
                    <pre>>>> foo 42
42</pre>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        let decl = &result.declarations[0];
        assert!(decl.doc.len() >= 2);
        assert!(matches!(decl.doc[1], DocBlock::CodeBlock { .. }));
    }

    #[test]
    fn parse_doc_with_list() {
        let html = r#"<html><body>
            <div class="top">
                <p class="src"><a id="v:bar" class="def">bar</a> :: Int</p>
                <div class="doc">
                    <ul>
                        <li>First item</li>
                        <li>Second item</li>
                    </ul>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        let decl = &result.declarations[0];
        assert!(decl
            .doc
            .iter()
            .any(|b| matches!(b, DocBlock::UnorderedList(items) if items.len() == 2)));
    }

    #[test]
    fn parse_inline_code_and_links() {
        let html = r#"<html><body>
            <div class="top">
                <p class="src"><a id="v:baz" class="def">baz</a> :: Int</p>
                <div class="doc">
                    <p>See <code>foo</code> and <a href="Data-Map.html">Data.Map</a>.</p>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        let decl = &result.declarations[0];
        if let Some(DocBlock::Paragraph(inlines)) = decl.doc.first() {
            assert!(inlines
                .iter()
                .any(|i| matches!(i, Inline::Code(s) if s == "foo")));
            assert!(inlines
                .iter()
                .any(|i| matches!(i, Inline::ModuleLink(s) if s == "Data.Map")));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn parse_multiple_declarations() {
        let html = r#"<html><body>
            <div class="top">
                <p class="src"><a id="v:foo" class="def">foo</a> :: Int</p>
            </div>
            <div class="top">
                <p class="src"><a id="v:bar" class="def">bar</a> :: Bool</p>
            </div>
            <div class="top">
                <p class="src"><a id="v:baz" class="def">baz</a> :: String</p>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        assert_eq!(result.declarations.len(), 3);
        assert_eq!(result.declarations[0].name, "foo");
        assert_eq!(result.declarations[1].name, "bar");
        assert_eq!(result.declarations[2].name, "baz");
    }

    #[test]
    fn parse_description_section() {
        let html = r#"<html><body>
            <div id="description">
                <div class="doc">
                    <p>This module provides the Maybe type.</p>
                    <p>It is very useful.</p>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        assert_eq!(result.description.len(), 2);
    }

    #[test]
    fn parse_source_html_basic() {
        let html = r#"<html><body>
            <pre>module Data.Maybe where
fromJust :: Maybe a -> a
fromJust (Just x) = x
fromJust Nothing = error "fromJust"
            </pre>
        </body></html>"#;

        let source = parse_source_html(html);
        assert!(source.contains("fromJust"));
        assert!(source.contains("Maybe a -> a"));
    }

    #[test]
    fn no_panic_on_malformed_html() {
        let html = "<div class='top'><p class='src'><a></a></div><<>>!!!";
        let result = parse_haddock_html(html, &test_url());
        assert!(result.is_ok());
    }

    #[test]
    fn parse_header_blocks() {
        let html = r#"<html><body>
            <div id="description">
                <div class="doc">
                    <h2>Overview</h2>
                    <p>Some text.</p>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        assert!(result
            .description
            .iter()
            .any(|b| matches!(b, DocBlock::Header { level: 2, .. })));
    }

    #[test]
    fn parse_table() {
        let html = r#"<html><body>
            <div class="top">
                <p class="src"><a id="v:tab" class="def">tab</a> :: Int</p>
                <div class="doc">
                    <table>
                        <thead><tr><th>Name</th><th>Type</th></tr></thead>
                        <tbody>
                            <tr><td>foo</td><td>Int</td></tr>
                            <tr><td>bar</td><td>Bool</td></tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        let decl = &result.declarations[0];
        let table = decl
            .doc
            .iter()
            .find(|b| matches!(b, DocBlock::Table { .. }));
        assert!(table.is_some(), "expected a Table block");
        if let Some(DocBlock::Table { headers, rows }) = table {
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 2);
        }
    }

    #[test]
    fn parse_table_no_thead() {
        let html = r#"<html><body>
            <div class="top">
                <p class="src"><a id="v:t2" class="def">t2</a> :: Int</p>
                <div class="doc">
                    <table>
                        <tr><th>A</th><th>B</th></tr>
                        <tr><td>1</td><td>2</td></tr>
                    </table>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        let decl = &result.declarations[0];
        let table = decl
            .doc
            .iter()
            .find(|b| matches!(b, DocBlock::Table { .. }));
        assert!(table.is_some());
        if let Some(DocBlock::Table { headers, rows }) = table {
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 1);
        }
    }

    #[test]
    fn parse_blockquote_as_note() {
        let html = r#"<html><body>
            <div id="description">
                <div class="doc">
                    <blockquote>This is a quoted note.</blockquote>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        assert!(result
            .description
            .iter()
            .any(|b| matches!(b, DocBlock::Note(_))));
    }

    #[test]
    fn parse_details_summary() {
        let html = r#"<html><body>
            <div id="description">
                <div class="doc">
                    <details>
                        <summary>Click to expand</summary>
                        <p>Hidden content here.</p>
                    </details>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        // Should have a header from summary + paragraph from content
        assert!(result.description.len() >= 2);
    }

    #[test]
    fn parse_details_summary_without_duplicate_body_paragraph() {
        let html = r#"<html><body>
            <div id="description">
                <div class="doc">
                    <details>
                        <summary>Examples</summary>
                        <p>Use this function for total inputs.</p>
                    </details>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        assert_eq!(result.description.len(), 2);
        assert!(matches!(
            &result.description[0],
            DocBlock::Header { content, .. }
                if content.iter().any(|inline| matches!(inline, Inline::Text(text) if text.contains("Examples")))
        ));
        assert!(matches!(
            &result.description[1],
            DocBlock::Paragraph(inlines)
                if inlines.iter().any(|inline| matches!(inline, Inline::Text(text) if text.contains("total inputs")))
        ));
    }

    #[test]
    fn parse_definition_list() {
        let html = r#"<html><body>
            <div id="description">
                <div class="doc">
                    <dl>
                        <dt>Term 1</dt><dd>Definition 1</dd>
                        <dt>Term 2</dt><dd>Definition 2</dd>
                    </dl>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        assert!(result
            .description
            .iter()
            .any(|b| matches!(b, DocBlock::UnorderedList(items) if items.len() == 2)));
    }

    #[test]
    fn parse_inline_img_alt() {
        let html = r#"<html><body>
            <div class="top">
                <p class="src"><a id="v:img" class="def">img</a> :: Int</p>
                <div class="doc">
                    <p>See <img alt="diagram" src="foo.png"/> for details.</p>
                </div>
            </div>
        </body></html>"#;

        let result = parse_haddock_html(html, &test_url()).unwrap();
        let decl = &result.declarations[0];
        if let Some(DocBlock::Paragraph(inlines)) = decl.doc.first() {
            assert!(inlines
                .iter()
                .any(|i| matches!(i, Inline::Text(s) if s.contains("[diagram]"))));
        }
    }
}
