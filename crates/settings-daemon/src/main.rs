use std::env;
use std::process;
use std::thread;
use std::time::Duration;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_settings_daemon::run_settings_smoke;

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-settings-daemon: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut verify = false;
    let mut help = false;
    let mut idle_probe_ms = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            help = true;
        } else if arg == "--verify" {
            verify = true;
        } else if let Some(value) = arg.strip_prefix("--idle-probe-ms=") {
            idle_probe_ms = Some(parse_u64("--idle-probe-ms", value)?);
        } else if arg == "--idle-probe-ms" {
            let value = args
                .next()
                .ok_or_else(|| String::from("missing value for --idle-probe-ms"))?;
            idle_probe_ms = Some(parse_u64("--idle-probe-ms", &value)?);
        } else {
            return Err(format!("unknown flag: {arg}"));
        }
    }

    if help {
        print_help();
        return Ok(());
    }

    let report = run_settings_smoke();

    println!(
        "{}",
        event_json(
            "settings_daemon.verified",
            &[
                ("passed", FieldValue::Bool(report.passed())),
                ("verify", FieldValue::Bool(verify)),
                (
                    "display_validated",
                    FieldValue::Bool(report.display_validated),
                ),
                ("input_validated", FieldValue::Bool(report.input_validated)),
                ("power_validated", FieldValue::Bool(report.power_validated)),
                (
                    "invalid_display_rejected",
                    FieldValue::Bool(report.invalid_display_rejected),
                ),
                (
                    "invalid_input_rejected",
                    FieldValue::Bool(report.invalid_input_rejected),
                ),
                (
                    "invalid_power_rejected",
                    FieldValue::Bool(report.invalid_power_rejected),
                ),
                (
                    "power_menu_complete",
                    FieldValue::Bool(report.power_menu_complete),
                ),
                (
                    "power_menu_actions",
                    FieldValue::U64(report.power_menu_actions),
                ),
                ("state_generation", FieldValue::U64(report.state_generation),),
            ],
        )
    );

    if verify && !report.passed() {
        return Err(String::from("settings daemon verification failed"));
    }

    if let Some(duration_ms) = idle_probe_ms {
        println!(
            "{}",
            event_json(
                "settings_daemon.idle_probe_start",
                &[("duration_ms", FieldValue::U64(duration_ms))],
            )
        );
        thread::sleep(Duration::from_millis(duration_ms));
        println!(
            "{}",
            event_json(
                "settings_daemon.idle_probe_complete",
                &[("duration_ms", FieldValue::U64(duration_ms))],
            )
        );
    }

    Ok(())
}

fn parse_u64(flag: &str, value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn print_help() {
    println!(
        "\
backlit-settings-daemon

Usage:
  backlit-settings-daemon [--verify] [--idle-probe-ms=1000]

Flags:
  --verify  Fail if display, input, and power settings policy verification fails.
"
    );
}
