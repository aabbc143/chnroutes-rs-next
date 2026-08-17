use chnroutes::source::classifier::{classify_origin_asn, NetworkClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Truth {
    CnMainland,
    NonCnMainland,
}

#[derive(Debug, Clone, Copy)]
struct Case {
    cidr: &'static str,
    truth: Truth,
    origin_asn: u32,
    chnroutes2_coverage: f64,
}

#[derive(Debug, Default, Clone, Copy)]
struct ConfusionMatrix {
    tp: usize,
    fp: usize,
    tn: usize,
    fn_: usize,
}

impl ConfusionMatrix {
    fn precision(self) -> f64 {
        let denominator = self.tp + self.fp;

        if denominator == 0 {
            0.0
        } else {
            self.tp as f64 * 100.0 / denominator as f64
        }
    }

    fn recall(self) -> f64 {
        let denominator = self.tp + self.fn_;

        if denominator == 0 {
            0.0
        } else {
            self.tp as f64 * 100.0 / denominator as f64
        }
    }

    fn f1(self) -> f64 {
        let precision = self.precision();
        let recall = self.recall();

        if precision + recall == 0.0 {
            0.0
        } else {
            2.0 * precision * recall / (precision + recall)
        }
    }

    fn false_positive_rate(self) -> f64 {
        let denominator = self.fp + self.tn;

        if denominator == 0 {
            0.0
        } else {
            self.fp as f64 * 100.0 / denominator as f64
        }
    }
}

const CASES: &[Case] = &[
    Case {
        cidr: "8.152.0.0/13",
        truth: Truth::CnMainland,
        origin_asn: 37963,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.136.0.0/13",
        truth: Truth::CnMainland,
        origin_asn: 37963,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.144.0.0/14",
        truth: Truth::CnMainland,
        origin_asn: 37963,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.160.0.0/15",
        truth: Truth::CnMainland,
        origin_asn: 37963,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.130.0.0/15",
        truth: Truth::CnMainland,
        origin_asn: 37963,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.129.0.0/16",
        truth: Truth::CnMainland,
        origin_asn: 37963,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.149.0.0/16",
        truth: Truth::CnMainland,
        origin_asn: 37963,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.163.0.0/16",
        truth: Truth::CnMainland,
        origin_asn: 37963,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.148.128.0/17",
        truth: Truth::CnMainland,
        origin_asn: 37963,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "117.128.0.0/10",
        truth: Truth::CnMainland,
        origin_asn: 9808,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "223.122.0.0/15",
        truth: Truth::NonCnMainland,
        origin_asn: 135097,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "156.224.128.0/17",
        truth: Truth::NonCnMainland,
        origin_asn: 135097,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "154.197.128.0/17",
        truth: Truth::NonCnMainland,
        origin_asn: 135097,
        chnroutes2_coverage: 100.0,
    },
];

fn classify_v2(case: &Case) -> bool {
    if case.chnroutes2_coverage <= 0.0 {
        return false;
    }

    classify_origin_asn(case.origin_asn).is_direct()
}

fn evaluate() -> ConfusionMatrix {
    let mut matrix = ConfusionMatrix::default();

    for case in CASES {
        let predicted_positive = classify_v2(case);

        match (case.truth, predicted_positive) {
            (Truth::CnMainland, true) => matrix.tp += 1,
            (Truth::CnMainland, false) => matrix.fn_ += 1,
            (Truth::NonCnMainland, true) => matrix.fp += 1,
            (Truth::NonCnMainland, false) => matrix.tn += 1,
        }
    }

    matrix
}

#[test]
fn compare_smart_v2() {
    let matrix = evaluate();

    println!("=== SMART_V2 strategy ===");
    println!();
    println!(
        "  TP: {:>2}  FP: {:>2}  TN: {:>2}  FN: {:>2}",
        matrix.tp, matrix.fp, matrix.tn, matrix.fn_
    );
    println!("  Precision:           {:>8.4}%", matrix.precision());
    println!("  Recall:              {:>8.4}%", matrix.recall());
    println!("  F1:                  {:>8.4}%", matrix.f1());
    println!(
        "  False Positive Rate: {:>8.4}%",
        matrix.false_positive_rate()
    );

    assert_eq!(matrix.tp, 10);
    assert_eq!(matrix.fn_, 0);
    assert_eq!(matrix.fp, 0);
    assert_eq!(matrix.tn, 3);
}

#[test]
fn verify_known_network_classes() {
    assert_eq!(classify_origin_asn(37963), NetworkClass::CnMainlandCloud);
    assert_eq!(classify_origin_asn(9808), NetworkClass::CnMainlandIsp);
    assert_eq!(classify_origin_asn(135097), NetworkClass::Overseas);
}
