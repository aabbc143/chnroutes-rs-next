use chnroutes::source::bgp::parse_table_jsonl;
use chnroutes::source::classifier::{AsnClass, AsnClassifier};

#[test]
fn test_bgp_records_feed_asn_classifier() {
    let content = r#"{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":1234}
{"CIDR":"117.128.0.0/10","ASN":9808,"Hits":5678}
{"CIDR":"223.122.0.0/15","ASN":135097,"Hits":999}
"#;

    let records = parse_table_jsonl(content).unwrap();

    assert_eq!(records.len(), 3);

    let mut classifier = AsnClassifier::new();

    classifier.insert_mainland_cloud(37963);
    classifier.insert_mainland_isp(9808);
    classifier.insert_overseas(135097);

    assert_eq!(
        classifier.classify(records[0].asn),
        AsnClass::CnMainlandCloud
    );

    assert_eq!(
        classifier.classify(records[1].asn),
        AsnClass::CnMainlandIsp
    );

    assert_eq!(
        classifier.classify(records[2].asn),
        AsnClass::Overseas
    );
}

#[test]
fn test_unknown_asn_is_conservative() {
    let content = r#"{"CIDR":"1.2.3.0/24","ASN":99999,"Hits":1}
"#;

    let records = parse_table_jsonl(content).unwrap();

    let classifier = AsnClassifier::new();

    assert_eq!(
        classifier.classify(records[0].asn),
        AsnClass::Unknown
    );

    assert!(!classifier.is_direct(records[0].asn));
}
