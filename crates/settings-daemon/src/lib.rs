#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplaySettings {
    pub width: u32,
    pub height: u32,
    pub scale_milli: u32,
    pub refresh_millihz: u32,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            width: 800,
            height: 520,
            scale_milli: 1000,
            refresh_millihz: 60_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputSettings {
    pub keyboard_repeat_delay_ms: u32,
    pub keyboard_repeat_rate_hz: u32,
    pub pointer_accel_milli: i32,
    pub touchpad_tap_to_click: bool,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            keyboard_repeat_delay_ms: 350,
            keyboard_repeat_rate_hz: 30,
            pointer_accel_milli: 0,
            touchpad_tap_to_click: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerAction {
    Lock,
    Logout,
    Reboot,
    Shutdown,
    Suspend,
    Ignore,
}

impl PowerAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lock => "lock",
            Self::Logout => "logout",
            Self::Reboot => "reboot",
            Self::Shutdown => "shutdown",
            Self::Suspend => "suspend",
            Self::Ignore => "ignore",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerSettings {
    pub idle_dim_minutes: u32,
    pub idle_suspend_minutes: u32,
    pub lid_action: PowerAction,
}

impl Default for PowerSettings {
    fn default() -> Self {
        Self {
            idle_dim_minutes: 5,
            idle_suspend_minutes: 20,
            lid_action: PowerAction::Suspend,
        }
    }
}

pub const DEFAULT_POWER_MENU: &[PowerAction] = &[
    PowerAction::Lock,
    PowerAction::Logout,
    PowerAction::Reboot,
    PowerAction::Shutdown,
];

pub const REQUIRED_POWER_ACTIONS: &[PowerAction] = &[
    PowerAction::Lock,
    PowerAction::Logout,
    PowerAction::Suspend,
    PowerAction::Reboot,
    PowerAction::Shutdown,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerActionCommand {
    pub action: PowerAction,
    pub program: &'static str,
    pub args: &'static [&'static str],
    pub requires_session_id: bool,
    pub disruptive: bool,
}

impl PowerActionCommand {
    pub fn command_line(self) -> String {
        if self.args.is_empty() {
            self.program.to_string()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

pub const POWER_ACTION_COMMANDS: &[PowerActionCommand] = &[
    PowerActionCommand {
        action: PowerAction::Lock,
        program: "loginctl",
        args: &["lock-session"],
        requires_session_id: false,
        disruptive: false,
    },
    PowerActionCommand {
        action: PowerAction::Logout,
        program: "loginctl",
        args: &["terminate-session", "$XDG_SESSION_ID"],
        requires_session_id: true,
        disruptive: true,
    },
    PowerActionCommand {
        action: PowerAction::Suspend,
        program: "systemctl",
        args: &["suspend"],
        requires_session_id: false,
        disruptive: true,
    },
    PowerActionCommand {
        action: PowerAction::Reboot,
        program: "systemctl",
        args: &["reboot"],
        requires_session_id: false,
        disruptive: true,
    },
    PowerActionCommand {
        action: PowerAction::Shutdown,
        program: "systemctl",
        args: &["poweroff"],
        requires_session_id: false,
        disruptive: true,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerActionDispatch {
    pub command: PowerActionCommand,
    pub dry_run: bool,
}

impl PowerActionDispatch {
    pub const fn would_execute(self) -> bool {
        !self.dry_run
    }
}

pub fn power_action_command(action: PowerAction) -> Option<PowerActionCommand> {
    POWER_ACTION_COMMANDS
        .iter()
        .copied()
        .find(|command| command.action == action)
}

pub fn prepare_power_action(action: PowerAction, dry_run: bool) -> Option<PowerActionDispatch> {
    power_action_command(action).map(|command| PowerActionDispatch { command, dry_run })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsValidationError {
    DisplayMode,
    DisplayScale,
    DisplayRefresh,
    KeyboardRepeat,
    PointerAcceleration,
    PowerIdlePolicy,
}

impl SettingsValidationError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DisplayMode => "display-mode",
            Self::DisplayScale => "display-scale",
            Self::DisplayRefresh => "display-refresh",
            Self::KeyboardRepeat => "keyboard-repeat",
            Self::PointerAcceleration => "pointer-acceleration",
            Self::PowerIdlePolicy => "power-idle-policy",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsDaemonState {
    pub display: DisplaySettings,
    pub input: InputSettings,
    pub power: PowerSettings,
    pub generation: u64,
}

impl SettingsDaemonState {
    pub fn apply_display(
        &mut self,
        settings: DisplaySettings,
    ) -> Result<(), SettingsValidationError> {
        validate_display(settings)?;
        self.display = settings;
        self.generation += 1;
        Ok(())
    }

    pub fn apply_input(&mut self, settings: InputSettings) -> Result<(), SettingsValidationError> {
        validate_input(settings)?;
        self.input = settings;
        self.generation += 1;
        Ok(())
    }

    pub fn apply_power(&mut self, settings: PowerSettings) -> Result<(), SettingsValidationError> {
        validate_power(settings)?;
        self.power = settings;
        self.generation += 1;
        Ok(())
    }
}

pub fn validate_display(settings: DisplaySettings) -> Result<(), SettingsValidationError> {
    if settings.width < 320 || settings.height < 200 {
        return Err(SettingsValidationError::DisplayMode);
    }

    if !(500..=4000).contains(&settings.scale_milli) {
        return Err(SettingsValidationError::DisplayScale);
    }

    if !(30_000..=240_000).contains(&settings.refresh_millihz) {
        return Err(SettingsValidationError::DisplayRefresh);
    }

    Ok(())
}

pub fn validate_input(settings: InputSettings) -> Result<(), SettingsValidationError> {
    if !(100..=2000).contains(&settings.keyboard_repeat_delay_ms)
        || !(1..=120).contains(&settings.keyboard_repeat_rate_hz)
    {
        return Err(SettingsValidationError::KeyboardRepeat);
    }

    if !(-1000..=1000).contains(&settings.pointer_accel_milli) {
        return Err(SettingsValidationError::PointerAcceleration);
    }

    Ok(())
}

pub fn validate_power(settings: PowerSettings) -> Result<(), SettingsValidationError> {
    if settings.idle_suspend_minutes != 0
        && settings.idle_suspend_minutes < settings.idle_dim_minutes
    {
        return Err(SettingsValidationError::PowerIdlePolicy);
    }

    if settings.idle_suspend_minutes > 180 || settings.idle_dim_minutes > 180 {
        return Err(SettingsValidationError::PowerIdlePolicy);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsSmokeReport {
    pub display_validated: bool,
    pub input_validated: bool,
    pub power_validated: bool,
    pub invalid_display_rejected: bool,
    pub invalid_input_rejected: bool,
    pub invalid_power_rejected: bool,
    pub power_menu_complete: bool,
    pub power_menu_actions: u64,
    pub power_action_commands_complete: bool,
    pub power_action_commands: u64,
    pub power_actions_dry_run: bool,
    pub disruptive_power_actions_guarded: bool,
    pub lock_action_ready: bool,
    pub logout_action_ready: bool,
    pub suspend_action_ready: bool,
    pub reboot_action_ready: bool,
    pub shutdown_action_ready: bool,
    pub logout_requires_session_id: bool,
    pub state_generation: u64,
}

impl SettingsSmokeReport {
    pub fn passed(self) -> bool {
        self.display_validated
            && self.input_validated
            && self.power_validated
            && self.invalid_display_rejected
            && self.invalid_input_rejected
            && self.invalid_power_rejected
            && self.power_menu_complete
            && self.power_menu_actions == 4
            && self.power_action_commands_complete
            && self.power_action_commands == 5
            && self.power_actions_dry_run
            && self.disruptive_power_actions_guarded
            && self.lock_action_ready
            && self.logout_action_ready
            && self.suspend_action_ready
            && self.reboot_action_ready
            && self.shutdown_action_ready
            && self.logout_requires_session_id
            && self.state_generation == 3
    }
}

pub fn run_settings_smoke() -> SettingsSmokeReport {
    let mut state = SettingsDaemonState::default();

    let display = DisplaySettings {
        width: 1920,
        height: 1080,
        scale_milli: 1000,
        refresh_millihz: 60_000,
    };
    let input = InputSettings {
        keyboard_repeat_delay_ms: 280,
        keyboard_repeat_rate_hz: 40,
        pointer_accel_milli: -100,
        touchpad_tap_to_click: true,
    };
    let power = PowerSettings {
        idle_dim_minutes: 3,
        idle_suspend_minutes: 15,
        lid_action: PowerAction::Suspend,
    };

    let display_validated = state.apply_display(display).is_ok() && state.display == display;
    let input_validated = state.apply_input(input).is_ok() && state.input == input;
    let power_validated = state.apply_power(power).is_ok() && state.power == power;

    let invalid_display_rejected = matches!(
        state.apply_display(DisplaySettings {
            width: 200,
            height: 120,
            ..display
        }),
        Err(SettingsValidationError::DisplayMode)
    );
    let invalid_input_rejected = matches!(
        state.apply_input(InputSettings {
            pointer_accel_milli: 1500,
            ..input
        }),
        Err(SettingsValidationError::PointerAcceleration)
    );
    let invalid_power_rejected = matches!(
        state.apply_power(PowerSettings {
            idle_dim_minutes: 30,
            idle_suspend_minutes: 10,
            ..power
        }),
        Err(SettingsValidationError::PowerIdlePolicy)
    );

    let lock_command = power_action_command(PowerAction::Lock);
    let logout_command = power_action_command(PowerAction::Logout);
    let suspend_command = power_action_command(PowerAction::Suspend);
    let reboot_command = power_action_command(PowerAction::Reboot);
    let shutdown_command = power_action_command(PowerAction::Shutdown);
    let dry_run_dispatches: Vec<PowerActionDispatch> = REQUIRED_POWER_ACTIONS
        .iter()
        .copied()
        .filter_map(|action| prepare_power_action(action, true))
        .collect();
    let power_action_commands_complete = REQUIRED_POWER_ACTIONS
        .iter()
        .copied()
        .all(|action| power_action_command(action).is_some())
        && POWER_ACTION_COMMANDS.len() == REQUIRED_POWER_ACTIONS.len();
    let power_actions_dry_run = dry_run_dispatches.len() == REQUIRED_POWER_ACTIONS.len()
        && dry_run_dispatches
            .iter()
            .all(|dispatch| dispatch.dry_run && !dispatch.would_execute());
    let disruptive_power_actions_guarded = dry_run_dispatches
        .iter()
        .filter(|dispatch| dispatch.command.disruptive)
        .count()
        == 4
        && dry_run_dispatches
            .iter()
            .filter(|dispatch| dispatch.command.disruptive)
            .all(|dispatch| dispatch.dry_run);

    SettingsSmokeReport {
        display_validated,
        input_validated,
        power_validated,
        invalid_display_rejected,
        invalid_input_rejected,
        invalid_power_rejected,
        power_menu_complete: DEFAULT_POWER_MENU
            == [
                PowerAction::Lock,
                PowerAction::Logout,
                PowerAction::Reboot,
                PowerAction::Shutdown,
            ],
        power_menu_actions: DEFAULT_POWER_MENU.len() as u64,
        power_action_commands_complete,
        power_action_commands: POWER_ACTION_COMMANDS.len() as u64,
        power_actions_dry_run,
        disruptive_power_actions_guarded,
        lock_action_ready: matches!(
            lock_command,
            Some(PowerActionCommand {
                program: "loginctl",
                args: ["lock-session"],
                ..
            })
        ),
        logout_action_ready: matches!(
            logout_command,
            Some(PowerActionCommand {
                program: "loginctl",
                args: ["terminate-session", "$XDG_SESSION_ID"],
                ..
            })
        ),
        suspend_action_ready: matches!(
            suspend_command,
            Some(PowerActionCommand {
                program: "systemctl",
                args: ["suspend"],
                ..
            })
        ),
        reboot_action_ready: matches!(
            reboot_command,
            Some(PowerActionCommand {
                program: "systemctl",
                args: ["reboot"],
                ..
            })
        ),
        shutdown_action_ready: matches!(
            shutdown_command,
            Some(PowerActionCommand {
                program: "systemctl",
                args: ["poweroff"],
                ..
            })
        ),
        logout_requires_session_id: logout_command
            .map(|command| command.requires_session_id)
            .unwrap_or(false),
        state_generation: state.generation,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        power_action_command, prepare_power_action, run_settings_smoke, DisplaySettings,
        InputSettings, PowerAction, PowerSettings, SettingsDaemonState, SettingsValidationError,
    };

    #[test]
    fn applies_valid_settings_and_tracks_generation() {
        let mut state = SettingsDaemonState::default();

        state
            .apply_display(DisplaySettings {
                width: 1366,
                height: 768,
                scale_milli: 1000,
                refresh_millihz: 60_000,
            })
            .expect("display should validate");
        state
            .apply_input(InputSettings {
                keyboard_repeat_delay_ms: 300,
                keyboard_repeat_rate_hz: 35,
                pointer_accel_milli: 250,
                touchpad_tap_to_click: true,
            })
            .expect("input should validate");
        state
            .apply_power(PowerSettings {
                idle_dim_minutes: 5,
                idle_suspend_minutes: 30,
                lid_action: PowerAction::Suspend,
            })
            .expect("power should validate");

        assert_eq!(state.generation, 3);
    }

    #[test]
    fn rejects_unsafe_settings_without_incrementing_generation() {
        let mut state = SettingsDaemonState::default();

        let error = state
            .apply_display(DisplaySettings {
                width: 240,
                ..DisplaySettings::default()
            })
            .expect_err("tiny display mode should be rejected");
        assert_eq!(error, SettingsValidationError::DisplayMode);
        assert_eq!(state.generation, 0);

        let error = state
            .apply_input(InputSettings {
                keyboard_repeat_rate_hz: 0,
                ..InputSettings::default()
            })
            .expect_err("zero keyboard repeat should be rejected");
        assert_eq!(error, SettingsValidationError::KeyboardRepeat);
        assert_eq!(state.generation, 0);

        let error = state
            .apply_power(PowerSettings {
                idle_dim_minutes: 20,
                idle_suspend_minutes: 5,
                ..PowerSettings::default()
            })
            .expect_err("suspend before dim should be rejected");
        assert_eq!(error, SettingsValidationError::PowerIdlePolicy);
        assert_eq!(state.generation, 0);
    }

    #[test]
    fn settings_smoke_passes() {
        assert!(run_settings_smoke().passed());
    }

    #[test]
    fn maps_power_actions_to_logind_and_systemd_commands() {
        let lock = power_action_command(PowerAction::Lock).expect("lock command");
        let logout = power_action_command(PowerAction::Logout).expect("logout command");
        let suspend = power_action_command(PowerAction::Suspend).expect("suspend command");
        let reboot = power_action_command(PowerAction::Reboot).expect("reboot command");
        let shutdown = power_action_command(PowerAction::Shutdown).expect("shutdown command");

        assert_eq!(lock.command_line(), "loginctl lock-session");
        assert_eq!(
            logout.command_line(),
            "loginctl terminate-session $XDG_SESSION_ID"
        );
        assert!(logout.requires_session_id);
        assert_eq!(suspend.command_line(), "systemctl suspend");
        assert_eq!(reboot.command_line(), "systemctl reboot");
        assert_eq!(shutdown.command_line(), "systemctl poweroff");
        assert!(power_action_command(PowerAction::Ignore).is_none());
    }

    #[test]
    fn prepares_power_actions_as_dry_run_until_execution_is_requested() {
        let dry_run = prepare_power_action(PowerAction::Shutdown, true).expect("shutdown dispatch");
        assert!(dry_run.dry_run);
        assert!(!dry_run.would_execute());

        let executable =
            prepare_power_action(PowerAction::Shutdown, false).expect("shutdown dispatch");
        assert!(!executable.dry_run);
        assert!(executable.would_execute());
    }
}
