#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalCapability {
    Screenshot,
    Screencast,
    FileChooser,
    RemoteDesktop,
}

impl PortalCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Screenshot => "screenshot",
            Self::Screencast => "screencast",
            Self::FileChooser => "file-chooser",
            Self::RemoteDesktop => "remote-desktop",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalRequestPath {
    DirectClient,
    PortalMediated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalRequest {
    pub capability: PortalCapability,
    pub path: PortalRequestPath,
    pub user_consented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalDecision {
    pub allowed: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalSecurityReport {
    pub direct_screenshot_denied: bool,
    pub direct_screencast_denied: bool,
    pub direct_remote_desktop_denied: bool,
    pub unconsented_portal_denied: bool,
    pub consented_screenshot_allowed: bool,
    pub consented_screencast_allowed: bool,
    pub file_chooser_allowed: bool,
}

impl PortalSecurityReport {
    pub fn passed(self) -> bool {
        self.direct_screenshot_denied
            && self.direct_screencast_denied
            && self.direct_remote_desktop_denied
            && self.unconsented_portal_denied
            && self.consented_screenshot_allowed
            && self.consented_screencast_allowed
            && self.file_chooser_allowed
    }
}

pub fn decide_request(request: PortalRequest) -> PortalDecision {
    match request.path {
        PortalRequestPath::DirectClient => PortalDecision {
            allowed: false,
            reason: "requires-portal",
        },
        PortalRequestPath::PortalMediated if !request.user_consented => PortalDecision {
            allowed: false,
            reason: "requires-user-consent",
        },
        PortalRequestPath::PortalMediated => PortalDecision {
            allowed: true,
            reason: "allowed-with-consent",
        },
    }
}

pub fn run_portal_security_smoke() -> PortalSecurityReport {
    let direct_screenshot = decide_request(PortalRequest {
        capability: PortalCapability::Screenshot,
        path: PortalRequestPath::DirectClient,
        user_consented: false,
    });
    let direct_screencast = decide_request(PortalRequest {
        capability: PortalCapability::Screencast,
        path: PortalRequestPath::DirectClient,
        user_consented: false,
    });
    let direct_remote_desktop = decide_request(PortalRequest {
        capability: PortalCapability::RemoteDesktop,
        path: PortalRequestPath::DirectClient,
        user_consented: false,
    });
    let unconsented_portal = decide_request(PortalRequest {
        capability: PortalCapability::Screenshot,
        path: PortalRequestPath::PortalMediated,
        user_consented: false,
    });
    let consented_screenshot = decide_request(PortalRequest {
        capability: PortalCapability::Screenshot,
        path: PortalRequestPath::PortalMediated,
        user_consented: true,
    });
    let consented_screencast = decide_request(PortalRequest {
        capability: PortalCapability::Screencast,
        path: PortalRequestPath::PortalMediated,
        user_consented: true,
    });
    let file_chooser = decide_request(PortalRequest {
        capability: PortalCapability::FileChooser,
        path: PortalRequestPath::PortalMediated,
        user_consented: true,
    });

    PortalSecurityReport {
        direct_screenshot_denied: !direct_screenshot.allowed
            && direct_screenshot.reason == "requires-portal",
        direct_screencast_denied: !direct_screencast.allowed
            && direct_screencast.reason == "requires-portal",
        direct_remote_desktop_denied: !direct_remote_desktop.allowed
            && direct_remote_desktop.reason == "requires-portal",
        unconsented_portal_denied: !unconsented_portal.allowed
            && unconsented_portal.reason == "requires-user-consent",
        consented_screenshot_allowed: consented_screenshot.allowed
            && consented_screenshot.reason == "allowed-with-consent",
        consented_screencast_allowed: consented_screencast.allowed
            && consented_screencast.reason == "allowed-with-consent",
        file_chooser_allowed: file_chooser.allowed && file_chooser.reason == "allowed-with-consent",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decide_request, run_portal_security_smoke, PortalCapability, PortalRequest,
        PortalRequestPath,
    };

    #[test]
    fn direct_privileged_capture_is_denied() {
        let decision = decide_request(PortalRequest {
            capability: PortalCapability::Screenshot,
            path: PortalRequestPath::DirectClient,
            user_consented: true,
        });

        assert!(!decision.allowed);
        assert_eq!(decision.reason, "requires-portal");
    }

    #[test]
    fn portal_requests_require_user_consent() {
        let denied = decide_request(PortalRequest {
            capability: PortalCapability::Screencast,
            path: PortalRequestPath::PortalMediated,
            user_consented: false,
        });
        let allowed = decide_request(PortalRequest {
            capability: PortalCapability::Screencast,
            path: PortalRequestPath::PortalMediated,
            user_consented: true,
        });

        assert!(!denied.allowed);
        assert_eq!(denied.reason, "requires-user-consent");
        assert!(allowed.allowed);
        assert_eq!(allowed.reason, "allowed-with-consent");
    }

    #[test]
    fn portal_security_smoke_passes() {
        assert!(run_portal_security_smoke().passed());
    }
}
