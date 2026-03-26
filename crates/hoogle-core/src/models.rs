use serde::{Deserialize, Serialize};
use std::fmt;
use url::Url;

/// A single result returned from a Hoogle search query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The name of the function, type, class, or module.
    pub name: String,
    /// The module this result belongs to, if any.
    pub module: Option<ModulePath>,
    /// The package this result belongs to, if any.
    pub package: Option<PackageInfo>,
    /// The Haskell type signature, if applicable.
    pub signature: Option<String>,
    /// URL to the Haddock documentation page.
    pub doc_url: Option<Url>,
    /// A brief excerpt from the documentation.
    pub short_doc: Option<String>,
    /// The kind of Haskell entity this result represents.
    pub result_kind: ResultKind,
}

/// A Haskell module path represented as a list of components (e.g., `["Data", "Map", "Strict"]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModulePath(pub Vec<String>);

impl ModulePath {
    pub fn as_dotted(&self) -> String {
        self.0.join(".")
    }
}

impl fmt::Display for ModulePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_dotted())
    }
}

/// Metadata about a Hackage package, including its name and optional version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: Option<String>,
}

impl fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version {
            Some(v) => write!(f, "{}-{}", self.name, v),
            None => write!(f, "{}", self.name),
        }
    }
}

/// The kind of Haskell entity a search result represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultKind {
    Function,
    TypeAlias,
    DataType,
    Newtype,
    Class,
    Module,
    Package,
}

impl fmt::Display for ResultKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResultKind::Function => write!(f, "function"),
            ResultKind::TypeAlias => write!(f, "type"),
            ResultKind::DataType => write!(f, "data"),
            ResultKind::Newtype => write!(f, "newtype"),
            ResultKind::Class => write!(f, "class"),
            ResultKind::Module => write!(f, "module"),
            ResultKind::Package => write!(f, "package"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_path_display() {
        let mp = ModulePath(vec!["Data".into(), "Map".into(), "Strict".into()]);
        assert_eq!(mp.to_string(), "Data.Map.Strict");
    }

    #[test]
    fn module_path_single() {
        let mp = ModulePath(vec!["Prelude".into()]);
        assert_eq!(mp.to_string(), "Prelude");
    }

    #[test]
    fn module_path_empty() {
        let mp = ModulePath(vec![]);
        assert_eq!(mp.to_string(), "");
    }

    #[test]
    fn package_info_with_version() {
        let pi = PackageInfo {
            name: "containers".into(),
            version: Some("0.6.7".into()),
        };
        assert_eq!(pi.to_string(), "containers-0.6.7");
    }

    #[test]
    fn package_info_without_version() {
        let pi = PackageInfo {
            name: "base".into(),
            version: None,
        };
        assert_eq!(pi.to_string(), "base");
    }

    #[test]
    fn result_kind_display() {
        assert_eq!(ResultKind::Function.to_string(), "function");
        assert_eq!(ResultKind::DataType.to_string(), "data");
        assert_eq!(ResultKind::Class.to_string(), "class");
        assert_eq!(ResultKind::Module.to_string(), "module");
    }

    #[test]
    fn search_result_serialization_roundtrip() {
        let result = SearchResult {
            name: "lookup".into(),
            module: Some(ModulePath(vec![
                "Data".into(),
                "Map".into(),
                "Strict".into(),
            ])),
            package: Some(PackageInfo {
                name: "containers".into(),
                version: Some("0.6.7".into()),
            }),
            signature: Some("Ord k => k -> Map k a -> Maybe a".into()),
            doc_url: Some(Url::parse("https://hackage.haskell.org/package/containers-0.6.7/docs/Data-Map-Strict.html#v:lookup").unwrap()),
            short_doc: Some("O(log n). Look up the value at a key in the map.".into()),
            result_kind: ResultKind::Function,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: SearchResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "lookup");
        assert_eq!(deserialized.module.unwrap().to_string(), "Data.Map.Strict");
        assert_eq!(
            deserialized.package.unwrap().to_string(),
            "containers-0.6.7"
        );
        assert_eq!(deserialized.result_kind, ResultKind::Function);
    }
}
