use std::{env, env::temp_dir, io, path::PathBuf, time::Duration};

use crate::error::CacheError;

pub const CACHE_TTL_SECS: u64 = 7 * 24 * 60 * 60; // 7 天缓存策略
pub const CACHE_TTL: Duration = Duration::from_secs(CACHE_TTL_SECS);

/// 带过期时间的本地文件缓存器，用于暂存 IP 数据源文件
pub struct Cache {
    name: String,
    expire_time: Duration,
}

impl Cache {
    pub fn new(name: impl AsRef<str>, expire_time: Duration) -> Self {
        Self {
            name: name.as_ref().to_string(),
            expire_time,
        }
    }

    /// 获取缓存文件的物理路径
    pub fn get_path(&self) -> PathBuf {
        temp_dir().join(env!("CARGO_PKG_NAME")).join(&self.name)
    }

    /// 检查缓存文件是否存在或已过期
    pub fn is_expired(&self) -> Result<bool, CacheError> {
        let path = self.get_path();
        if !path.exists() {
            return Ok(true);
        }
        let metadata = std::fs::metadata(&path)?;
        let last_modified = metadata.modified()?;
        Ok(last_modified.elapsed().unwrap_or(self.expire_time) >= self.expire_time)
    }

    /// 将字节数据写入本地缓存
    pub fn save<'a>(&self, bytes: &'a [u8]) -> io::Result<&'a [u8]> {
        let path = self.get_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)?;
        Ok(bytes)
    }

    /// 从缓存加载数据；若文件不存在或已过期则返回 [`None`]
    pub fn load(&self) -> std::result::Result<Option<Vec<u8>>, CacheError> {
        if !self.is_expired()? {
            let path = self.get_path();
            return Ok(Some(std::fs::read(path)?));
        }
        Ok(None)
    }

    pub fn save_str<'a>(&self, s: &'a str) -> std::result::Result<&'a str, CacheError> {
        self.save(s.as_bytes())?;
        Ok(s)
    }

    #[allow(unused)]
    pub fn remove(&self) -> std::result::Result<(), CacheError> {
        let path = self.get_path();
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache() {
        let cache = Cache::new("test", Duration::from_millis(20));
        let path = cache.get_path();
        assert!(!path.exists());
        assert!(cache.is_expired().unwrap());

        cache.save_str("test").unwrap();
        assert!(path.exists());
        assert!(!cache.is_expired().unwrap());
        assert_eq!(cache.load().unwrap().unwrap(), "test".as_bytes());

        std::thread::sleep(Duration::from_millis(25));
        assert!(cache.is_expired().unwrap());
        assert!(cache.load().unwrap().is_none());

        cache.remove().unwrap();
    }
}
