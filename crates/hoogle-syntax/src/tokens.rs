/// A lexical token produced by the Haskell tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Keyword(String),
    TypeConstructor(String),
    TypeVariable(String),
    Operator(String),
    QualifiedName(String),
    Punctuation(char),
    StringLiteral(String),
    NumericLiteral(String),
    Whitespace(usize),
    Comment(String),
    Pragma(String),
    Unknown(String),
}
