use url::Url;

/// A parsed Haddock documentation page for a Haskell module.
#[derive(Debug, Clone)]
pub struct HaddockDoc {
    pub module: String,
    pub package: String,
    pub description: Vec<DocBlock>,
    pub declarations: Vec<Declaration>,
}

/// A single exported declaration (function, type, class, etc.) within a module.
#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub signature: Option<String>,
    pub doc: Vec<DocBlock>,
    pub since: Option<String>,
    pub source_url: Option<Url>,
    pub anchor: Option<String>,
}

/// A block-level element in Haddock documentation (paragraph, code block, list, etc.).
#[derive(Debug, Clone)]
pub enum DocBlock {
    Paragraph(Vec<Inline>),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    UnorderedList(Vec<Vec<Inline>>),
    OrderedList(Vec<Vec<Inline>>),
    Header {
        level: u8,
        content: Vec<Inline>,
    },
    HorizontalRule,
    Note(Vec<Inline>),
    /// An HTML table parsed into rows of cells, each cell containing inline content.
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
}

/// An inline element within documentation text (plain text, code, links, emphasis, etc.).
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Code(String),
    Link { text: String, url: Url },
    ModuleLink(String),
    Emphasis(String),
    Bold(String),
    Math(String),
}
