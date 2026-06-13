//! Network target descriptions.

use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NetworkTarget {
    pub logical_name: String,
    pub is_local_system: bool,
    pub netbios_name: Option<String>,
    pub dns_name: Option<String>,
    pub ipv4_address: Option<Ipv4Addr>,
    pub ipv6_address: Option<Ipv6Addr>,
}

impl NetworkTarget {
    #[must_use]
    pub fn local_system() -> Self {
        Self {
            is_local_system: true,
            ..Self::default()
        }
    }

    /// Name suitable for APIs where an empty target means the local system.
    #[must_use]
    pub fn machine_name(&self) -> Option<&str> {
        if self.is_local_system {
            None
        } else {
            self.netbios_name
                .as_deref()
                .or(self.dns_name.as_deref())
                .or((!self.logical_name.is_empty()).then_some(self.logical_name.as_str()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkTarget;

    #[test]
    fn selects_appropriate_machine_name() {
        assert_eq!(NetworkTarget::local_system().machine_name(), None);
        let target = NetworkTarget {
            logical_name: "logical".into(),
            dns_name: Some("host.example".into()),
            ..NetworkTarget::default()
        };
        assert_eq!(target.machine_name(), Some("host.example"));
    }
}
