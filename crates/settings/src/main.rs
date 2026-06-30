use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_settings::run_settings_app_smoke;

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-settings: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut socket = String::from("backlit-0");
    let mut verify = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            print_help();
            return Ok(());
        } else if arg == "--verify" {
            verify = true;
        } else if let Some(value) = arg.strip_prefix("--socket=") {
            socket = value.to_string();
        } else if arg == "--socket" {
            socket = args
                .next()
                .ok_or_else(|| String::from("missing value for --socket"))?;
        } else {
            return Err(format!("unknown flag: {arg}"));
        }
    }

    let report = run_settings_app_smoke();
    println!(
        "{}",
        event_json(
            "settings_app.verified",
            &[
                ("socket", FieldValue::Str(socket.as_str())),
                ("passed", FieldValue::Bool(report.passed())),
                ("verify", FieldValue::Bool(verify)),
                ("application_id", FieldValue::Str(report.application_id)),
                (
                    "launcher_target_ready",
                    FieldValue::Bool(report.launcher_target_ready)
                ),
                ("required_panels", FieldValue::U64(report.required_panels)),
                (
                    "display_panel_ready",
                    FieldValue::Bool(report.display.ready())
                ),
                ("display_output", FieldValue::Str(report.display.output)),
                (
                    "display_modes",
                    FieldValue::U64(report.display.mode_count())
                ),
                (
                    "display_scale_options",
                    FieldValue::U64(report.display.scale_option_count())
                ),
                (
                    "display_current_width",
                    FieldValue::U64(report.display.current.width as u64)
                ),
                (
                    "display_current_height",
                    FieldValue::U64(report.display.current.height as u64)
                ),
                (
                    "display_current_refresh_millihz",
                    FieldValue::U64(report.display.current.refresh_millihz as u64)
                ),
                (
                    "display_apply_validated",
                    FieldValue::Bool(report.display.apply_validated)
                ),
                ("input_panel_ready", FieldValue::Bool(report.input.ready())),
                (
                    "keyboard_repeat_visible",
                    FieldValue::Bool(report.input.keyboard_repeat_visible)
                ),
                (
                    "pointer_accel_visible",
                    FieldValue::Bool(report.input.pointer_accel_visible)
                ),
                (
                    "touchpad_toggle_visible",
                    FieldValue::Bool(report.input.touchpad_toggle_visible)
                ),
                (
                    "input_apply_validated",
                    FieldValue::Bool(report.input.apply_validated)
                ),
                ("power_panel_ready", FieldValue::Bool(report.power.ready())),
                (
                    "power_idle_policy_visible",
                    FieldValue::Bool(report.power.idle_policy_visible)
                ),
                (
                    "power_lid_action_visible",
                    FieldValue::Bool(report.power.lid_action_visible)
                ),
                (
                    "power_menu_visible",
                    FieldValue::Bool(report.power.power_menu_visible)
                ),
                (
                    "power_menu_actions",
                    FieldValue::U64(report.power.power_menu_action_count())
                ),
                (
                    "power_command_plans_available",
                    FieldValue::Bool(report.power.command_plans_available)
                ),
                (
                    "power_apply_validated",
                    FieldValue::Bool(report.power.apply_validated)
                ),
                (
                    "daemon_generation",
                    FieldValue::U64(report.daemon_generation)
                ),
            ],
        )
    );

    if verify && !report.passed() {
        return Err(String::from("settings app verification failed"));
    }

    Ok(())
}

fn print_help() {
    println!(
        "\
backlit-settings

Usage:
  backlit-settings [--socket=backlit-0] [--verify]

Flags:
  --verify  Fail if display, input, and power settings panels are incomplete.
"
    );
}
