use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const DEFAULT_CACHE_MAX_AGE: Duration = Duration::from_secs(30 * 60);
const CACHE_FILE_NAME: &str = "bgp-table.jsonl";

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

    pub fn default_path() -> io::Result<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            if let Some(base) = std::env::var_os("LOCALAPPDATA") {
                return Ok(PathBuf::from(base).join("chnroutes").join(CACHE_FILE_NAME));
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(home) = std::env::var_os("HOME") {
                return Ok(PathBuf::from(home)
                    .join("Library")
                    .join("Caches")
                    .join("chnroutes")
                    .join(CACHE_FILE_NAME));
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            if let Some(base) = std::env::var_os("XDG_CACHE_HOME") {
                return Ok(PathBuf::from(base).join("chnroutes").join(CACHE_FILE_NAME));
            }

            if let Some(home) = std::env::var_os("HOME") {
                return Ok(PathBuf::from(home)
                    .join(".cache")
                    .join("chnroutes")
                    .join(CACHE_FILE_NAME));
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "cannot determine cache directory",
        ))
    }

    pub fn default() -> io::Result<Self> {
        Ok(Self::new(Self::default_path()?))
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

    pub fn is_fresh(&self) -> io::Result<bool> {
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

    pub fn load(&self) -> io::Result<Option<String>> {
        if !self.is_fresh()? {
            return Ok(None);
        }

        Ok(Some(fs::read_to_string(&self.path)?))
    }

    pub fn save(&self, content: &str) -> io::Result<()> {
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

    fn temp_cache_path(name: &str) -> PathBuf {
        let unique = format!(
            "chnroutes-bgp-cache-{}-{}-{}",
            name,
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        std::env::temp_dir().join(unique)
    }

    #[test]
    fn test_missing_cache_is_not_fresh() {
        let path = temp_cache_path("missing");

        let _ = fs::remove_file(&path);

        let cache = BgpCache::new(&path);

        assert!(!cache.is_fresh().unwrap());
        assert_eq!(cache.load().unwrap(), None);
    }

    #[test]
    fn test_save_and_load_fresh_cache() {
        let path = temp_cache_path("fresh");

        let _ = fs::remove_file(&path);

        let cache = BgpCache::with_max_age(&path, Duration::from_secs(60));

        cache.save("test-content").unwrap();

        assert!(cache.is_fresh().unwrap());
        assert_eq!(cache.load().unwrap(), Some("test-content".to_string()));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_expired_cache_is_not_loaded() {
        let path = temp_cache_path("expired");

        let _ = fs::remove_file(&path);

        let cache = BgpCache::with_max_age(&path, Duration::from_secs(0));

        cache.save("expired-content").unwrap();

        std::thread::sleep(Duration::from_millis(10));

        assert!(!cache.is_fresh().unwrap());
        assert_eq!(cache.load().unwrap(), None);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_default_path_has_expected_file_name() {
        let path = BgpCache::default_path().unwrap();

        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some(CACHE_FILE_NAME)
        );
    }

    #[test]
    fn test_default_cache_can_be_created() {
        let cache = BgpCache::default().unwrap();

        assert_eq!(
            cache.path().file_name().and_then(|name| name.to_str()),
            Some(CACHE_FILE_NAME)
        );
    }
}
