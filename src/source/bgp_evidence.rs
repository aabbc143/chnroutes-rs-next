#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgpAsnEvidence {
    pub asn: u32,
    pub name: String,
    pub network_class: String,
    pub country: String,
    pub registry: String,
}

impl BgpAsnEvidence {
    pub fn is_cn_country(&self) -> bool {
        self.country.eq_ignore_ascii_case("CN")
    }

    pub fn is_hk_country(&self) -> bool {
        self.country.eq_ignore_ascii_case("HK")
    }

    pub fn is_known(&self) -> bool {
        !self.country.is_empty() && !self.network_class.is_empty()
    }
}

pub fn merge_asn_evidence(
    asn_record: &super::bgp_asn::BgpAsnRecord,
    whois: &super::bgp_whois::BgpAsnWhois,
) -> BgpAsnEvidence {
    BgpAsnEvidence {
        asn: whois.asn,
        name: asn_record.name.clone(),
        network_class: asn_record.class.clone(),
        country: whois.country.clone(),
        registry: whois.registry.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_asn_evidence() {
        let asn = super::super::bgp_asn::BgpAsnRecord {
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

        let evidence = merge_asn_evidence(&asn, &whois);

        assert_eq!(evidence.asn, 4134);
        assert_eq!(evidence.country, "CN");
        assert!(evidence.is_cn_country());
        assert!(evidence.is_known());
    }
}
