use std::str::FromStr;

use ipnet::IpNet;

pub mod apnic;
pub mod bgp;
pub mod bgp_asn;
pub mod bgp_cache;
pub mod bgp_evidence;
pub mod bgp_whois;
pub mod chnroutes2;
pub mod classifier;

/// Data source used to generate the CN IP route list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Source {
    /// APNIC delegated address data.
    #[default]
    Apnic,

    /// Sukka's optimized China IPv4 CIDR list.
    Chnroutes2,

    #[cfg(test)]
    Test,
}

impl FromStr for Source {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "apnic" => Ok(Self::Apnic),
            "chnroutes2" => Ok(Self::Chnroutes2),

            #[cfg(test)]
            "test" => Ok(Self::Test),

            _ => Err(format!(
                "unknown source '{value}', supported sources: apnic, chnroutes2"
            )),
        }
    }
}

impl Source {
    /// Return a stable string representation used by the CLI.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Apnic => "apnic",
            Self::Chnroutes2 => "chnroutes2",

            #[cfg(test)]
            Self::Test => "test",
        }
    }

    pub fn get_cn_ips(&self) -> crate::error::Result<Vec<IpNet>> {
        get_cn_ips(self)
    }
}

pub fn get_cn_ips(source: &Source) -> crate::error::Result<Vec<IpNet>> {
    match source {
        Source::Apnic => apnic::fetch_ip_data(),
        Source::Chnroutes2 => chnroutes2::fetch_ip_data(),

        #[cfg(test)]
        Source::Test => Ok(vec![
            IpNet::from_str("1.0.1.0/24").unwrap(),
            IpNet::from_str("1.0.2.0/23").unwrap(),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_from_str() {
        assert_eq!("apnic".parse::<Source>().unwrap(), Source::Apnic);
        assert_eq!("APNIC".parse::<Source>().unwrap(), Source::Apnic);
        assert_eq!("chnroutes2".parse::<Source>().unwrap(), Source::Chnroutes2);
        assert!("unknown".parse::<Source>().is_err());
    }

    #[test]
    fn test_source_as_str() {
        assert_eq!(Source::Apnic.as_str(), "apnic");
        assert_eq!(Source::Chnroutes2.as_str(), "chnroutes2");
    }
}
