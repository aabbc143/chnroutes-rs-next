#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Truth {
    CnMainland,
    NonCnMainland,
}

#[derive(Debug, Clone, Copy)]
struct CoverageCase {
    cidr: &'static str,
    truth: Truth,
    apnic_coverage: f64,
    chnroutes2_coverage: f64,
}

const CASES: &[CoverageCase] = &[
    CoverageCase {
        cidr: "8.152.0.0/13",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "8.136.0.0/13",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "8.144.0.0/14",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "8.160.0.0/15",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "8.130.0.0/15",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "8.129.0.0/16",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "8.149.0.0/16",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "8.163.0.0/16",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "8.148.128.0/17",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "117.128.0.0/10",
        truth: Truth::CnMainland,
        apnic_coverage: 99.6094,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "223.122.0.0/15",
        truth: Truth::NonCnMainland,
        apnic_coverage: 9.3750,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "156.224.128.0/17",
        truth: Truth::NonCnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    CoverageCase {
        cidr: "154.197.128.0/17",
        truth: Truth::NonCnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
];

#[test]
fn test_source_metrics() {
    let mainland = CASES
        .iter()
        .filter(|case| case.truth == Truth::CnMainland)
        .count();

    let non_mainland = CASES
        .iter()
        .filter(|case| case.truth == Truth::NonCnMainland)
        .count();

    let apnic_recall = average_coverage(
        CASES
            .iter()
            .filter(|case| case.truth == Truth::CnMainland)
            .map(|case| case.apnic_coverage),
    );

    let chnroutes2_recall = average_coverage(
        CASES
            .iter()
            .filter(|case| case.truth == Truth::CnMainland)
            .map(|case| case.chnroutes2_coverage),
    );

    let apnic_false_positive_rate = positive_rate(
        CASES
            .iter()
            .filter(|case| case.truth == Truth::NonCnMainland)
            .map(|case| case.apnic_coverage),
    );

    let chnroutes2_false_positive_rate = positive_rate(
        CASES
            .iter()
            .filter(|case| case.truth == Truth::NonCnMainland)
            .map(|case| case.chnroutes2_coverage),
    );

    println!("=== Source metrics ===");
    println!();
    println!("Ground Truth:");
    println!("  Mainland cases:       {}", mainland);
    println!("  Non-mainland cases:   {}", non_mainland);
    println!();
    println!("Coverage recall:");
    println!("  APNIC:                {:.4}%", apnic_recall);
    println!("  chnroutes2:           {:.4}%", chnroutes2_recall);
    println!();
    println!("False-positive coverage:");
    println!("  APNIC:                {:.4}%", apnic_false_positive_rate);
    println!(
        "  chnroutes2:           {:.4}%",
        chnroutes2_false_positive_rate
    );

    assert_eq!(mainland, 10);
    assert_eq!(non_mainland, 3);

    assert!(chnroutes2_recall > apnic_recall);
}

fn average_coverage<I>(values: I) -> f64
where
    I: Iterator<Item = f64>,
{
    let values: Vec<f64> = values.collect();

    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn positive_rate<I>(values: I) -> f64
where
    I: Iterator<Item = f64>,
{
    let values: Vec<f64> = values.collect();

    if values.is_empty() {
        0.0
    } else {
        let positive = values.iter().filter(|value| **value > 0.0).count();

        positive as f64 * 100.0 / values.len() as f64
    }
}
