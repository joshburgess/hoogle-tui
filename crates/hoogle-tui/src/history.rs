use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;

const MAX_HISTORY: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub query: String,
    pub timestamp: DateTime<Utc>,
    pub result_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchHistory {
    entries: VecDeque<HistoryEntry>,
    #[serde(skip)]
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadStatus {
    Loaded,
    Missing,
    Unreadable(String),
    Corrupt(String),
}

impl SearchHistory {
    pub fn load_with_status(path: PathBuf) -> (Self, LoadStatus) {
        match std::fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<SearchHistory>(&contents) {
                Ok(mut history) => {
                    history.path = path;
                    (history, LoadStatus::Loaded)
                }
                Err(e) => (Self::empty(path), LoadStatus::Corrupt(e.to_string())),
            },
            Err(e) if e.kind() == ErrorKind::NotFound => (Self::empty(path), LoadStatus::Missing),
            Err(e) => (Self::empty(path), LoadStatus::Unreadable(e.to_string())),
        }
    }

    fn empty(path: PathBuf) -> Self {
        Self {
            entries: VecDeque::new(),
            path,
        }
    }

    pub fn add(&mut self, query: &str, result_count: usize) {
        let query = query.trim().to_string();
        if query.is_empty() {
            return;
        }

        // Remove existing entry with the same query (dedup)
        self.entries.retain(|e| e.query != query);

        // Add to front
        self.entries.push_front(HistoryEntry {
            query,
            timestamp: Utc::now(),
            result_count,
        });

        // Enforce max size
        while self.entries.len() > MAX_HISTORY {
            self.entries.pop_back();
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.entries.len() {
            self.entries.remove(index);
        }
    }

    pub fn entries(&self) -> &VecDeque<HistoryEntry> {
        &self.entries
    }

    pub fn try_save(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&self.path, json)
    }
}

pub fn history_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hoogle-tui")
        .join("history.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_history() -> (TempDir, SearchHistory) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("history.json");
        (dir, SearchHistory::load_with_status(path).0)
    }

    #[test]
    fn add_and_retrieve() {
        let (_dir, mut h) = temp_history();
        h.add("map", 42);
        h.add("filter", 10);
        assert_eq!(h.entries().len(), 2);
        assert_eq!(h.entries()[0].query, "filter");
        assert_eq!(h.entries()[1].query, "map");
    }

    #[test]
    fn deduplication() {
        let (_dir, mut h) = temp_history();
        h.add("map", 42);
        h.add("filter", 10);
        h.add("map", 50);
        assert_eq!(h.entries().len(), 2);
        assert_eq!(h.entries()[0].query, "map");
        assert_eq!(h.entries()[0].result_count, 50);
    }

    #[test]
    fn max_size() {
        let (_dir, mut h) = temp_history();
        for i in 0..600 {
            h.add(&format!("query_{i}"), i);
        }
        assert_eq!(h.entries().len(), MAX_HISTORY);
    }

    #[test]
    fn persistence_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("history.json");

        {
            let mut h = SearchHistory::load_with_status(path.clone()).0;
            h.add("map", 42);
            h.add("filter", 10);
            h.try_save().unwrap();
        }

        let h = SearchHistory::load_with_status(path).0;
        assert_eq!(h.entries().len(), 2);
        assert_eq!(h.entries()[0].query, "filter");
    }

    #[test]
    fn load_reports_missing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("history.json");

        let (h, status) = SearchHistory::load_with_status(path);

        assert_eq!(status, LoadStatus::Missing);
        assert!(h.entries().is_empty());
    }

    #[test]
    fn load_reports_corrupt_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("history.json");
        std::fs::write(&path, "{not json").unwrap();

        let (h, status) = SearchHistory::load_with_status(path);

        assert!(matches!(status, LoadStatus::Corrupt(_)));
        assert!(h.entries().is_empty());
    }

    #[test]
    fn remove_entry() {
        let (_dir, mut h) = temp_history();
        h.add("a", 1);
        h.add("b", 2);
        h.add("c", 3);
        h.remove(1); // remove "b"
        assert_eq!(h.entries().len(), 2);
        assert_eq!(h.entries()[0].query, "c");
        assert_eq!(h.entries()[1].query, "a");
    }

    #[test]
    fn empty_query_ignored() {
        let (_dir, mut h) = temp_history();
        h.add("", 0);
        h.add("   ", 0);
        assert!(h.entries().is_empty());
    }

    #[test]
    fn save_reports_filesystem_errors() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("not-a-dir").join("history.json");
        std::fs::write(dir.path().join("not-a-dir"), "file").unwrap();

        let mut h = SearchHistory::load_with_status(path).0;
        h.add("map", 42);

        assert!(h.try_save().is_err());
    }
}
