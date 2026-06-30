use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_shortcuts::{resolve_shortcut, verify_shortcuts, MVP_SHORTCUTS};

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-shortcuts: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    if config.list {
        for binding in MVP_SHORTCUTS {
            println!(
                "{}",
                event_json(
                    "shortcut.binding",
                    &[
                        ("shortcut", FieldValue::Str(binding.shortcut)),
                        ("action", FieldValue::Str(binding.action.as_str())),
                        ("mvp_required", FieldValue::Bool(binding.mvp_required)),
                    ],
                )
            );
        }
    }

    if let Some(shortcut) = config.resolve.as_deref() {
        let action = resolve_shortcut(shortcut);
        println!(
            "{}",
            event_json(
                "shortcut.resolve",
                &[
                    ("shortcut", FieldValue::Str(shortcut)),
                    ("resolved", FieldValue::Bool(action.is_some()),),
                    (
                        "action",
                        FieldValue::Str(action.map(|action| action.as_str()).unwrap_or("")),
                    ),
                ],
            )
        );
    }

    let report = verify_shortcuts(MVP_SHORTCUTS);
    println!(
        "{}",
        event_json(
            "shortcut.verified",
            &[
                ("passed", FieldValue::Bool(report.passed())),
                (
                    "required_bindings",
                    FieldValue::U64(report.required_bindings as u64),
                ),
                (
                    "binding_count",
                    FieldValue::U64(report.binding_count as u64)
                ),
                (
                    "duplicate_shortcuts",
                    FieldValue::U64(report.duplicate_shortcuts.len() as u64),
                ),
                (
                    "missing_actions",
                    FieldValue::U64(report.missing_actions.len() as u64),
                ),
            ],
        )
    );

    if config.verify && !report.passed() {
        return Err(String::from("shortcut verification failed"));
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Config {
    list: bool,
    verify: bool,
    resolve: Option<String>,
    help: bool,
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
            } else if arg == "--list" {
                config.list = true;
            } else if let Some(value) = arg.strip_prefix("--resolve=") {
                config.resolve = Some(value.to_string());
            } else if arg == "--resolve" {
                config.resolve = Some(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --resolve"))?,
                );
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
backlit-shortcuts

Usage:
  backlit-shortcuts [--verify] [--list] [--resolve=Super+Enter]

Flags:
  --verify   Fail if the required shortcut map is incomplete.
  --list     Emit shortcut bindings as JSON.
  --resolve  Resolve one shortcut in dry-run mode.
"
    );
}
