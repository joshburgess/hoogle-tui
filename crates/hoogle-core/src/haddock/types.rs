use url::Url;

#[derive(Debug, Clone)]
pub struct HaddockDoc {
    pub module: String,
    pub package: String,
    pub description: Vec<DocBlock>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub signature: Option<String>,
    pub doc: Vec<DocBlock>,
    pub since: Option<String>,
    pub source_url: Option<Url>,
    pub anchor: Option<String>,
}

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
}

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
