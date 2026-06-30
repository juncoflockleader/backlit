use std::env;
use std::process;
use std::thread;
use std::time::Duration;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_launcher::LaunchTarget;
use backlit_settings_daemon::PowerAction;
use backlit_shell::run_shell_chrome_smoke;
use backlit_shell_protocol::{ShellSurfaceRole, MVP_SHELL_ROLES};

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-shell: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut socket = String::from("backlit-0");
    let mut component = ComponentSelection::One(ShellSurfaceRole::Panel);
    let mut verify = false;
    let mut idle_probe_ms = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            print_help();
            return Ok(());
        } else if arg == "--verify" {
            verify = true;
        } else if let Some(value) = arg.strip_prefix("--idle-probe-ms=") {
            idle_probe_ms = Some(parse_u64("--idle-probe-ms", value)?);
        } else if arg == "--idle-probe-ms" {
            let value = args
                .next()
                .ok_or_else(|| String::from("missing value for --idle-probe-ms"))?;
            idle_probe_ms = Some(parse_u64("--idle-probe-ms", &value)?);
        } else if let Some(value) = arg.strip_prefix("--socket=") {
            socket = value.to_string();
        } else if arg == "--socket" {
            socket = args
                .next()
                .ok_or_else(|| String::from("missing value for --socket"))?;
        } else if let Some(value) = arg.strip_prefix("--component=") {
            component = ComponentSelection::parse(value)?;
        } else if arg == "--component" {
            let value = args
                .next()
                .ok_or_else(|| String::from("missing value for --component"))?;
            component = ComponentSelection::parse(&value)?;
        } else {
            return Err(format!("unknown flag: {arg}"));
        }
    }

    let components = component.components();
    let report = run_shell_chrome_smoke();
    for role in components {
        emit_component_ready(*role, socket.as_str(), report.role_ready(*role));
    }

    let passed = report.passed();
    println!(
        "{}",
        event_json(
            "shell.verified",
            &[
                ("socket", FieldValue::Str(socket.as_str())),
                ("passed", FieldValue::Bool(passed)),
                ("verify", FieldValue::Bool(verify)),
                (
                    "required_components",
                    FieldValue::U64(components.len() as u64)
                ),
                ("required_roles", FieldValue::U64(report.required_roles)),
                (
                    "wallpaper_ready",
                    FieldValue::Bool(report.wallpaper.ready())
                ),
                ("panel_ready", FieldValue::Bool(report.panel.ready())),
                ("launcher_ready", FieldValue::Bool(report.launcher.ready())),
                (
                    "app_switcher_ready",
                    FieldValue::Bool(report.app_switcher.ready())
                ),
                (
                    "lock_screen_ready",
                    FieldValue::Bool(report.lock_screen.ready())
                ),
                (
                    "clock_visible",
                    FieldValue::Bool(report.panel.clock_visible)
                ),
                (
                    "battery_visible",
                    FieldValue::Bool(report.panel.battery_visible)
                ),
                (
                    "network_visible",
                    FieldValue::Bool(report.panel.network_visible)
                ),
                (
                    "volume_visible",
                    FieldValue::Bool(report.panel.volume_visible)
                ),
                (
                    "power_menu_ready",
                    FieldValue::Bool(report.panel.power_menu.ready())
                ),
                (
                    "power_menu_visible",
                    FieldValue::Bool(report.panel.power_menu.visible)
                ),
                (
                    "power_menu_actions",
                    FieldValue::U64(report.panel.power_menu.action_count())
                ),
                (
                    "power_menu_lock",
                    FieldValue::Bool(report.panel.power_menu.has_action(PowerAction::Lock))
                ),
                (
                    "power_menu_logout",
                    FieldValue::Bool(report.panel.power_menu.has_action(PowerAction::Logout))
                ),
                (
                    "power_menu_reboot",
                    FieldValue::Bool(report.panel.power_menu.has_action(PowerAction::Reboot))
                ),
                (
                    "power_menu_shutdown",
                    FieldValue::Bool(report.panel.power_menu.has_action(PowerAction::Shutdown))
                ),
                (
                    "network_status_ready",
                    FieldValue::Bool(report.panel.network.ready())
                ),
                (
                    "network_backend",
                    FieldValue::Str(report.panel.network.backend)
                ),
                (
                    "network_control_tool",
                    FieldValue::Str(report.panel.network.control_tool)
                ),
                (
                    "network_connected",
                    FieldValue::Bool(report.panel.network.connected)
                ),
                (
                    "network_strength_percent",
                    FieldValue::U64(report.panel.network.strength_percent)
                ),
                (
                    "audio_status_ready",
                    FieldValue::Bool(report.panel.audio.ready())
                ),
                ("audio_backend", FieldValue::Str(report.panel.audio.backend)),
                (
                    "audio_control_tool",
                    FieldValue::Str(report.panel.audio.control_tool)
                ),
                ("audio_muted", FieldValue::Bool(report.panel.audio.muted)),
                (
                    "audio_volume_percent",
                    FieldValue::U64(report.panel.audio.volume_percent)
                ),
                (
                    "workspace_indicator_visible",
                    FieldValue::Bool(report.panel.workspace.visible)
                ),
                (
                    "workspace_count",
                    FieldValue::U64(report.panel.workspace.count)
                ),
                (
                    "active_workspace",
                    FieldValue::U64(report.panel.workspace.active)
                ),
                (
                    "launcher_targets",
                    FieldValue::U64(report.launcher.target_count())
                ),
                (
                    "terminal_target",
                    FieldValue::Bool(report.launcher.has_target(LaunchTarget::Terminal))
                ),
                (
                    "browser_target",
                    FieldValue::Bool(report.launcher.has_target(LaunchTarget::Browser))
                ),
                (
                    "settings_target",
                    FieldValue::Bool(report.launcher.has_target(LaunchTarget::Settings))
                ),
                (
                    "app_switcher_entries",
                    FieldValue::U64(report.app_switcher.entry_count())
                ),
                (
                    "lock_screen_covers_output",
                    FieldValue::Bool(report.lock_screen.covers_output)
                ),
                (
                    "lock_screen_unlock_prompt_visible",
                    FieldValue::Bool(report.lock_screen.unlock_prompt_visible)
                ),
                (
                    "lock_screen_password_field_focused",
                    FieldValue::Bool(report.lock_screen.password_field_focused)
                ),
            ],
        )
    );

    if verify && !passed {
        return Err(String::from("shell chrome verification failed"));
    }

    if let Some(duration_ms) = idle_probe_ms {
        println!(
            "{}",
            event_json(
                "shell.idle_probe_start",
                &[
                    ("socket", FieldValue::Str(socket.as_str())),
                    ("duration_ms", FieldValue::U64(duration_ms)),
                    ("components", FieldValue::U64(components.len() as u64)),
                ],
            )
        );
        thread::sleep(Duration::from_millis(duration_ms));
        println!(
            "{}",
            event_json(
                "shell.idle_probe_complete",
                &[
                    ("socket", FieldValue::Str(socket.as_str())),
                    ("duration_ms", FieldValue::U64(duration_ms)),
                    ("components", FieldValue::U64(components.len() as u64)),
                ],
            )
        );
    }

    Ok(())
}

fn emit_component_ready(role: ShellSurfaceRole, socket: &str, connected: bool) {
    println!(
        "{}",
        event_json(
            "shell.component_ready",
            &[
                ("component", FieldValue::Str(role.as_str())),
                ("socket", FieldValue::Str(socket)),
                ("mvp_required", FieldValue::Bool(role.mvp_required())),
                ("connected", FieldValue::Bool(connected)),
            ],
        )
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComponentSelection {
    One(ShellSurfaceRole),
    All,
}

impl ComponentSelection {
    fn parse(value: &str) -> Result<Self, String> {
        if value == "all" {
            Ok(Self::All)
        } else {
            value.parse().map(Self::One)
        }
    }

    fn components(self) -> &'static [ShellSurfaceRole] {
        match self {
            Self::One(role) => match role {
                ShellSurfaceRole::Wallpaper => &[ShellSurfaceRole::Wallpaper],
                ShellSurfaceRole::Panel => &[ShellSurfaceRole::Panel],
                ShellSurfaceRole::Launcher => &[ShellSurfaceRole::Launcher],
                ShellSurfaceRole::AppSwitcher => &[ShellSurfaceRole::AppSwitcher],
                ShellSurfaceRole::NotificationHost => &[ShellSurfaceRole::NotificationHost],
                ShellSurfaceRole::LockScreen => &[ShellSurfaceRole::LockScreen],
            },
            Self::All => MVP_SHELL_ROLES,
        }
    }
}

fn print_help() {
    println!(
        "\
backlit-shell

Usage:
  backlit-shell [--component=all|panel|launcher|wallpaper|app-switcher|notification-host|lock-screen] [--socket=backlit-0] [--verify] [--idle-probe-ms=1000]
"
    );
}

fn parse_u64(flag: &str, value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}
