#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkClass {
    CnMainlandCloud,
    CnMainlandIsp,
    HongKong,
    Overseas,
    Unknown,
}

impl NetworkClass {
    pub const fn is_cn_mainland(self) -> bool {
        matches!(self, Self::CnMainlandCloud | Self::CnMainlandIsp)
    }

    pub const fn is_direct(self) -> bool {
        self.is_cn_mainland()
    }
}

pub fn classify_origin_asn(asn: u32) -> NetworkClass {
    match asn {
        37963 => NetworkClass::CnMainlandCloud,
        9808 => NetworkClass::CnMainlandIsp,
        135097 => NetworkClass::Overseas,
        _ => NetworkClass::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_known_asns() {
        assert_eq!(classify_origin_asn(37963), NetworkClass::CnMainlandCloud);
        assert_eq!(classify_origin_asn(9808), NetworkClass::CnMainlandIsp);
        assert_eq!(classify_origin_asn(135097), NetworkClass::Overseas);
    }

    #[test]
    fn test_unknown_asn() {
        assert_eq!(classify_origin_asn(99999), NetworkClass::Unknown);
    }

    #[test]
    fn test_direct_policy() {
        assert!(NetworkClass::CnMainlandCloud.is_direct());
        assert!(NetworkClass::CnMainlandIsp.is_direct());

        assert!(!NetworkClass::HongKong.is_direct());
        assert!(!NetworkClass::Overseas.is_direct());
        assert!(!NetworkClass::Unknown.is_direct());
    }
}
