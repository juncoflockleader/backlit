use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_session_supervisor::run_crash_smoke;

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-session-supervisor: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    let report = run_crash_smoke();
    println!(
        "{}",
        event_json(
            "supervisor.crash_smoke",
            &[
                ("passed", FieldValue::Bool(report.passed())),
                (
                    "shell_crash_isolated",
                    FieldValue::Bool(report.shell_crash_isolated),
                ),
                (
                    "compositor_crash_ends_session",
                    FieldValue::Bool(report.compositor_crash_ends_session),
                ),
                ("restarted_shells", FieldValue::U64(report.restarted_shells)),
            ],
        )
    );

    if config.verify && !report.passed() {
        return Err(String::from("session supervisor crash smoke failed"));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Config {
    verify: bool,
    help: bool,
}

impl Config {
    fn parse<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut config = Self::default();

        for arg in args.into_iter().map(Into::into) {
            if arg == "--help" || arg == "-h" {
                config.help = true;
            } else if arg == "--verify" {
                config.verify = true;
            } else {
                return Err(format!("unknown flag: {arg}"));
            }
        }

        Ok(config)
    }
}

fn print_help() {
    println!(
        "\
backlit-session-supervisor

Usage:
  backlit-session-supervisor [--verify]

Flags:
  --verify  Fail if shell crash isolation or compositor crash handling is incorrect.
"
    );
}
