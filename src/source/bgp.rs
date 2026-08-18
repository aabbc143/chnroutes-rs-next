use serde::Deserialize;

use super::bgp_cache::BgpCache;

const BGP_TABLE_URL: &str = "https://bgp.tools/table.jsonl";
const BGP_USER_AGENT: &str = "chnroutes-rs-next/0.2.0";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BgpTableRecord {
    #[serde(rename = "CIDR")]
    pub cidr: String,

    #[serde(rename = "ASN")]
    pub asn: u32,

    #[serde(rename = "Hits")]
    pub hits: u64,
}

pub fn parse_table_jsonl(content: &str) -> crate::error::Result<Vec<BgpTableRecord>> {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn fetch_table_jsonl() -> crate::error::Result<String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(BGP_USER_AGENT)
        .build()?;

    let response = client.get(BGP_TABLE_URL).send()?;

    let response = response.error_for_status()?;

    Ok(response.text()?)
}

pub fn fetch_table_jsonl_cached(cache: &BgpCache) -> crate::error::Result<String> {
    if let Some(content) = cache.load()? {
        return Ok(content);
    }

    let content = fetch_table_jsonl()?;

    cache.save(&content)?;

    Ok(content)
}

pub fn fetch_records_cached(cache: &BgpCache) -> crate::error::Result<Vec<BgpTableRecord>> {
    let content = fetch_table_jsonl_cached(cache)?;
    parse_table_jsonl(&content)
}

pub fn fetch_records() -> crate::error::Result<Vec<BgpTableRecord>> {
    let cache = BgpCache::from_default_path()?;
    fetch_records_cached(&cache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    fn temp_cache_path(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "chnroutes-bgp-provider-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        std::env::temp_dir().join(unique)
    }

    #[test]
    fn test_cached_fetch_uses_fresh_cache() {
        let path = temp_cache_path("fresh");
        let _ = fs::remove_file(&path);

        let cache = BgpCache::with_max_age(&path, Duration::from_secs(60));

        let content = r#"{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":1234}
"#;

        cache.save(content).unwrap();

        let loaded = fetch_table_jsonl_cached(&cache).unwrap();

        assert_eq!(loaded, content);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_cached_records_are_parsed() {
        let path = temp_cache_path("records");
        let _ = fs::remove_file(&path);

        let cache = BgpCache::with_max_age(&path, Duration::from_secs(60));

        let content = r#"{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":1234}
{"CIDR":"117.128.0.0/10","ASN":9808,"Hits":5678}
"#;

        cache.save(content).unwrap();

        let records = fetch_records_cached(&cache).unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].asn, 37963);
        assert_eq!(records[1].asn, 9808);

        let _ = fs::remove_file(&path);
    }
}
