#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgpAsnWhois {
    pub asn: u32,
    pub country: String,
    pub registry: String,
    pub name: String,
}

pub fn parse_asn_whois_line(line: &str) -> crate::error::Result<BgpAsnWhois> {
    let fields: Vec<_> = line.split('|').map(str::trim).collect();

    if fields.len() < 7 {
        return Err(crate::error::Error::InvalidTarget);
    }

    let asn = fields[0]
        .parse::<u32>()
        .map_err(|_| crate::error::Error::InvalidTarget)?;

    let country = fields[3].to_string();
    let registry = fields[4].to_string();
    let name = fields[6..].join(" | ");

    if country.is_empty() || registry.is_empty() || name.is_empty() {
        return Err(crate::error::Error::InvalidTarget);
    }

    Ok(BgpAsnWhois {
        asn,
        country,
        registry,
        name,
    })
}

pub fn is_mainland_china_country_code(country: &str) -> bool {
    country.trim().eq_ignore_ascii_case("CN")
}

pub fn is_hong_kong_country_code(country: &str) -> bool {
    country.trim().eq_ignore_ascii_case("HK")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_asn_whois_line() {
        let line = "37963  |        |           | CN | APNIC    | 2001-03-27 | Alibaba Cloud";

        let record = parse_asn_whois_line(line).unwrap();

        assert_eq!(record.asn, 37963);
        assert_eq!(record.country, "CN");
        assert_eq!(record.registry, "APNIC");
        assert_eq!(record.name, "Alibaba Cloud");
    }

    #[test]
    fn test_parse_whois_with_empty_prefix_fields() {
        let line = "9808   |        |           | CN | APNIC    | 2002-08-09 | China Mobile";

        let record = parse_asn_whois_line(line).unwrap();

        assert_eq!(record.asn, 9808);
        assert_eq!(record.country, "CN");
        assert_eq!(record.name, "China Mobile");
    }

    #[test]
    fn test_parse_whois_with_embedded_pipes_in_name() {
        let line =
            "4134   |        |           | CN | APNIC    | 2000-01-01 | China Telecom | Backbone";

        let record = parse_asn_whois_line(line).unwrap();

        assert_eq!(record.asn, 4134);
        assert_eq!(record.country, "CN");
        assert_eq!(record.name, "China Telecom | Backbone");
    }

    #[test]
    fn test_country_helpers() {
        assert!(is_mainland_china_country_code("CN"));
        assert!(is_mainland_china_country_code("cn"));
        assert!(!is_mainland_china_country_code("HK"));

        assert!(is_hong_kong_country_code("HK"));
        assert!(is_hong_kong_country_code("hk"));
        assert!(!is_hong_kong_country_code("CN"));
    }

    #[test]
    fn test_invalid_whois_line_is_rejected() {
        assert!(parse_asn_whois_line("invalid").is_err());
    }

    #[test]
    fn test_short_whois_line_is_rejected() {
        let line = "37963 | | | CN | APNIC | 2001-03-27";

        assert!(parse_asn_whois_line(line).is_err());
    }

    #[test]
    fn test_empty_country_is_rejected() {
        let line = "37963  |        |           |    | APNIC    | 2001-03-27 | Alibaba Cloud";

        assert!(parse_asn_whois_line(line).is_err());
    }

    #[test]
    fn test_empty_registry_is_rejected() {
        let line = "37963  |        |           | CN |          | 2001-03-27 | Alibaba Cloud";

        assert!(parse_asn_whois_line(line).is_err());
    }

    #[test]
    fn test_empty_name_is_rejected() {
        let line = "37963 | | | CN | APNIC | 2001-03-27 |";

        assert!(parse_asn_whois_line(line).is_err());
    }
}
