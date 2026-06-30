use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_input::run_input_smoke;

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-input: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    let report = run_input_smoke();
    println!(
        "{}",
        event_json(
            "input.smoke",
            &[
                ("passed", FieldValue::Bool(report.passed())),
                (
                    "terminal_launch_resolved",
                    FieldValue::Bool(report.terminal_launch_resolved),
                ),
                (
                    "windows_after_terminal_launch",
                    FieldValue::U64(report.windows_after_terminal_launch),
                ),
                (
                    "app_switcher_changed_focus",
                    FieldValue::Bool(report.app_switcher_changed_focus),
                ),
                (
                    "pointer_focus_window",
                    FieldValue::Bool(report.pointer_focus_window),
                ),
                (
                    "pointer_move_window",
                    FieldValue::Bool(report.pointer_move_window),
                ),
                (
                    "pointer_resize_window",
                    FieldValue::Bool(report.pointer_resize_window),
                ),
                (
                    "pointer_grab_ended",
                    FieldValue::Bool(report.pointer_grab_ended),
                ),
                ("final_focus", FieldValue::U64(report.final_focus)),
                ("final_width", FieldValue::U64(report.final_width)),
                ("final_height", FieldValue::U64(report.final_height)),
            ],
        )
    );

    if config.verify && !report.passed() {
        return Err(String::from("input smoke verification failed"));
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
backlit-input

Usage:
  backlit-input [--verify]

Flags:
  --verify  Fail if keyboard and pointer routing smoke checks fail.
"
    );
}
