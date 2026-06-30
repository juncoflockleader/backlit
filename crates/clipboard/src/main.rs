use std::env;
use std::process;

use backlit_clipboard::run_clipboard_smoke;
use backlit_common::metrics::{event_json, FieldValue};

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-clipboard: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    let report = run_clipboard_smoke();
    println!(
        "{}",
        event_json(
            "clipboard.smoke",
            &[
                ("passed", FieldValue::Bool(report.passed())),
                ("set_ok", FieldValue::Bool(report.set_ok)),
                ("replace_ok", FieldValue::Bool(report.replace_ok)),
                ("clear_ok", FieldValue::Bool(report.clear_ok)),
                ("generation", FieldValue::U64(report.generation)),
            ],
        )
    );

    if config.verify && !report.passed() {
        return Err(String::from("clipboard smoke verification failed"));
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
backlit-clipboard

Usage:
  backlit-clipboard [--verify]

Flags:
  --verify  Fail if the clipboard state smoke check fails.
"
    );
}
