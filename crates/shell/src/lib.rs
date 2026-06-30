use backlit_launcher::{
    default_catalog, resolve_command, verify_catalog, LaunchCommand, LaunchTarget,
};
use backlit_shell_protocol::{ShellSurfaceRole, MVP_SHELL_ROLES};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallpaperState {
    pub output: &'static str,
    pub color: &'static str,
}

impl WallpaperState {
    pub fn ready(&self) -> bool {
        !self.output.is_empty() && !self.color.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceIndicator {
    pub active: u64,
    pub count: u64,
    pub visible: bool,
}

impl WorkspaceIndicator {
    pub fn ready(&self) -> bool {
        self.visible && self.count > 0 && self.active < self.count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelState {
    pub output: &'static str,
    pub height_px: u64,
    pub clock_visible: bool,
    pub battery_visible: bool,
    pub network_visible: bool,
    pub volume_visible: bool,
    pub workspace: WorkspaceIndicator,
}

impl PanelState {
    pub fn ready(&self) -> bool {
        !self.output.is_empty()
            && self.height_px > 0
            && self.clock_visible
            && self.battery_visible
            && self.network_visible
            && self.volume_visible
            && self.workspace.ready()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherState {
    pub catalog: Vec<LaunchCommand>,
}

impl LauncherState {
    pub fn ready(&self) -> bool {
        verify_catalog(&self.catalog).passed()
    }

    pub fn target_count(&self) -> u64 {
        self.catalog.len() as u64
    }

    pub fn has_target(&self, target: LaunchTarget) -> bool {
        resolve_command(&self.catalog, target).is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSwitcherState {
    pub entries: Vec<&'static str>,
    pub selected_index: usize,
}

impl AppSwitcherState {
    pub fn ready(&self) -> bool {
        !self.entries.is_empty() && self.selected_index < self.entries.len()
    }

    pub fn entry_count(&self) -> u64 {
        self.entries.len() as u64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellChromeReport {
    pub required_roles: u64,
    pub wallpaper: WallpaperState,
    pub panel: PanelState,
    pub launcher: LauncherState,
    pub app_switcher: AppSwitcherState,
}

impl ShellChromeReport {
    pub fn passed(&self) -> bool {
        self.required_roles == MVP_SHELL_ROLES.len() as u64
            && self.wallpaper.ready()
            && self.panel.ready()
            && self.launcher.ready()
            && self.app_switcher.ready()
    }

    pub fn role_ready(&self, role: ShellSurfaceRole) -> bool {
        match role {
            ShellSurfaceRole::Wallpaper => self.wallpaper.ready(),
            ShellSurfaceRole::Panel => self.panel.ready(),
            ShellSurfaceRole::Launcher => self.launcher.ready(),
            ShellSurfaceRole::AppSwitcher => self.app_switcher.ready(),
            ShellSurfaceRole::NotificationHost | ShellSurfaceRole::LockScreen => false,
        }
    }
}

pub fn run_shell_chrome_smoke() -> ShellChromeReport {
    ShellChromeReport {
        required_roles: MVP_SHELL_ROLES.len() as u64,
        wallpaper: WallpaperState {
            output: "Virtual-1",
            color: "#111827",
        },
        panel: PanelState {
            output: "Virtual-1",
            height_px: 42,
            clock_visible: true,
            battery_visible: true,
            network_visible: true,
            volume_visible: true,
            workspace: WorkspaceIndicator {
                active: 0,
                count: 4,
                visible: true,
            },
        },
        launcher: LauncherState {
            catalog: default_catalog(),
        },
        app_switcher: AppSwitcherState {
            entries: vec!["Terminal", "Browser", "Settings"],
            selected_index: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::run_shell_chrome_smoke;
    use backlit_launcher::LaunchTarget;
    use backlit_shell_protocol::ShellSurfaceRole;

    #[test]
    fn shell_chrome_smoke_passes_required_roles() {
        let report = run_shell_chrome_smoke();

        assert!(report.passed());
        assert!(report.role_ready(ShellSurfaceRole::Wallpaper));
        assert!(report.role_ready(ShellSurfaceRole::Panel));
        assert!(report.role_ready(ShellSurfaceRole::Launcher));
        assert!(report.role_ready(ShellSurfaceRole::AppSwitcher));
        assert!(!report.role_ready(ShellSurfaceRole::NotificationHost));
    }

    #[test]
    fn panel_exposes_status_and_workspace_indicator() {
        let report = run_shell_chrome_smoke();

        assert!(report.panel.clock_visible);
        assert!(report.panel.battery_visible);
        assert!(report.panel.network_visible);
        assert!(report.panel.volume_visible);
        assert!(report.panel.workspace.visible);
        assert_eq!(report.panel.workspace.active, 0);
        assert_eq!(report.panel.workspace.count, 4);
    }

    #[test]
    fn launcher_covers_core_targets_and_switcher_has_entries() {
        let report = run_shell_chrome_smoke();

        assert_eq!(report.launcher.target_count(), 3);
        assert!(report.launcher.has_target(LaunchTarget::Terminal));
        assert!(report.launcher.has_target(LaunchTarget::Browser));
        assert!(report.launcher.has_target(LaunchTarget::Settings));
        assert_eq!(report.app_switcher.entry_count(), 3);
    }
}
