#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Page {
    Targets,
    Monitor,
    Vpn,
    Settings,
}

impl Page {
    pub(crate) const ALL: [Self; 4] = [Self::Targets, Self::Monitor, Self::Vpn, Self::Settings];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Targets => "Targets",
            Self::Monitor => "Monitor",
            Self::Vpn => "VPN",
            Self::Settings => "Settings",
        }
    }
}
