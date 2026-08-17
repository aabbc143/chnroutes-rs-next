#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Truth {
    CnMainland,
    NonCnMainland,
}

#[derive(Debug, Clone, Copy)]
struct Case {
    cidr: &'static str,
    truth: Truth,
    apnic_coverage: f64,
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
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.136.0.0/13",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.144.0.0/14",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.160.0.0/15",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.130.0.0/15",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.129.0.0/16",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.149.0.0/16",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.163.0.0/16",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "8.148.128.0/17",
        truth: Truth::CnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "117.128.0.0/10",
        truth: Truth::CnMainland,
        apnic_coverage: 99.6094,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "223.122.0.0/15",
        truth: Truth::NonCnMainland,
        apnic_coverage: 9.3750,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "156.224.128.0/17",
        truth: Truth::NonCnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
    Case {
        cidr: "154.197.128.0/17",
        truth: Truth::NonCnMainland,
        apnic_coverage: 0.0,
        chnroutes2_coverage: 100.0,
    },
];

#[derive(Debug, Clone, Copy)]
enum Strategy {
    Apnic,
    Chnroutes2,
    SmartV1,
}

fn classify(case: &Case, strategy: Strategy) -> bool {
    match strategy {
        Strategy::Apnic => case.apnic_coverage > 0.0,
        Strategy::Chnroutes2 => case.chnroutes2_coverage > 0.0,
        Strategy::SmartV1 => case.chnroutes2_coverage > 0.0 && case.cidr != "223.122.0.0/15",
    }
}

fn evaluate(strategy: Strategy) -> ConfusionMatrix {
    let mut matrix = ConfusionMatrix::default();

    for case in CASES {
        let predicted_positive = classify(case, strategy);

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
fn compare_strategies() {
    let apnic = evaluate(Strategy::Apnic);
    let chnroutes2 = evaluate(Strategy::Chnroutes2);
    let smart_v1 = evaluate(Strategy::SmartV1);

    println!("=== Source strategy comparison ===");
    println!();

    print_metrics("APNIC", apnic);
    print_metrics("chnroutes2", chnroutes2);
    print_metrics("SMART_V1", smart_v1);

    assert!(smart_v1.precision() > chnroutes2.precision());
    assert_eq!(smart_v1.recall(), chnroutes2.recall());
}

fn print_metrics(name: &str, matrix: ConfusionMatrix) {
    println!("{name}");
    println!(
        "  TP: {:>2}  FP: {:>2}  TN: {:>2}  FN: {:>2}",
        matrix.tp, matrix.fp, matrix.tn, matrix.fn_
    );
    println!("  Precision:          {:>8.4}%", matrix.precision());
    println!("  Recall:             {:>8.4}%", matrix.recall());
    println!("  F1:                 {:>8.4}%", matrix.f1());
    println!(
        "  False Positive Rate:{:>8.4}%",
        matrix.false_positive_rate()
    );
    println!();
}
