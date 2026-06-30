use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolDomain {
    CoreWayland,
    XdgShell,
    Presentation,
    BufferSharing,
    DesktopShell,
}

impl ProtocolDomain {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CoreWayland => "core-wayland",
            Self::XdgShell => "xdg-shell",
            Self::Presentation => "presentation",
            Self::BufferSharing => "buffer-sharing",
            Self::DesktopShell => "desktop-shell",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolStage {
    HeadlessSmoke,
    Planned,
}

impl ProtocolStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HeadlessSmoke => "headless-smoke",
            Self::Planned => "planned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolDefinition {
    pub global_name: &'static str,
    pub display_name: &'static str,
    pub domain: ProtocolDomain,
    pub minimum_version: u32,
    pub mvp_required: bool,
    pub stage: ProtocolStage,
}

impl ProtocolDefinition {
    pub const fn required(
        global_name: &'static str,
        display_name: &'static str,
        domain: ProtocolDomain,
        minimum_version: u32,
        stage: ProtocolStage,
    ) -> Self {
        Self {
            global_name,
            display_name,
            domain,
            minimum_version,
            mvp_required: true,
            stage,
        }
    }

    pub const fn planned(
        global_name: &'static str,
        display_name: &'static str,
        domain: ProtocolDomain,
        minimum_version: u32,
    ) -> Self {
        Self {
            global_name,
            display_name,
            domain,
            minimum_version,
            mvp_required: false,
            stage: ProtocolStage::Planned,
        }
    }
}

pub const MVP_PROTOCOLS: &[ProtocolDefinition] = &[
    ProtocolDefinition::required(
        "wl_compositor",
        "wl_compositor",
        ProtocolDomain::CoreWayland,
        4,
        ProtocolStage::HeadlessSmoke,
    ),
    ProtocolDefinition::required(
        "wl_shm",
        "wl_shm",
        ProtocolDomain::CoreWayland,
        1,
        ProtocolStage::HeadlessSmoke,
    ),
    ProtocolDefinition::required(
        "xdg_wm_base",
        "xdg-shell",
        ProtocolDomain::XdgShell,
        1,
        ProtocolStage::HeadlessSmoke,
    ),
    ProtocolDefinition::required(
        "zxdg_output_manager_v1",
        "xdg-output",
        ProtocolDomain::CoreWayland,
        3,
        ProtocolStage::HeadlessSmoke,
    ),
    ProtocolDefinition::required(
        "wp_viewporter",
        "viewporter",
        ProtocolDomain::CoreWayland,
        1,
        ProtocolStage::HeadlessSmoke,
    ),
    ProtocolDefinition::required(
        "wp_presentation",
        "presentation-time",
        ProtocolDomain::Presentation,
        1,
        ProtocolStage::HeadlessSmoke,
    ),
    ProtocolDefinition::required(
        "zwp_linux_dmabuf_v1",
        "linux-dmabuf",
        ProtocolDomain::BufferSharing,
        4,
        ProtocolStage::HeadlessSmoke,
    ),
];

pub const SHELL_PROTOCOLS: &[ProtocolDefinition] = &[ProtocolDefinition::planned(
    "zwlr_layer_shell_v1",
    "layer-shell-style shell surfaces",
    ProtocolDomain::DesktopShell,
    4,
)];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSmokeReport {
    pub required_protocols: usize,
    pub registered_protocols: usize,
    pub duplicate_globals: Vec<&'static str>,
    pub missing_required_globals: Vec<&'static str>,
}

impl ProtocolSmokeReport {
    pub fn passed(&self) -> bool {
        self.duplicate_globals.is_empty() && self.missing_required_globals.is_empty()
    }
}

pub fn protocol_smoke_report() -> ProtocolSmokeReport {
    let mut seen = BTreeSet::new();
    let mut duplicate_globals = Vec::new();
    let mut registered_globals = BTreeSet::new();

    for protocol in MVP_PROTOCOLS.iter().chain(SHELL_PROTOCOLS) {
        if !seen.insert(protocol.global_name) {
            duplicate_globals.push(protocol.global_name);
        }

        if protocol.stage == ProtocolStage::HeadlessSmoke {
            registered_globals.insert(protocol.global_name);
        }
    }

    let missing_required_globals = MVP_PROTOCOLS
        .iter()
        .filter(|protocol| protocol.mvp_required)
        .filter(|protocol| !registered_globals.contains(protocol.global_name))
        .map(|protocol| protocol.global_name)
        .collect();

    ProtocolSmokeReport {
        required_protocols: MVP_PROTOCOLS
            .iter()
            .filter(|protocol| protocol.mvp_required)
            .count(),
        registered_protocols: registered_globals.len(),
        duplicate_globals,
        missing_required_globals,
    }
}

pub fn lookup_protocol(global_name: &str) -> Option<ProtocolDefinition> {
    MVP_PROTOCOLS
        .iter()
        .chain(SHELL_PROTOCOLS)
        .copied()
        .find(|protocol| protocol.global_name == global_name)
}

#[cfg(test)]
mod tests {
    use super::{lookup_protocol, protocol_smoke_report, MVP_PROTOCOLS};

    #[test]
    fn mvp_protocols_match_design_count() {
        assert_eq!(MVP_PROTOCOLS.len(), 7);
    }

    #[test]
    fn protocol_smoke_report_passes() {
        let report = protocol_smoke_report();

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.required_protocols, 7);
        assert_eq!(report.registered_protocols, 7);
    }

    #[test]
    fn can_lookup_canonical_wayland_global() {
        let protocol = lookup_protocol("xdg_wm_base").unwrap();

        assert_eq!(protocol.display_name, "xdg-shell");
        assert!(protocol.mvp_required);
    }
}
