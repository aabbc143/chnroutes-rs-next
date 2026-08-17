use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const DEFAULT_CACHE_MAX_AGE: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone)]
pub struct BgpCache {
    path: PathBuf,
    max_age: Duration,
}

impl BgpCache {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            max_age: DEFAULT_CACHE_MAX_AGE,
        }
    }

    pub fn with_max_age(path: impl Into<PathBuf>, max_age: Duration) -> Self {
        Self {
            path: path.into(),
            max_age,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_fresh(&self) -> std::io::Result<bool> {
        if !self.path.exists() {
            return Ok(false);
        }

        let metadata = fs::metadata(&self.path)?;
        let modified = metadata.modified()?;
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default();

        Ok(age < self.max_age)
    }

    pub fn load(&self) -> std::io::Result<Option<String>> {
        if !self.is_fresh()? {
            return Ok(None);
        }

        Ok(Some(fs::read_to_string(&self.path)?))
    }

    pub fn save(&self, content: &str) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(&self.path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_cache_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "chnroutes-bgp-cache-{}",
            std::process::id()
        ))
    }

    #[test]
    fn test_missing_cache_is_not_fresh() {
        let path = temp_cache_path();

        let _ = fs::remove_file(&path);

        let cache = BgpCache::new(&path);

        assert!(!cache.is_fresh().unwrap());
        assert_eq!(cache.load().unwrap(), None);
    }

    #[test]
    fn test_save_and_load_fresh_cache() {
        let path = temp_cache_path();

        let _ = fs::remove_file(&path);

        let cache = BgpCache::with_max_age(&path, Duration::from_secs(60));

        cache.save("test-content").unwrap();

        assert!(cache.is_fresh().unwrap());
        assert_eq!(
            cache.load().unwrap(),
            Some("test-content".to_string())
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_expired_cache_is_not_loaded() {
        let path = temp_cache_path();

        let _ = fs::remove_file(&path);

        let cache = BgpCache::with_max_age(&path, Duration::ZERO);

        cache.save("expired-content").unwrap();

        assert!(!cache.is_fresh().unwrap());
        assert_eq!(cache.load().unwrap(), None);

        let _ = fs::remove_file(&path);
    }
}
