use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_protocols::{protocol_smoke_report, MVP_PROTOCOLS, SHELL_PROTOCOLS};

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-protocols: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut verify = false;
    let mut list = false;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--verify" => verify = true,
            "--list" => list = true,
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => return Err(format!("unknown flag: {other}")),
        }
    }

    if list {
        for protocol in MVP_PROTOCOLS.iter().chain(SHELL_PROTOCOLS) {
            println!(
                "{}",
                event_json(
                    "protocol.registry_entry",
                    &[
                        ("global", FieldValue::Str(protocol.global_name)),
                        ("display_name", FieldValue::Str(protocol.display_name)),
                        ("domain", FieldValue::Str(protocol.domain.as_str())),
                        (
                            "minimum_version",
                            FieldValue::U64(protocol.minimum_version as u64)
                        ),
                        ("mvp_required", FieldValue::Bool(protocol.mvp_required)),
                        ("stage", FieldValue::Str(protocol.stage.as_str())),
                    ],
                )
            );
        }
    }

    let report = protocol_smoke_report();
    println!(
        "{}",
        event_json(
            "protocol.smoke",
            &[
                ("passed", FieldValue::Bool(report.passed())),
                (
                    "required_protocols",
                    FieldValue::U64(report.required_protocols as u64),
                ),
                (
                    "registered_protocols",
                    FieldValue::U64(report.registered_protocols as u64),
                ),
                (
                    "duplicate_globals",
                    FieldValue::U64(report.duplicate_globals.len() as u64),
                ),
                (
                    "missing_required_globals",
                    FieldValue::U64(report.missing_required_globals.len() as u64),
                ),
            ],
        )
    );

    if verify && !report.passed() {
        return Err(String::from("protocol smoke verification failed"));
    }

    Ok(())
}

fn print_help() {
    println!(
        "\
backlit-protocols

Usage:
  backlit-protocols [--verify] [--list]

Flags:
  --verify  Fail when the MVP protocol registry is incomplete.
  --list    Emit each protocol registry entry as JSON.
"
    );
}
