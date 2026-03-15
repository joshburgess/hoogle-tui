use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::fs;

#[derive(Debug, Clone)]
pub struct DiskCache {
    base_dir: PathBuf,
    ttl: Duration,
    max_size_bytes: u64,
}

impl DiskCache {
    pub fn new(base_dir: PathBuf, ttl_hours: u64, max_size_mb: u64) -> Self {
        Self {
            base_dir,
            ttl: Duration::from_secs(ttl_hours * 3600),
            max_size_bytes: max_size_mb * 1024 * 1024,
        }
    }

    fn cache_path(&self, key: &str) -> PathBuf {
        let hash = hex_hash(key);
        let subdir = &hash[..2];
        self.base_dir.join(subdir).join(&hash)
    }

    fn meta_path(&self, key: &str) -> PathBuf {
        let mut p = self.cache_path(key);
        p.set_extension("meta");
        p
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.cache_path(key);
        let meta_path = self.meta_path(key);

        // Check if file exists and is not expired
        let meta = fs::metadata(&meta_path).await.ok()?;
        let modified = meta.modified().ok()?;
        if SystemTime::now().duration_since(modified).ok()? > self.ttl {
            tracing::debug!("cache expired for key: {key}");
            return None;
        }

        fs::read(&path).await.ok()
    }

    pub async fn get_string(&self, key: &str) -> Option<String> {
        let data = self.get(key).await?;
        String::from_utf8(data).ok()
    }

    /// Get even if expired (for fallback on network failure).
    pub async fn get_stale(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.cache_path(key);
        fs::read(&path).await.ok()
    }

    pub async fn put(&self, key: &str, data: &[u8]) -> Result<(), std::io::Error> {
        let path = self.cache_path(key);
        let meta_path = self.meta_path(key);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&path, data).await?;
        // Write meta file (just touch it for timestamp)
        fs::write(&meta_path, key.as_bytes()).await?;

        Ok(())
    }

    pub async fn put_string(&self, key: &str, data: &str) -> Result<(), std::io::Error> {
        self.put(key, data.as_bytes()).await
    }

    #[allow(dead_code)]
    pub async fn invalidate(&self, key: &str) -> Result<(), std::io::Error> {
        let path = self.cache_path(key);
        let meta_path = self.meta_path(key);
        let _ = fs::remove_file(&path).await;
        let _ = fs::remove_file(&meta_path).await;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn clear(&self) -> Result<(), std::io::Error> {
        if self.base_dir.exists() {
            fs::remove_dir_all(&self.base_dir).await?;
        }
        fs::create_dir_all(&self.base_dir).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn prune(&self) -> Result<(), std::io::Error> {
        let mut total_size: u64 = 0;
        let mut entries: Vec<(PathBuf, SystemTime, u64)> = Vec::new();

        let mut stack = vec![self.base_dir.clone()];
        while let Some(dir) = stack.pop() {
            let mut read_dir = match fs::read_dir(&dir).await {
                Ok(rd) => rd,
                Err(_) => continue,
            };
            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_none() {
                    // Data files (no extension)
                    if let Ok(meta) = fs::metadata(&path).await {
                        let size = meta.len();
                        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                        total_size += size;
                        entries.push((path, modified, size));
                    }
                }
            }
        }

        // Remove expired entries
        let now = SystemTime::now();
        for (path, modified, size) in &entries {
            if now.duration_since(*modified).unwrap_or_default() > self.ttl {
                let _ = fs::remove_file(&path).await;
                let mut meta = path.clone();
                meta.set_extension("meta");
                let _ = fs::remove_file(&meta).await;
                total_size -= size;
            }
        }

        // If still over max size, remove oldest first
        if total_size > self.max_size_bytes {
            let mut remaining: Vec<_> =
                entries.into_iter().filter(|(p, _, _)| p.exists()).collect();
            remaining.sort_by_key(|(_, modified, _)| *modified);

            for (path, _, size) in remaining {
                if total_size <= self.max_size_bytes {
                    break;
                }
                let _ = fs::remove_file(&path).await;
                let mut meta = path;
                meta.set_extension("meta");
                let _ = fs::remove_file(&meta).await;
                total_size -= size;
            }
        }

        Ok(())
    }
}

fn hex_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn put_and_get_roundtrip() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), 1, 100);

        cache.put("test_key", b"hello world").await.unwrap();
        let data = cache.get("test_key").await.unwrap();
        assert_eq!(data, b"hello world");
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), 1, 100);

        assert!(cache.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn string_roundtrip() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), 1, 100);

        cache
            .put_string("html_key", "<html>test</html>")
            .await
            .unwrap();
        let data = cache.get_string("html_key").await.unwrap();
        assert_eq!(data, "<html>test</html>");
    }

    #[tokio::test]
    async fn invalidate_removes_entry() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), 1, 100);

        cache.put("key", b"data").await.unwrap();
        assert!(cache.get("key").await.is_some());

        cache.invalidate("key").await.unwrap();
        assert!(cache.get("key").await.is_none());
    }

    #[tokio::test]
    async fn stale_returns_expired_content() {
        let dir = TempDir::new().unwrap();
        // TTL of 0 hours means everything is immediately expired
        let cache = DiskCache::new(dir.path().to_path_buf(), 0, 100);

        cache.put("key", b"stale data").await.unwrap();
        // get() should return None (expired)
        // But we need a small delay for the TTL=0 to actually expire
        tokio::time::sleep(Duration::from_millis(10)).await;
        // get_stale() should still return the data
        let data = cache.get_stale("key").await.unwrap();
        assert_eq!(data, b"stale data");
    }
}
