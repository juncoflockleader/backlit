use std::env;
use std::process;
use std::thread;
use std::time::Duration;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_notification_daemon::run_notification_smoke;

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-notification-daemon: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    let report = run_notification_smoke();
    println!(
        "{}",
        event_json(
            "notification_daemon.smoke",
            &[
                ("passed", FieldValue::Bool(report.passed())),
                ("notify_calls", FieldValue::U64(report.notify_calls)),
                (
                    "active_after_replace",
                    FieldValue::U64(report.active_after_replace)
                ),
                (
                    "replacement_preserved_id",
                    FieldValue::Bool(report.replacement_preserved_id)
                ),
                ("action_invoked", FieldValue::Bool(report.action_invoked)),
                ("closed_replaced", FieldValue::Bool(report.closed_replaced)),
                ("closed_expired", FieldValue::Bool(report.closed_expired)),
                (
                    "closed_dismissed",
                    FieldValue::Bool(report.closed_dismissed)
                ),
                (
                    "critical_persistent",
                    FieldValue::Bool(report.critical_persistent)
                ),
                (
                    "spec_fields_valid",
                    FieldValue::Bool(report.spec_fields_valid)
                ),
                (
                    "active_after_cleanup",
                    FieldValue::U64(report.active_after_cleanup)
                ),
            ],
        )
    );

    if config.verify && !report.passed() {
        return Err(String::from("notification daemon smoke failed"));
    }

    if let Some(duration_ms) = config.idle_probe_ms {
        println!(
            "{}",
            event_json(
                "notification_daemon.idle_probe_start",
                &[("duration_ms", FieldValue::U64(duration_ms))],
            )
        );
        thread::sleep(Duration::from_millis(duration_ms));
        println!(
            "{}",
            event_json(
                "notification_daemon.idle_probe_complete",
                &[("duration_ms", FieldValue::U64(duration_ms))],
            )
        );
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Config {
    verify: bool,
    help: bool,
    idle_probe_ms: Option<u64>,
}

impl Config {
    fn parse<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut config = Self::default();
        let mut args = args.into_iter().map(Into::into);

        while let Some(arg) = args.next() {
            if arg == "--help" || arg == "-h" {
                config.help = true;
            } else if arg == "--verify" {
                config.verify = true;
            } else if let Some(value) = arg.strip_prefix("--idle-probe-ms=") {
                config.idle_probe_ms = Some(parse_u64("--idle-probe-ms", value)?);
            } else if arg == "--idle-probe-ms" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --idle-probe-ms"))?;
                config.idle_probe_ms = Some(parse_u64("--idle-probe-ms", &value)?);
            } else {
                return Err(format!("unknown flag: {arg}"));
            }
        }

        Ok(config)
    }
}

fn parse_u64(flag: &str, value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn print_help() {
    println!(
        "\
backlit-notification-daemon

Usage:
  backlit-notification-daemon [--verify] [--idle-probe-ms=1000]

Flags:
  --verify  Fail if notification replacement, actions, persistence, or close reasons regress.
"
    );
}
