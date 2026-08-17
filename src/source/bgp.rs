use serde::Deserialize;

const BGP_TABLE_URL: &str = "https://bgp.tools/table.jsonl";
const BGP_USER_AGENT: &str = "chnroutes-rs-next/0.2.0";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BgpTableRecord {
    #[serde(rename = "CIDR")]
    pub cidr: String,

    #[serde(rename = "ASN")]
    pub asn: u32,

    #[serde(rename = "Hits")]
    pub hits: u64,
}

pub fn parse_table_jsonl(content: &str) -> crate::error::Result<Vec<BgpTableRecord>> {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn fetch_table_jsonl() -> crate::error::Result<String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(BGP_USER_AGENT)
        .build()?;

    let response = client.get(BGP_TABLE_URL).send()?;

    let response = response.error_for_status()?;

    Ok(response.text()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_table_jsonl() {
        let content = r#"{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":1234}
{"CIDR":"117.128.0.0/10","ASN":9808,"Hits":5678}
{"CIDR":"156.224.128.0/17","ASN":135097,"Hits":42}
"#;

        let records = parse_table_jsonl(content).unwrap();

        assert_eq!(records.len(), 3);

        assert_eq!(
            records[0],
            BgpTableRecord {
                cidr: "8.152.0.0/13".to_string(),
                asn: 37963,
                hits: 1234,
            }
        );

        assert_eq!(records[1].asn, 9808);
        assert_eq!(records[2].hits, 42);
    }

    #[test]
    fn test_parse_table_jsonl_ignores_blank_lines() {
        let content = r#"
{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":1234}

{"CIDR":"117.128.0.0/10","ASN":9808,"Hits":5678}

"#;

        let records = parse_table_jsonl(content).unwrap();

        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_parse_table_jsonl_invalid_json() {
        let content = r#"{"CIDR":"8.152.0.0/13","ASN":37963,"Hits":1234}
not-json
"#;

        assert!(parse_table_jsonl(content).is_err());
    }
}
