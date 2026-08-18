use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BgpAsnRecord {
    pub asn: String,
    pub name: String,
    pub class: String,
}

pub fn parse_asns_csv(content: &str) -> crate::error::Result<Vec<BgpAsnRecord>> {
    let mut reader = csv::Reader::from_reader(content.as_bytes());

    reader
        .deserialize()
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
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
}
