use std::{io::Cursor, net::IpAddr, str::FromStr, time::Duration};

use ipnet::IpNet;
use log::{info, warn};

use crate::cache::{Cache, CACHE_TTL};

const CACHE_NAME: &str = "apnic";
const APNIC_URL: &str = "https://ftp.apnic.net/apnic/stats/apnic/delegated-apnic-latest";

/// Fetch CN IPv4 data from APNIC, using a 7-day cache.
pub fn fetch_ip_data() -> crate::error::Result<Vec<IpNet>> {
    fetch_ip_data_internal(false)
}

/// Force refresh CN IPv4 data from APNIC, bypassing the local cache.
pub fn fetch_ip_data_force() -> crate::error::Result<Vec<IpNet>> {
    fetch_ip_data_internal(true)
}

fn fetch_ip_data_internal(force_refresh: bool) -> crate::error::Result<Vec<IpNet>> {
    let cache = Cache::new(CACHE_NAME, CACHE_TTL);

    if !force_refresh {
        if let Some(data) = cache.load()? {
            info!("Loading APNIC data from cache ...");

            let content = String::from_utf8(data)?;
            return Ok(parse_ip_data(&content));
        }
    }

    if force_refresh {
        info!("Force fetching APNIC data ...");
    } else {
        info!("Fetching APNIC data ...");
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    match client.get(APNIC_URL).send() {
        Ok(response) => match response.error_for_status() {
            Ok(response) => match response.text() {
                Ok(data) => {
                    info!("Fetching APNIC data done");
                    cache.save_str(&data)?;
                    Ok(parse_ip_data(&data))
                }
                Err(error) => {
                    warn!("Failed to read APNIC response: {error}");
                    load_builtin_data()
                }
            },
            Err(error) => {
                warn!("APNIC returned an unsuccessful HTTP status: {error}");
                load_builtin_data()
            }
        },
        Err(error) => {
            warn!("Failed to fetch APNIC data: {error}");
            load_builtin_data()
        }
    }
}

/// Load the APNIC snapshot embedded in the binary.
///
/// This is only a fallback and must not be written to the normal cache.
fn load_builtin_data() -> crate::error::Result<Vec<IpNet>> {
    warn!("Using built-in APNIC data as fallback");

    let compressed_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/apnic.zst"));

    let data = zstd::stream::decode_all(Cursor::new(compressed_bytes))?;
    let content = String::from_utf8(data)?;

    Ok(parse_ip_data(&content))
}

/// Parse APNIC delegated statistics and return CN IPv4 networks.
///
/// IPv6 records are intentionally ignored because chnroutes is designed
/// for IPv4 split tunneling.
pub fn parse_ip_data(content: &str) -> Vec<IpNet> {
    content
        .lines()
        .filter_map(parse_record)
        .filter(|net| matches!(net, IpNet::V4(_)))
        .collect()
}

fn parse_record(line: &str) -> Option<IpNet> {
    let fields: Vec<&str> = line.split('|').collect();

    if fields.len() < 5 {
        return None;
    }

    if fields[0] != "apnic" || fields[1] != "CN" {
        return None;
    }

    match fields[2] {
        "ipv4" => parse_ipv4_record(fields[3], fields[4]),
        // Ignore IPv6 routes.
        "ipv6" => None,
        _ => None,
    }
}

fn parse_ipv4_record(address: &str, count: &str) -> Option<IpNet> {
    let ip = IpAddr::from_str(address).ok()?;

    if !ip.is_ipv4() {
        return None;
    }

    let count = count.parse::<u32>().ok()?;

    if count == 0 || !count.is_power_of_two() {
        return None;
    }

    let host_bits = count.trailing_zeros();

    if host_bits > 32 {
        return None;
    }

    let prefix_len = 32 - host_bits as u8;

    IpNet::new(ip, prefix_len).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ipv4_record() {
        assert_eq!(
            parse_ipv4_record("1.0.1.0", "256"),
            Some(IpNet::from_str("1.0.1.0/24").unwrap())
        );

        assert_eq!(
            parse_ipv4_record("1.0.2.0", "512"),
            Some(IpNet::from_str("1.0.2.0/23").unwrap())
        );
    }

    #[test]
    fn test_parse_ipv4_record_invalid_count() {
        assert_eq!(parse_ipv4_record("1.0.1.0", "255"), None);
        assert_eq!(parse_ipv4_record("1.0.1.0", "0"), None);
    }

    #[test]
    fn test_parse_ip_data_ipv4_only() {
        let content = "\
apnic|CN|ipv4|1.0.1.0|256|20110101|allocated
apnic|CN|ipv4|1.0.2.0|512|20110101|allocated
apnic|CN|ipv6|2400:da00::|32|20110101|allocated
apnic|US|ipv4|1.1.1.0|256|20110101|allocated
invalid
";

        let results = parse_ip_data(content);

        assert_eq!(results.len(), 2);

        assert_eq!(results[0], IpNet::from_str("1.0.1.0/24").unwrap());

        assert_eq!(results[1], IpNet::from_str("1.0.2.0/23").unwrap());
    }

    #[test]
    fn test_parse_ip_data_from_fixture() {
        let content = std::fs::read_to_string("tests_assets/apnic.txt").unwrap();

        let results = parse_ip_data(&content);

        assert!(!results.is_empty());

        for route in results {
            assert!(matches!(route, IpNet::V4(_)));
        }

        assert_eq!(
            parse_ipv4_record("1.0.1.0", "256"),
            Some(IpNet::from_str("1.0.1.0/24").unwrap())
        );
    }
}
