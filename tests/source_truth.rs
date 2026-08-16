use ipnet::IpNet;
use std::str::FromStr;

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

#[test]
fn test_source_truth_cases() {
    let cases = [
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
    ];

    for case in cases {
        let network =
            IpNet::from_str(case.cidr).expect("truth case must contain a valid CIDR");

        assert!(
            network.is_ipv4(),
            "truth case must contain an IPv4 network: {}",
            case.cidr
        );

        println!(
            "{:<20} {:?} - {}",
            case.cidr, case.truth, case.reason
        );
    }
}

#[test]
fn test_truth_category_values() {
    assert_ne!(
        Truth::CnMainlandCloud,
        Truth::Overseas
    );

    assert_ne!(
        Truth::CnMainlandIsp,
        Truth::HongKong
    );
}
