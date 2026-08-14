use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

use ipnet::IpNet;
use log::info;

const CHNROUTES2_URL: &str = "https://chnroutes2.cdn.skk.moe/chnroutes.txt";
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// Fetch the optimized China IPv4 CIDR list from chnroutes2.
pub fn fetch_ip_data() -> crate::error::Result<Vec<IpNet>> {
    info!("Fetching chnroutes2 data ...");

    let client = reqwest::blocking::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()?;

    let response = client
        .get(CHNROUTES2_URL)
        .send()?
        .error_for_status()?;

    let data = response.text()?;

    info!("Fetching chnroutes2 data done");

    Ok(parse_ip_data(&data))
}

/// Parse chnroutes2 text into IPv4 networks.
///
/// Lines beginning with `#` and blank lines are ignored.
pub fn parse_ip_data(content: &str) -> Vec<IpNet> {
    content
        .lines()
        .filter_map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(parse_cidr)
        .collect()
}

fn parse_cidr(line: &str) -> Option<IpNet> {
    let network = IpNet::from_str(line).ok()?;

    match network {
        IpNet::V4(_) => Some(network),
        IpNet::V6(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ip_data() {
        let content = "\
#########################################
# Sukka's Optimized CHNRoutes
# Size: 3
#########################################
1.1.8.0/24
1.12.0.0/14

1.24.0.0/13
2400:da00::/32
invalid
";

        let results = parse_ip_data(content);

        assert_eq!(results.len(), 3);
        assert_eq!(
            results[0],
            IpNet::from_str("1.1.8.0/24").unwrap()
        );
        assert_eq!(
            results[1],
            IpNet::from_str("1.12.0.0/14").unwrap()
        );
        assert_eq!(
            results[2],
            IpNet::from_str("1.24.0.0/13").unwrap()
        );
    }

    #[test]
    fn test_parse_cidr() {
        assert_eq!(
            parse_cidr("1.1.8.0/24"),
            Some(IpNet::from_str("1.1.8.0/24").unwrap())
        );

        assert_eq!(parse_cidr("2400:da00::/32"), None);
        assert_eq!(parse_cidr("not-a-cidr"), None);
    }
}
