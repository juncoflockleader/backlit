use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_surface::run_surface_lifecycle_smoke;

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-surface: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    let report = run_surface_lifecycle_smoke();
    println!(
        "{}",
        event_json(
            "surface.lifecycle",
            &[
                ("passed", FieldValue::Bool(report.passed()),),
                (
                    "xdg_shell_registered",
                    FieldValue::Bool(report.xdg_shell_registered),
                ),
                (
                    "created_toplevel",
                    FieldValue::Bool(report.created_toplevel),
                ),
                (
                    "initial_configured",
                    FieldValue::Bool(report.initial_configured),
                ),
                (
                    "ack_configure_ok",
                    FieldValue::Bool(report.ack_configure_ok),
                ),
                ("mapped_window", FieldValue::Bool(report.mapped_window)),
                (
                    "focused_after_map",
                    FieldValue::Bool(report.focused_after_map),
                ),
                (
                    "maximize_configured",
                    FieldValue::Bool(report.maximize_configured),
                ),
                (
                    "maximize_uses_work_area",
                    FieldValue::Bool(report.maximize_uses_work_area),
                ),
                (
                    "fullscreen_configured",
                    FieldValue::Bool(report.fullscreen_configured),
                ),
                (
                    "fullscreen_uses_output",
                    FieldValue::Bool(report.fullscreen_uses_output),
                ),
                ("close_requested", FieldValue::Bool(report.close_requested)),
                ("window_removed", FieldValue::Bool(report.window_removed)),
                (
                    "windows_after_close",
                    FieldValue::U64(report.windows_after_close),
                ),
            ],
        )
    );

    if config.verify && !report.passed() {
        return Err(String::from("surface lifecycle smoke verification failed"));
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
backlit-surface

Usage:
  backlit-surface [--verify]

Flags:
  --verify  Fail if xdg-style toplevel lifecycle smoke checks fail.
"
    );
}
