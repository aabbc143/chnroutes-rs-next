use std::collections::HashMap;

use super::bgp_evidence::BgpAsnEvidence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgpAsnRecord {
    pub asn: String,
    pub name: String,
    pub class: String,
}

impl BgpAsnRecord {
    pub fn asn_number(&self) -> Option<u32> {
        self.asn
            .strip_prefix("AS")
            .or_else(|| self.asn.strip_prefix("as"))
            .and_then(|value| value.parse::<u32>().ok())
    }
}

pub fn build_asn_index(records: &[BgpAsnRecord]) -> HashMap<u32, BgpAsnRecord> {
    records
        .iter()
        .filter_map(|record| record.asn_number().map(|asn| (asn, record.clone())))
        .collect()
}

pub fn merge_with_whois(
    record: &BgpAsnRecord,
    whois: &super::bgp_whois::BgpAsnWhois,
) -> BgpAsnEvidence {
    super::bgp_evidence::merge_asn_evidence(record, whois)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asn_number() {
        let record = BgpAsnRecord {
            asn: "AS4134".to_string(),
            name: "China Telecom".to_string(),
            class: "Eyeball".to_string(),
        };

        assert_eq!(record.asn_number(), Some(4134));
    }

    #[test]
    fn test_lowercase_asn_number() {
        let record = BgpAsnRecord {
            asn: "as9808".to_string(),
            name: "China Mobile".to_string(),
            class: "Eyeball".to_string(),
        };

        assert_eq!(record.asn_number(), Some(9808));
    }

    #[test]
    fn test_invalid_asn_number() {
        let record = BgpAsnRecord {
            asn: "invalid".to_string(),
            name: "Unknown".to_string(),
            class: "Unknown".to_string(),
        };

        assert_eq!(record.asn_number(), None);
    }

    #[test]
    fn test_build_asn_index() {
        let records = vec![
            BgpAsnRecord {
                asn: "AS4134".to_string(),
                name: "China Telecom".to_string(),
                class: "Eyeball".to_string(),
            },
            BgpAsnRecord {
                asn: "AS9808".to_string(),
                name: "China Mobile".to_string(),
                class: "Eyeball".to_string(),
            },
        ];

        let index = build_asn_index(&records);

        assert_eq!(index.len(), 2);
        assert_eq!(index.get(&4134).unwrap().name, "China Telecom");
        assert_eq!(index.get(&9808).unwrap().name, "China Mobile");
    }

    #[test]
    fn test_merge_with_whois() {
        let record = BgpAsnRecord {
            asn: "AS4134".to_string(),
            name: "China Telecom".to_string(),
            class: "Eyeball".to_string(),
        };

        let whois = super::super::bgp_whois::BgpAsnWhois {
            asn: 4134,
            country: "CN".to_string(),
            registry: "APNIC".to_string(),
            name: "CHINANET".to_string(),
        };

        let evidence = merge_with_whois(&record, &whois);

        assert_eq!(evidence.asn, 4134);
        assert_eq!(evidence.country, "CN");
        assert_eq!(evidence.network_class, "Eyeball");
    }
}
