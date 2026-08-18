use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgpNetworkClass {
    Eyeball,
    Hosting,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BgpAsnRecord {
    pub asn: String,
    pub name: String,
    pub class: String,
}

impl BgpAsnRecord {
    pub fn asn_number(&self) -> crate::error::Result<u32> {
        parse_asn(&self.asn)
    }

    pub fn network_class(&self) -> BgpNetworkClass {
        classify_network_class(&self.class)
    }
}

pub fn parse_asn(value: &str) -> crate::error::Result<u32> {
    let value = value.trim();

    let number = value
        .strip_prefix("AS")
        .or_else(|| value.strip_prefix("as"))
        .unwrap_or(value);

    number
        .parse::<u32>()
        .map_err(|_| crate::error::Error::InvalidTarget)
}

pub fn classify_network_class(value: &str) -> BgpNetworkClass {
    match value.trim().to_ascii_lowercase().as_str() {
        "eyeball" => BgpNetworkClass::Eyeball,
        "hosting" | "server hosting" => BgpNetworkClass::Hosting,
        _ => BgpNetworkClass::Unknown,
    }
}

pub fn parse_asns_csv(content: &str) -> crate::error::Result<Vec<BgpAsnRecord>> {
    let mut reader = csv::Reader::from_reader(content.as_bytes());

    reader
        .deserialize()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| crate::error::Error::InvalidTarget)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_asns_csv() {
        let content = r#"asn,name,class
AS37963,"Hangzhou Alibaba Advertising Co.,Ltd.",Eyeball
AS9808,"China Mobile Backbone",Eyeball
AS4134,"China Telecom Backbone",Eyeball
"#;

        let records = parse_asns_csv(content).unwrap();

        assert_eq!(records.len(), 3);

        assert_eq!(records[0].asn, "AS37963");
        assert_eq!(records[0].name, "Hangzhou Alibaba Advertising Co.,Ltd.");
        assert_eq!(records[0].class, "Eyeball");

        assert_eq!(records[1].asn, "AS9808");
        assert_eq!(records[2].asn, "AS4134");
    }

    #[test]
    fn test_parse_empty_csv() {
        let content = "asn,name,class\n";

        let records = parse_asns_csv(content).unwrap();

        assert!(records.is_empty());
    }

    #[test]
    fn test_parse_csv_with_quoted_commas() {
        let content = r#"asn,name,class
AS12345,"Example Network, Inc.",Unknown
"#;

        let records = parse_asns_csv(content).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].asn, "AS12345");
        assert_eq!(records[0].name, "Example Network, Inc.");
    }

    #[test]
    fn test_parse_asn() {
        assert_eq!(parse_asn("AS37963").unwrap(), 37963);
        assert_eq!(parse_asn("AS9808").unwrap(), 9808);
        assert_eq!(parse_asn("AS4134").unwrap(), 4134);
    }

    #[test]
    fn test_parse_asn_accepts_lowercase_prefix() {
        assert_eq!(parse_asn("as37963").unwrap(), 37963);
    }

    #[test]
    fn test_parse_asn_accepts_numeric_value() {
        assert_eq!(parse_asn("37963").unwrap(), 37963);
    }

    #[test]
    fn test_parse_asn_trims_whitespace() {
        assert_eq!(parse_asn("  AS37963  ").unwrap(), 37963);
    }

    #[test]
    fn test_parse_invalid_asn() {
        assert!(parse_asn("AS-not-a-number").is_err());
        assert!(parse_asn("").is_err());
        assert!(parse_asn("AS").is_err());
    }

    #[test]
    fn test_classify_network_class() {
        assert_eq!(
            classify_network_class("Eyeball"),
            BgpNetworkClass::Eyeball
        );

        assert_eq!(classify_network_class("Hosting"), BgpNetworkClass::Hosting);

        assert_eq!(
            classify_network_class("Server Hosting"),
            BgpNetworkClass::Hosting
        );

        assert_eq!(classify_network_class("Unknown"), BgpNetworkClass::Unknown);

        assert_eq!(
            classify_network_class("something-else"),
            BgpNetworkClass::Unknown
        );
    }

    #[test]
    fn test_asn_record_network_class() {
        let record = BgpAsnRecord {
            asn: "AS37963".to_string(),
            name: "Example".to_string(),
            class: "Eyeball".to_string(),
        };

        assert_eq!(record.network_class(), BgpNetworkClass::Eyeball);
    }

    #[test]
    fn test_asn_record_number() {
        let record = BgpAsnRecord {
            asn: "AS37963".to_string(),
            name: "Example".to_string(),
            class: "Eyeball".to_string(),
        };

        assert_eq!(record.asn_number().unwrap(), 37963);
    }
}
