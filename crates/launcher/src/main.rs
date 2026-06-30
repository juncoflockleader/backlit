use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_launcher::{default_catalog, verify_catalog, LaunchTarget};

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-launcher: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    let catalog = default_catalog();

    if config.list {
        for command in &catalog {
            println!(
                "{}",
                event_json(
                    "launcher.entry",
                    &[
                        ("target", FieldValue::Str(command.target.as_str())),
                        ("shortcut", FieldValue::Str(command.target.shortcut())),
                        ("program", FieldValue::Str(command.program)),
                        ("command", FieldValue::Str(command.shell_words().as_str())),
                    ],
                )
            );
        }
    }

    if let Some(target) = config.target {
        let command = catalog
            .iter()
            .find(|command| command.target == target)
            .ok_or_else(|| format!("missing launch target {}", target.as_str()))?;

        println!(
            "{}",
            event_json(
                "launcher.resolve",
                &[
                    ("target", FieldValue::Str(target.as_str())),
                    ("dry_run", FieldValue::Bool(true)),
                    ("program", FieldValue::Str(command.program)),
                    ("command", FieldValue::Str(command.shell_words().as_str())),
                ],
            )
        );
    }

    let report = verify_catalog(&catalog);
    println!(
        "{}",
        event_json(
            "launcher.verified",
            &[
                ("passed", FieldValue::Bool(report.passed())),
                (
                    "required_targets",
                    FieldValue::U64(report.required_targets as u64),
                ),
                (
                    "command_count",
                    FieldValue::U64(report.command_count as u64)
                ),
                (
                    "missing_targets",
                    FieldValue::U64(report.missing_targets.len() as u64),
                ),
                (
                    "empty_programs",
                    FieldValue::U64(report.empty_programs.len() as u64),
                ),
            ],
        )
    );

    if config.verify && !report.passed() {
        return Err(String::from("launcher catalog verification failed"));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Config {
    target: Option<LaunchTarget>,
    list: bool,
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
        let mut args = args.into_iter().map(Into::into);

        while let Some(arg) = args.next() {
            if arg == "--help" || arg == "-h" {
                config.help = true;
            } else if arg == "--verify" {
                config.verify = true;
            } else if arg == "--list" {
                config.list = true;
            } else if let Some(value) = arg.strip_prefix("--target=") {
                config.target = Some(parse_target(value)?);
            } else if arg == "--target" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --target"))?;
                config.target = Some(parse_target(&value)?);
            } else {
                return Err(format!("unknown flag: {arg}"));
            }
        }

        Ok(config)
    }
}

fn parse_target(value: &str) -> Result<LaunchTarget, String> {
    value.parse()
}

fn print_help() {
    println!(
        "\
backlit-launcher

Usage:
  backlit-launcher [--verify] [--list] [--target=terminal|browser|settings]

Flags:
  --verify  Fail if the required launch catalog is incomplete.
  --list    Emit the required launch catalog as JSON.
  --target  Resolve a single target in dry-run mode.
"
    );
}
