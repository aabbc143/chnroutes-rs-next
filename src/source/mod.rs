use std::str::FromStr;

use ipnet::IpNet;

pub mod apnic;

/// Data source used to generate the CN IP route list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Source {
    /// APNIC delegated address data.
    #[default]
    Apnic,

    #[cfg(test)]
    Test,
}

impl FromStr for Source {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "apnic" => Ok(Self::Apnic),

            #[cfg(test)]
            "test" => Ok(Self::Test),

            _ => Err(format!(
                "unknown source '{value}', supported sources: apnic"
            )),
        }
    }
}

impl Source {
    /// Return a stable string representation used by the CLI.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Apnic => "apnic",

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
        assert!("unknown".parse::<Source>().is_err());
    }

    #[test]
    fn test_source_as_str() {
        assert_eq!(Source::Apnic.as_str(), "apnic");
    }
}
