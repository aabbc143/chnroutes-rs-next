use chnroutes::source::bgp::parse_ipv4_routes_jsonl;
use chnroutes::source::classifier::{AsnClass, AsnClassifier};

#[test]
fn test_bgp_routes_are_classified_by_origin_asn() {
    let content = r#"{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":1234}
{"CIDR":"117.128.0.0/10","ASN":9808,"Hits":5678}
{"CIDR":"223.122.0.0/15","ASN":4515,"Hits":999}
{"CIDR":"156.224.128.0/17","ASN":135097,"Hits":42}
{"CIDR":"1.2.3.0/24","ASN":99999,"Hits":1}
"#;

    let routes = parse_ipv4_routes_jsonl(content).unwrap();

    assert_eq!(routes.len(), 5);

    let mut classifier = AsnClassifier::new();

    classifier.insert_mainland_cloud(37963);
    classifier.insert_mainland_isp(9808);
    classifier.insert_hong_kong(4515);
    classifier.insert_overseas(135097);

    assert_eq!(
        classifier.classify(routes[0].asn),
        AsnClass::CnMainlandCloud
    );

    assert_eq!(classifier.classify(routes[1].asn), AsnClass::CnMainlandIsp);

    assert_eq!(classifier.classify(routes[2].asn), AsnClass::HongKong);

    assert_eq!(classifier.classify(routes[3].asn), AsnClass::Overseas);

    assert_eq!(classifier.classify(routes[4].asn), AsnClass::Unknown);
}

#[test]
fn test_bgp_direct_policy_is_conservative() {
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

    assert!(classifier.is_direct(routes[0].asn));
    assert!(classifier.is_direct(routes[1].asn));

    assert!(!classifier.is_direct(routes[2].asn));
    assert!(!classifier.is_direct(routes[3].asn));
    assert!(!classifier.is_direct(routes[4].asn));
}
