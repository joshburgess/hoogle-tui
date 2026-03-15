use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub module: Option<String>,
    pub package: Option<String>,
    pub signature: Option<String>,
    pub doc_url: Option<Url>,
    pub added: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookmarkStore {
    bookmarks: Vec<Bookmark>,
    #[serde(skip)]
    path: PathBuf,
}

impl BookmarkStore {
    pub fn load(path: PathBuf) -> Self {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(mut store) = serde_json::from_str::<BookmarkStore>(&contents) {
                store.path = path;
                return store;
            }
        }
        Self {
            bookmarks: Vec::new(),
            path,
        }
    }

    pub fn add(&mut self, bookmark: Bookmark) {
        // Deduplicate by name + module
        self.bookmarks
            .retain(|b| !(b.name == bookmark.name && b.module == bookmark.module));
        self.bookmarks.insert(0, bookmark);
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.bookmarks.len() {
            self.bookmarks.remove(index);
        }
    }

    pub fn bookmarks(&self) -> &[Bookmark] {
        &self.bookmarks
    }

    pub fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&self.path, json);
        }
    }
}

pub fn bookmarks_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hoogle-tui")
        .join("bookmarks.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_store() -> (TempDir, BookmarkStore) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bookmarks.json");
        (dir, BookmarkStore::load(path))
    }

    fn make_bookmark(name: &str, module: Option<&str>) -> Bookmark {
        Bookmark {
            name: name.into(),
            module: module.map(String::from),
            package: None,
            signature: None,
            doc_url: None,
            added: Utc::now(),
        }
    }

    #[test]
    fn add_and_retrieve() {
        let (_dir, mut store) = temp_store();
        store.add(make_bookmark("map", Some("Data.Map")));
        store.add(make_bookmark("filter", Some("Data.List")));
        assert_eq!(store.bookmarks().len(), 2);
        assert_eq!(store.bookmarks()[0].name, "filter");
    }

    #[test]
    fn deduplication() {
        let (_dir, mut store) = temp_store();
        store.add(make_bookmark("map", Some("Data.Map")));
        store.add(make_bookmark("map", Some("Data.Map")));
        assert_eq!(store.bookmarks().len(), 1);
    }

    #[test]
    fn remove_bookmark() {
        let (_dir, mut store) = temp_store();
        store.add(make_bookmark("a", None));
        store.add(make_bookmark("b", None));
        store.remove(0);
        assert_eq!(store.bookmarks().len(), 1);
        assert_eq!(store.bookmarks()[0].name, "a");
    }

    #[test]
    fn persistence_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bookmarks.json");

        {
            let mut store = BookmarkStore::load(path.clone());
            store.add(make_bookmark("map", Some("Data.Map")));
            store.save();
        }

        let store = BookmarkStore::load(path);
        assert_eq!(store.bookmarks().len(), 1);
        assert_eq!(store.bookmarks()[0].name, "map");
    }
}
