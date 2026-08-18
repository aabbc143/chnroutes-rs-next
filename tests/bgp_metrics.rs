use chnroutes::source::bgp::{parse_ipv4_routes_jsonl, BgpRoute};
use chnroutes::source::classifier::AsnClassifier;
use ipnet::IpNet;
use std::collections::HashSet;

type Interval = (u32, u32);

#[derive(Debug, Default, PartialEq, Eq)]
struct BgpMetrics {
    total_routes: usize,
    direct_routes: usize,
    excluded_routes: usize,
    direct_addresses: u64,
    excluded_addresses: u64,
}

fn route_interval(route: &BgpRoute) -> Option<Interval> {
    match route.network {
        IpNet::V4(network) => Some((u32::from(network.network()), u32::from(network.broadcast()))),
        IpNet::V6(_) => None,
    }
}

fn normalize(routes: &[BgpRoute]) -> Vec<Interval> {
    let mut intervals: Vec<Interval> = routes.iter().filter_map(route_interval).collect();

    intervals.sort_unstable_by_key(|interval| interval.0);

    let mut merged: Vec<Interval> = Vec::with_capacity(intervals.len());

    for (start, end) in intervals {
        if let Some((_, current_end)) = merged.last_mut() {
            if start <= current_end.saturating_add(1) {
                if end > *current_end {
                    *current_end = end;
                }

                continue;
            }
        }

        merged.push((start, end));
    }

    merged
}

fn total_addresses(intervals: &[Interval]) -> u64 {
    intervals
        .iter()
        .map(|(start, end)| u64::from(*end) - u64::from(*start) + 1)
        .sum()
}

fn calculate_metrics(routes: &[BgpRoute], classifier: &AsnClassifier) -> BgpMetrics {
    let direct: Vec<_> = routes
        .iter()
        .filter(|route| classifier.is_direct(route.asn))
        .cloned()
        .collect();

    let excluded: Vec<_> = routes
        .iter()
        .filter(|route| !classifier.is_direct(route.asn))
        .cloned()
        .collect();

    let direct_intervals = normalize(&direct);
    let excluded_intervals = normalize(&excluded);

    BgpMetrics {
        total_routes: routes.len(),
        direct_routes: direct.len(),
        excluded_routes: excluded.len(),
        direct_addresses: total_addresses(&direct_intervals),
        excluded_addresses: total_addresses(&excluded_intervals),
    }
}

#[test]
fn test_bgp_metrics() {
    let content = r#"{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":1234}
{"CIDR":"117.128.0.0/10","ASN":9808,"Hits":5678}
{"CIDR":"223.122.0.0/15","ASN":4515,"Hits":999}
{"CIDR":"156.224.128.0/17","ASN":135097,"Hits":42}
{"CIDR":"1.2.3.0/24","ASN":99999,"Hits":1}
"#;

    let routes = parse_ipv4_routes_jsonl(content).unwrap();

    let mut classifier = AsnClassifier::new();
    classifier.insert_mainland_cloud(37963);
    classifier.insert_mainland_isp(9808);
    classifier.insert_hong_kong(4515);
    classifier.insert_overseas(135097);

    let metrics = calculate_metrics(&routes, &classifier);

    assert_eq!(
        metrics,
        BgpMetrics {
            total_routes: 5,
            direct_routes: 2,
            excluded_routes: 3,
            direct_addresses: 4_718_592,
            excluded_addresses: 164_096,
        }
    );
}

#[test]
fn test_duplicate_direct_cidrs_are_not_double_counted() {
    let content = r#"{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":1234}
{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":5678}
{"CIDR":"117.128.0.0/10","ASN":9808,"Hits":42}
"#;

    let routes = parse_ipv4_routes_jsonl(content).unwrap();

    let mut classifier = AsnClassifier::new();
    classifier.insert_mainland_cloud(37963);
    classifier.insert_mainland_isp(9808);

    let direct: Vec<_> = routes
        .iter()
        .filter(|route| classifier.is_direct(route.asn))
        .cloned()
        .collect();

    let direct_intervals = normalize(&direct);
    let addresses = total_addresses(&direct_intervals);

    assert_eq!(direct.len(), 3);
    assert_eq!(addresses, 4_718_592);
}

#[test]
fn test_overlapping_direct_cidrs_are_not_double_counted() {
    let content = r#"{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":1234}
{"CIDR":"8.152.0.0/14","ASN":37963,"Hits":5678}
{"CIDR":"8.156.0.0/14","ASN":9808,"Hits":42}
"#;

    let routes = parse_ipv4_routes_jsonl(content).unwrap();

    let mut classifier = AsnClassifier::new();
    classifier.insert_mainland_cloud(37963);
    classifier.insert_mainland_isp(9808);

    let direct: Vec<_> = routes
        .iter()
        .filter(|route| classifier.is_direct(route.asn))
        .cloned()
        .collect();

    let direct_intervals = normalize(&direct);
    let addresses = total_addresses(&direct_intervals);

    assert_eq!(direct.len(), 3);
    assert_eq!(addresses, 524_288);
}

#[test]
fn test_direct_and_excluded_ranges_are_separately_measured() {
    let content = r#"{"CIDR":"10.0.0.0/24","ASN":37963,"Hits":1}
{"CIDR":"10.0.1.0/24","ASN":9808,"Hits":1}
{"CIDR":"10.0.2.0/24","ASN":4515,"Hits":1}
{"CIDR":"10.0.3.0/24","ASN":99999,"Hits":1}
"#;

    let routes = parse_ipv4_routes_jsonl(content).unwrap();

    let mut classifier = AsnClassifier::new();
    classifier.insert_mainland_cloud(37963);
    classifier.insert_mainland_isp(9808);
    classifier.insert_hong_kong(4515);

    let metrics = calculate_metrics(&routes, &classifier);

    assert_eq!(metrics.total_routes, 4);
    assert_eq!(metrics.direct_routes, 2);
    assert_eq!(metrics.excluded_routes, 2);
    assert_eq!(metrics.direct_addresses, 512);
    assert_eq!(metrics.excluded_addresses, 512);
}

#[test]
fn test_direct_route_networks_are_unique() {
    let content = r#"{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":1234}
{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":5678}
{"CIDR":"117.128.0.0/10","ASN":9808,"Hits":42}
"#;

    let routes = parse_ipv4_routes_jsonl(content).unwrap();

    let mut classifier = AsnClassifier::new();
    classifier.insert_mainland_cloud(37963);
    classifier.insert_mainland_isp(9808);

    let direct: Vec<IpNet> = routes
        .iter()
        .filter(|route| classifier.is_direct(route.asn))
        .map(|route| route.network)
        .collect();

    let unique: HashSet<_> = direct.iter().copied().collect();

    assert_eq!(direct.len(), 3);
    assert_eq!(unique.len(), 2);
}
