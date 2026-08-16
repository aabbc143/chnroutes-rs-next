use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::str::FromStr;

use chnroutes::source::{apnic, chnroutes2};
use ipnet::IpNet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Truth {
    CnMainlandCloud,
    CnMainlandIsp,
    HongKong,
    Overseas,
}

struct TruthCase {
    cidr: &'static str,
    truth: Truth,
    reason: &'static str,
}

type Interval = (u32, u32);

#[test]
fn test_truth_category_values() {
    assert_ne!(Truth::CnMainlandCloud, Truth::Overseas);
    assert_ne!(Truth::CnMainlandIsp, Truth::HongKong);
}

#[test]
fn test_source_truth_cases_are_valid_ipv4() {
    let cases = truth_cases();

    for case in cases {
        let network = IpNet::from_str(case.cidr).expect("truth case must contain a valid CIDR");

        assert!(
            matches!(network, IpNet::V4(_)),
            "truth case must contain an IPv4 network: {}",
            case.cidr
        );

        println!("{:<20} {:?} - {}", case.cidr, case.truth, case.reason);
    }
}

#[test]
#[ignore = "requires network access"]
fn compare_truth_cases_against_sources() {
    let apnic_data = apnic::fetch_ip_data().expect("failed to fetch APNIC data");
    let chnroutes2_data = chnroutes2::fetch_ip_data().expect("failed to fetch chnroutes2 data");

    let apnic_set: HashSet<_> = apnic_data
        .into_iter()
        .filter(|network| matches!(network, IpNet::V4(_)))
        .collect();

    let chnroutes2_set: HashSet<_> = chnroutes2_data
        .into_iter()
        .filter(|network| matches!(network, IpNet::V4(_)))
        .collect();

    let apnic_intervals = normalize(&apnic_set);
    let chnroutes2_intervals = normalize(&chnroutes2_set);

    println!("=== Ground Truth Source Coverage ===");
    println!();
    println!(
        "{:<20} {:<20} {:>12} {:>12}",
        "CIDR", "Truth", "APNIC", "chnroutes2"
    );

    for case in truth_cases() {
        let network = IpNet::from_str(case.cidr).expect("truth case must contain a valid CIDR");

        let (start, end) = match network {
            IpNet::V4(network) => (
                ipv4_to_u32(network.network()),
                ipv4_to_u32(network.broadcast()),
            ),
            IpNet::V6(_) => unreachable!(),
        };

        let apnic_coverage = coverage_percentage((start, end), &apnic_intervals);
        let chnroutes2_coverage = coverage_percentage((start, end), &chnroutes2_intervals);

        println!(
            "{:<20} {:<20} {:>11.4}% {:>11.4}%",
            case.cidr,
            truth_name(case.truth),
            apnic_coverage,
            chnroutes2_coverage
        );
    }
}

fn truth_cases() -> &'static [TruthCase] {
    &[
        TruthCase {
            cidr: "8.152.0.0/13",
            truth: Truth::CnMainlandCloud,
            reason: "Alibaba / AS37963",
        },
        TruthCase {
            cidr: "8.136.0.0/13",
            truth: Truth::CnMainlandCloud,
            reason: "Alibaba / AS37963",
        },
        TruthCase {
            cidr: "8.144.0.0/14",
            truth: Truth::CnMainlandCloud,
            reason: "Alibaba / AS37963",
        },
        TruthCase {
            cidr: "8.160.0.0/15",
            truth: Truth::CnMainlandCloud,
            reason: "Alibaba / AS37963",
        },
        TruthCase {
            cidr: "8.130.0.0/15",
            truth: Truth::CnMainlandCloud,
            reason: "Alibaba / AS37963",
        },
        TruthCase {
            cidr: "8.129.0.0/16",
            truth: Truth::CnMainlandCloud,
            reason: "Alibaba / AS37963",
        },
        TruthCase {
            cidr: "8.149.0.0/16",
            truth: Truth::CnMainlandCloud,
            reason: "Alibaba / AS37963",
        },
        TruthCase {
            cidr: "8.163.0.0/16",
            truth: Truth::CnMainlandCloud,
            reason: "Alibaba / AS37963",
        },
        TruthCase {
            cidr: "8.148.128.0/17",
            truth: Truth::CnMainlandCloud,
            reason: "Alibaba / AS37963",
        },
        TruthCase {
            cidr: "117.128.0.0/10",
            truth: Truth::CnMainlandIsp,
            reason: "China Mobile / AS9808",
        },
        TruthCase {
            cidr: "223.122.0.0/15",
            truth: Truth::HongKong,
            reason: "China Mobile Hong Kong",
        },
        TruthCase {
            cidr: "156.224.128.0/17",
            truth: Truth::Overseas,
            reason: "LUOGELANG / AS135097",
        },
        TruthCase {
            cidr: "154.197.128.0/17",
            truth: Truth::Overseas,
            reason: "LUOGELANG / AS135097",
        },
    ]
}

fn truth_name(truth: Truth) -> &'static str {
    match truth {
        Truth::CnMainlandCloud => "CN_MAINLAND_CLOUD",
        Truth::CnMainlandIsp => "CN_MAINLAND_ISP",
        Truth::HongKong => "CN_HK",
        Truth::Overseas => "OVERSEAS",
    }
}

fn normalize(networks: &HashSet<IpNet>) -> Vec<Interval> {
    let mut intervals: Vec<Interval> = networks
        .iter()
        .filter_map(|network| match network {
            IpNet::V4(network) => {
                let start = ipv4_to_u32(network.network());
                let end = ipv4_to_u32(network.broadcast());

                Some((start, end))
            }
            IpNet::V6(_) => None,
        })
        .collect();

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

fn coverage_percentage(target: Interval, ranges: &[Interval]) -> f64 {
    let total = u64::from(target.1) - u64::from(target.0) + 1;
    let covered = intersection_size(&[target], ranges);

    if total == 0 {
        0.0
    } else {
        covered as f64 * 100.0 / total as f64
    }
}

fn intersection_size(left: &[Interval], right: &[Interval]) -> u64 {
    let mut i = 0;
    let mut j = 0;
    let mut total = 0u64;

    while i < left.len() && j < right.len() {
        let start = left[i].0.max(right[j].0);
        let end = left[i].1.min(right[j].1);

        if start <= end {
            total += u64::from(end) - u64::from(start) + 1;
        }

        if left[i].1 < right[j].1 {
            i += 1;
        } else {
            j += 1;
        }
    }

    total
}

fn ipv4_to_u32(address: Ipv4Addr) -> u32 {
    u32::from(address)
}
