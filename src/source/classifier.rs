use std::collections::HashSet;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsnClass {
    CnMainlandCloud,
    CnMainlandIsp,
    HongKong,
    Overseas,
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct AsnClassifier {
    mainland_cloud: HashSet<u32>,
    mainland_isp: HashSet<u32>,
    hong_kong: HashSet<u32>,
    overseas: HashSet<u32>,
}

impl AsnClassifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_mainland_cloud(&mut self, asn: u32) {
        self.mainland_cloud.insert(asn);
    }

    pub fn insert_mainland_isp(&mut self, asn: u32) {
        self.mainland_isp.insert(asn);
    }

    pub fn insert_hong_kong(&mut self, asn: u32) {
        self.hong_kong.insert(asn);
    }

    pub fn insert_overseas(&mut self, asn: u32) {
        self.overseas.insert(asn);
    }

    pub fn classify(&self, asn: u32) -> AsnClass {
        if self.mainland_cloud.contains(&asn) {
            AsnClass::CnMainlandCloud
        } else if self.mainland_isp.contains(&asn) {
            AsnClass::CnMainlandIsp
        } else if self.hong_kong.contains(&asn) {
            AsnClass::HongKong
        } else if self.overseas.contains(&asn) {
            AsnClass::Overseas
        } else {
            AsnClass::Unknown
        }
    }

    pub fn is_direct(&self, asn: u32) -> bool {
        matches!(
            self.classify(asn),
            AsnClass::CnMainlandCloud | AsnClass::CnMainlandIsp
        )
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
        assert_eq!(
            classify_origin_asn(37963),
            NetworkClass::CnMainlandCloud
        );
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

    #[test]
    fn test_asn_classifier() {
        let mut classifier = AsnClassifier::new();

        classifier.insert_mainland_cloud(37963);
        classifier.insert_mainland_isp(9808);
        classifier.insert_hong_kong(4515);
        classifier.insert_overseas(135097);

        assert_eq!(
            classifier.classify(37963),
            AsnClass::CnMainlandCloud
        );
        assert_eq!(classifier.classify(9808), AsnClass::CnMainlandIsp);
        assert_eq!(classifier.classify(4515), AsnClass::HongKong);
        assert_eq!(classifier.classify(135097), AsnClass::Overseas);
        assert_eq!(classifier.classify(99999), AsnClass::Unknown);

        assert!(classifier.is_direct(37963));
        assert!(classifier.is_direct(9808));
        assert!(!classifier.is_direct(4515));
        assert!(!classifier.is_direct(135097));
        assert!(!classifier.is_direct(99999));
    }
}
