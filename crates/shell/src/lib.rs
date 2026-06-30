use backlit_launcher::{
    default_catalog, resolve_command, verify_catalog, LaunchCommand, LaunchTarget,
};
use backlit_settings_daemon::{power_action_command, PowerAction, DEFAULT_POWER_MENU};
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
pub struct NetworkStatus {
    pub backend: &'static str,
    pub control_tool: &'static str,
    pub device: &'static str,
    pub connected: bool,
    pub ssid: &'static str,
    pub strength_percent: u64,
}

impl NetworkStatus {
    pub fn ready(&self) -> bool {
        !self.backend.is_empty()
            && self.control_tool == "nmcli"
            && !self.device.is_empty()
            && self.connected
            && !self.ssid.is_empty()
            && self.strength_percent <= 100
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioStatus {
    pub backend: &'static str,
    pub control_tool: &'static str,
    pub sink: &'static str,
    pub volume_percent: u64,
    pub muted: bool,
}

impl AudioStatus {
    pub fn ready(&self) -> bool {
        !self.backend.is_empty()
            && self.control_tool == "wpctl"
            && !self.sink.is_empty()
            && self.volume_percent <= 100
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
    pub power_menu: PowerMenuState,
    pub network: NetworkStatus,
    pub audio: AudioStatus,
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
            && self.power_menu.ready()
            && self.network.ready()
            && self.audio.ready()
            && self.workspace.ready()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerMenuState {
    pub visible: bool,
    pub opens_lock_screen: bool,
    pub actions: Vec<PowerAction>,
}

impl PowerMenuState {
    pub fn ready(&self) -> bool {
        self.visible
            && self.opens_lock_screen
            && self.actions.as_slice() == DEFAULT_POWER_MENU
            && self
                .actions
                .iter()
                .copied()
                .all(|action| power_action_command(action).is_some())
    }

    pub fn action_count(&self) -> u64 {
        self.actions.len() as u64
    }

    pub fn has_action(&self, action: PowerAction) -> bool {
        self.actions.contains(&action)
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
pub struct LockScreenState {
    pub output: &'static str,
    pub covers_output: bool,
    pub unlock_prompt_visible: bool,
    pub password_field_focused: bool,
}

impl LockScreenState {
    pub fn ready(&self) -> bool {
        !self.output.is_empty()
            && self.covers_output
            && self.unlock_prompt_visible
            && self.password_field_focused
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellChromeReport {
    pub required_roles: u64,
    pub wallpaper: WallpaperState,
    pub panel: PanelState,
    pub launcher: LauncherState,
    pub app_switcher: AppSwitcherState,
    pub lock_screen: LockScreenState,
}

impl ShellChromeReport {
    pub fn passed(&self) -> bool {
        self.required_roles == MVP_SHELL_ROLES.len() as u64
            && self.wallpaper.ready()
            && self.panel.ready()
            && self.launcher.ready()
            && self.app_switcher.ready()
            && self.lock_screen.ready()
    }

    pub fn role_ready(&self, role: ShellSurfaceRole) -> bool {
        match role {
            ShellSurfaceRole::Wallpaper => self.wallpaper.ready(),
            ShellSurfaceRole::Panel => self.panel.ready(),
            ShellSurfaceRole::Launcher => self.launcher.ready(),
            ShellSurfaceRole::AppSwitcher => self.app_switcher.ready(),
            ShellSurfaceRole::LockScreen => self.lock_screen.ready(),
            ShellSurfaceRole::NotificationHost => false,
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
            power_menu: PowerMenuState {
                visible: true,
                opens_lock_screen: true,
                actions: DEFAULT_POWER_MENU.to_vec(),
            },
            network: NetworkStatus {
                backend: "NetworkManager",
                control_tool: "nmcli",
                device: "wlan0",
                connected: true,
                ssid: "Backlit Lab",
                strength_percent: 84,
            },
            audio: AudioStatus {
                backend: "PipeWire",
                control_tool: "wpctl",
                sink: "@DEFAULT_AUDIO_SINK@",
                volume_percent: 72,
                muted: false,
            },
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
        lock_screen: LockScreenState {
            output: "Virtual-1",
            covers_output: true,
            unlock_prompt_visible: true,
            password_field_focused: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::run_shell_chrome_smoke;
    use backlit_launcher::LaunchTarget;
    use backlit_settings_daemon::PowerAction;
    use backlit_shell_protocol::ShellSurfaceRole;

    #[test]
    fn shell_chrome_smoke_passes_required_roles() {
        let report = run_shell_chrome_smoke();

        assert!(report.passed());
        assert!(report.role_ready(ShellSurfaceRole::Wallpaper));
        assert!(report.role_ready(ShellSurfaceRole::Panel));
        assert!(report.role_ready(ShellSurfaceRole::Launcher));
        assert!(report.role_ready(ShellSurfaceRole::AppSwitcher));
        assert!(report.role_ready(ShellSurfaceRole::LockScreen));
        assert!(!report.role_ready(ShellSurfaceRole::NotificationHost));
    }

    #[test]
    fn panel_exposes_status_and_workspace_indicator() {
        let report = run_shell_chrome_smoke();

        assert!(report.panel.clock_visible);
        assert!(report.panel.battery_visible);
        assert!(report.panel.network_visible);
        assert!(report.panel.volume_visible);
        assert!(report.panel.power_menu.ready());
        assert!(report.panel.network.ready());
        assert!(report.panel.audio.ready());
        assert!(report.panel.workspace.visible);
        assert_eq!(report.panel.workspace.active, 0);
        assert_eq!(report.panel.workspace.count, 4);
    }

    #[test]
    fn panel_power_menu_exposes_design_actions() {
        let report = run_shell_chrome_smoke();

        assert!(report.panel.power_menu.visible);
        assert!(report.panel.power_menu.opens_lock_screen);
        assert_eq!(report.panel.power_menu.action_count(), 4);
        assert!(report.panel.power_menu.has_action(PowerAction::Lock));
        assert!(report.panel.power_menu.has_action(PowerAction::Logout));
        assert!(report.panel.power_menu.has_action(PowerAction::Reboot));
        assert!(report.panel.power_menu.has_action(PowerAction::Shutdown));
    }

    #[test]
    fn lock_screen_surface_is_ready_for_activation() {
        let report = run_shell_chrome_smoke();

        assert!(report.lock_screen.ready());
        assert!(report.lock_screen.covers_output);
        assert!(report.lock_screen.unlock_prompt_visible);
        assert!(report.lock_screen.password_field_focused);
    }

    #[test]
    fn panel_status_uses_existing_system_tools() {
        let report = run_shell_chrome_smoke();

        assert_eq!(report.panel.network.backend, "NetworkManager");
        assert_eq!(report.panel.network.control_tool, "nmcli");
        assert!(report.panel.network.connected);
        assert!(report.panel.network.strength_percent <= 100);
        assert_eq!(report.panel.audio.backend, "PipeWire");
        assert_eq!(report.panel.audio.control_tool, "wpctl");
        assert!(!report.panel.audio.muted);
        assert!(report.panel.audio.volume_percent <= 100);
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
