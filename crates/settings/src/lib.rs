use backlit_launcher::{default_catalog, resolve_command, LaunchTarget};
use backlit_settings_daemon::{
    power_action_command, DisplaySettings, InputSettings, PowerAction, PowerSettings,
    SettingsDaemonState, DEFAULT_POWER_MENU, REQUIRED_POWER_ACTIONS,
};

pub const SETTINGS_APPLICATION_ID: &str = "org.backlit.Settings";
pub const REQUIRED_SETTINGS_PANELS: u64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayModeOption {
    pub width: u32,
    pub height: u32,
    pub refresh_millihz: u32,
}

impl DisplayModeOption {
    pub const fn new(width: u32, height: u32, refresh_millihz: u32) -> Self {
        Self {
            width,
            height,
            refresh_millihz,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaySettingsPanel {
    pub output: &'static str,
    pub current: DisplaySettings,
    pub modes: Vec<DisplayModeOption>,
    pub scale_options_milli: Vec<u32>,
    pub apply_validated: bool,
}

impl DisplaySettingsPanel {
    pub fn ready(&self) -> bool {
        !self.output.is_empty()
            && !self.modes.is_empty()
            && !self.scale_options_milli.is_empty()
            && self
                .modes
                .iter()
                .any(|mode| mode.width == self.current.width && mode.height == self.current.height)
            && self.scale_options_milli.contains(&self.current.scale_milli)
            && self.apply_validated
    }

    pub fn mode_count(&self) -> u64 {
        self.modes.len() as u64
    }

    pub fn scale_option_count(&self) -> u64 {
        self.scale_options_milli.len() as u64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSettingsPanel {
    pub keyboard_repeat_visible: bool,
    pub pointer_accel_visible: bool,
    pub touchpad_toggle_visible: bool,
    pub apply_validated: bool,
}

impl InputSettingsPanel {
    pub fn ready(&self) -> bool {
        self.keyboard_repeat_visible
            && self.pointer_accel_visible
            && self.touchpad_toggle_visible
            && self.apply_validated
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerSettingsPanel {
    pub idle_policy_visible: bool,
    pub lid_action_visible: bool,
    pub power_menu_visible: bool,
    pub power_menu_actions: Vec<PowerAction>,
    pub command_plans_available: bool,
    pub apply_validated: bool,
}

impl PowerSettingsPanel {
    pub fn ready(&self) -> bool {
        self.idle_policy_visible
            && self.lid_action_visible
            && self.power_menu_visible
            && self.power_menu_actions.as_slice() == DEFAULT_POWER_MENU
            && self.command_plans_available
            && self.apply_validated
    }

    pub fn power_menu_action_count(&self) -> u64 {
        self.power_menu_actions.len() as u64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsAppReport {
    pub application_id: &'static str,
    pub launcher_target_ready: bool,
    pub required_panels: u64,
    pub display: DisplaySettingsPanel,
    pub input: InputSettingsPanel,
    pub power: PowerSettingsPanel,
    pub daemon_generation: u64,
}

impl SettingsAppReport {
    pub fn passed(&self) -> bool {
        self.application_id == SETTINGS_APPLICATION_ID
            && self.launcher_target_ready
            && self.required_panels == REQUIRED_SETTINGS_PANELS
            && self.display.ready()
            && self.input.ready()
            && self.power.ready()
            && self.daemon_generation == REQUIRED_SETTINGS_PANELS
    }
}

pub fn run_settings_app_smoke() -> SettingsAppReport {
    let mut daemon = SettingsDaemonState::default();
    let launcher_target_ready = resolve_command(&default_catalog(), LaunchTarget::Settings)
        .map(|command| command.program == "backlit-settings")
        .unwrap_or(false);

    let display = DisplaySettings {
        width: 1280,
        height: 720,
        scale_milli: 1000,
        refresh_millihz: 60_000,
    };
    let input = InputSettings {
        keyboard_repeat_delay_ms: 260,
        keyboard_repeat_rate_hz: 35,
        pointer_accel_milli: 100,
        touchpad_tap_to_click: true,
    };
    let power = PowerSettings {
        idle_dim_minutes: 5,
        idle_suspend_minutes: 30,
        lid_action: PowerAction::Suspend,
    };

    let display_apply_validated =
        daemon.apply_display(display).is_ok() && daemon.display == display;
    let input_apply_validated = daemon.apply_input(input).is_ok() && daemon.input == input;
    let power_apply_validated = daemon.apply_power(power).is_ok() && daemon.power == power;
    let command_plans_available = REQUIRED_POWER_ACTIONS
        .iter()
        .copied()
        .all(|action| power_action_command(action).is_some());

    SettingsAppReport {
        application_id: SETTINGS_APPLICATION_ID,
        launcher_target_ready,
        required_panels: REQUIRED_SETTINGS_PANELS,
        display: DisplaySettingsPanel {
            output: "Virtual-1",
            current: display,
            modes: vec![
                DisplayModeOption::new(1280, 720, 60_000),
                DisplayModeOption::new(1920, 1080, 60_000),
                DisplayModeOption::new(2560, 1440, 60_000),
            ],
            scale_options_milli: vec![1000, 1250, 1500, 2000],
            apply_validated: display_apply_validated,
        },
        input: InputSettingsPanel {
            keyboard_repeat_visible: true,
            pointer_accel_visible: true,
            touchpad_toggle_visible: true,
            apply_validated: input_apply_validated,
        },
        power: PowerSettingsPanel {
            idle_policy_visible: true,
            lid_action_visible: true,
            power_menu_visible: true,
            power_menu_actions: DEFAULT_POWER_MENU.to_vec(),
            command_plans_available,
            apply_validated: power_apply_validated,
        },
        daemon_generation: daemon.generation,
    }
}

#[cfg(test)]
mod tests {
    use super::{run_settings_app_smoke, REQUIRED_SETTINGS_PANELS, SETTINGS_APPLICATION_ID};

    #[test]
    fn settings_app_smoke_passes() {
        let report = run_settings_app_smoke();

        assert!(report.passed());
        assert_eq!(report.application_id, SETTINGS_APPLICATION_ID);
        assert_eq!(report.required_panels, REQUIRED_SETTINGS_PANELS);
        assert!(report.launcher_target_ready);
    }

    #[test]
    fn display_panel_exposes_basic_output_controls() {
        let report = run_settings_app_smoke();

        assert!(report.display.ready());
        assert_eq!(report.display.output, "Virtual-1");
        assert_eq!(report.display.mode_count(), 3);
        assert_eq!(report.display.scale_option_count(), 4);
        assert!(report.display.apply_validated);
    }

    #[test]
    fn input_and_power_panels_apply_through_daemon_policy() {
        let report = run_settings_app_smoke();

        assert!(report.input.ready());
        assert!(report.input.keyboard_repeat_visible);
        assert!(report.input.pointer_accel_visible);
        assert!(report.input.touchpad_toggle_visible);
        assert!(report.power.ready());
        assert_eq!(report.power.power_menu_action_count(), 4);
        assert_eq!(report.daemon_generation, 3);
    }
}
