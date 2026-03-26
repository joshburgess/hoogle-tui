use std::path::{Path, PathBuf};

/// Detected Haskell project info.
#[allow(dead_code)]
pub struct ProjectInfo {
    pub project_type: ProjectType,
    pub root: PathBuf,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ProjectType {
    Cabal,
    Stack,
}

/// Detect a Haskell project in the current directory or ancestors.
/// Returns the project type, root directory, and extracted dependency names.
pub fn detect_project() -> Option<ProjectInfo> {
    let cwd = std::env::current_dir().ok()?;

    // Walk up from cwd looking for project files
    let mut dir = cwd.as_path();
    loop {
        // Check for .cabal file
        if let Some(info) = try_cabal_project(dir) {
            return Some(info);
        }
        // Check for stack.yaml
        if dir.join("stack.yaml").exists() {
            // stack projects also have .cabal files, try to find one
            if let Some(info) = try_cabal_project(dir) {
                return Some(ProjectInfo {
                    project_type: ProjectType::Stack,
                    ..info
                });
            }
        }
        // Check for cabal.project
        if dir.join("cabal.project").exists() {
            if let Some(info) = try_cabal_project(dir) {
                return Some(info);
            }
        }
        dir = dir.parent()?;
    }
}

fn try_cabal_project(dir: &Path) -> Option<ProjectInfo> {
    // Find .cabal file in directory
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("cabal") {
            let contents = std::fs::read_to_string(&path).ok()?;
            let deps = extract_cabal_deps(&contents);
            return Some(ProjectInfo {
                project_type: ProjectType::Cabal,
                root: dir.to_path_buf(),
                dependencies: deps,
            });
        }
    }
    None
}

/// Extract build-depends package names from a .cabal file.
/// This is a simple parser — doesn't handle conditionals or version ranges perfectly,
/// but extracts the package names well enough for search scoping.
fn extract_cabal_deps(contents: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_build_depends = false;

    for line in contents.lines() {
        let trimmed = line.trim();

        if trimmed.to_lowercase().starts_with("build-depends:") {
            in_build_depends = true;
            // Parse deps on the same line after the colon
            let after_colon = trimmed
                .split_once(':')
                .map(|x| x.1)
                .unwrap_or("")
                .trim();
            parse_dep_list(after_colon, &mut deps);
            continue;
        }

        if in_build_depends {
            // Continuation lines are indented
            if line.starts_with(' ') || line.starts_with('\t') {
                // Check if this looks like a new field (contains a colon at a low indent)
                if trimmed.contains(':') && !trimmed.starts_with(',') && !trimmed.contains(">=") && !trimmed.contains("<=") && !trimmed.contains("==") {
                    in_build_depends = false;
                    continue;
                }
                parse_dep_list(trimmed, &mut deps);
            } else {
                in_build_depends = false;
            }
        }
    }

    deps.sort();
    deps.dedup();
    deps
}

fn parse_dep_list(text: &str, deps: &mut Vec<String>) {
    for part in text.split(',') {
        let part = part.trim().trim_start_matches(',');
        if part.is_empty() {
            continue;
        }
        // Extract package name (first word, before any version constraint)
        let name = part
            .split(|c: char| c.is_whitespace() || c == '>' || c == '<' || c == '=' || c == '^')
            .next()
            .unwrap_or("")
            .trim();
        if !name.is_empty() && name.chars().next().is_some_and(|c| c.is_alphabetic()) {
            deps.push(name.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cabal_deps() {
        let cabal = r#"
name: my-project
version: 0.1.0

library
  build-depends:
    base >= 4.14 && < 5,
    containers ^>= 0.6,
    text >= 1.2,
    aeson,
    bytestring
  exposed-modules: MyLib
"#;
        let deps = extract_cabal_deps(cabal);
        assert!(deps.contains(&"base".to_string()));
        assert!(deps.contains(&"containers".to_string()));
        assert!(deps.contains(&"text".to_string()));
        assert!(deps.contains(&"aeson".to_string()));
        assert!(deps.contains(&"bytestring".to_string()));
    }

    #[test]
    fn parse_inline_build_depends() {
        let cabal = "  build-depends: base, containers, text\n";
        let deps = extract_cabal_deps(cabal);
        assert_eq!(deps.len(), 3);
    }

    #[test]
    fn parse_complex_version_constraints() {
        let cabal = r#"
library
  build-depends:
    base ^>= 4.16,
    containers >= 0.6 && < 0.8,
    text == 2.0.*,
    bytestring >= 0.11.3.0
"#;
        let deps = extract_cabal_deps(cabal);
        assert!(deps.contains(&"base".to_string()));
        assert!(deps.contains(&"containers".to_string()));
        assert!(deps.contains(&"text".to_string()));
        assert!(deps.contains(&"bytestring".to_string()));
        assert_eq!(deps.len(), 4);
    }

    #[test]
    fn parse_multiple_build_depends_sections() {
        let cabal = r#"
library
  build-depends:
    base >= 4.14,
    containers

executable my-app
  build-depends:
    base >= 4.14,
    optparse-applicative >= 0.16

test-suite tests
  build-depends:
    base,
    hspec >= 2.7,
    QuickCheck
"#;
        let deps = extract_cabal_deps(cabal);
        assert!(deps.contains(&"base".to_string()));
        assert!(deps.contains(&"containers".to_string()));
        assert!(deps.contains(&"optparse-applicative".to_string()));
        assert!(deps.contains(&"hspec".to_string()));
        assert!(deps.contains(&"QuickCheck".to_string()));
        // base is deduplicated
        assert_eq!(deps.iter().filter(|d| d.as_str() == "base").count(), 1);
    }

    #[test]
    fn parse_empty_deps() {
        let cabal = r#"
name: empty-project
version: 0.1.0

library
  exposed-modules: Lib
"#;
        let deps = extract_cabal_deps(cabal);
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_build_depends_no_content_after_colon() {
        let cabal = r#"
library
  build-depends:
"#;
        let deps = extract_cabal_deps(cabal);
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_dep_list_empty_string() {
        let mut deps = Vec::new();
        parse_dep_list("", &mut deps);
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_dep_list_leading_comma() {
        let mut deps = Vec::new();
        parse_dep_list(", aeson, lens", &mut deps);
        assert!(deps.contains(&"aeson".to_string()));
        assert!(deps.contains(&"lens".to_string()));
    }

    #[test]
    fn parse_dep_list_with_version_constraints() {
        let mut deps = Vec::new();
        parse_dep_list("base >= 4.14 && < 5, containers ^>= 0.6", &mut deps);
        assert!(deps.contains(&"base".to_string()));
        assert!(deps.contains(&"containers".to_string()));
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn parse_dep_list_whitespace_only() {
        let mut deps = Vec::new();
        parse_dep_list("   ", &mut deps);
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_dep_list_single_dep_no_version() {
        let mut deps = Vec::new();
        parse_dep_list("aeson", &mut deps);
        assert_eq!(deps, vec!["aeson".to_string()]);
    }

    #[test]
    fn deps_are_sorted_and_deduped() {
        let cabal = r#"
library
  build-depends:
    zebra,
    alpha,
    alpha,
    middle
"#;
        let deps = extract_cabal_deps(cabal);
        assert_eq!(deps, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn parse_dep_list_ignores_non_alphabetic_start() {
        let mut deps = Vec::new();
        parse_dep_list("123bad, good-package", &mut deps);
        // 123bad starts with a digit, should be skipped
        assert_eq!(deps, vec!["good-package".to_string()]);
    }
}
