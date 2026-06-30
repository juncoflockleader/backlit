use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_portal_backend::run_portal_security_smoke;

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-portal-backend: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut verify = false;
    let mut help = false;

    for arg in env::args().skip(1) {
        if arg == "--help" || arg == "-h" {
            help = true;
        } else if arg == "--verify" {
            verify = true;
        } else {
            return Err(format!("unknown flag: {arg}"));
        }
    }

    if help {
        print_help();
        return Ok(());
    }

    let report = run_portal_security_smoke();

    println!(
        "{}",
        event_json(
            "portal_backend.security_smoke",
            &[
                ("passed", FieldValue::Bool(report.passed())),
                (
                    "direct_screenshot_denied",
                    FieldValue::Bool(report.direct_screenshot_denied),
                ),
                (
                    "direct_screencast_denied",
                    FieldValue::Bool(report.direct_screencast_denied),
                ),
                (
                    "direct_remote_desktop_denied",
                    FieldValue::Bool(report.direct_remote_desktop_denied),
                ),
                (
                    "unconsented_portal_denied",
                    FieldValue::Bool(report.unconsented_portal_denied),
                ),
                (
                    "consented_screenshot_allowed",
                    FieldValue::Bool(report.consented_screenshot_allowed),
                ),
                (
                    "consented_screencast_allowed",
                    FieldValue::Bool(report.consented_screencast_allowed),
                ),
                (
                    "file_chooser_allowed",
                    FieldValue::Bool(report.file_chooser_allowed),
                ),
            ],
        )
    );

    if verify && !report.passed() {
        return Err(String::from("portal security smoke verification failed"));
    }

    Ok(())
}

fn print_help() {
    println!(
        "\
backlit-portal-backend

Usage:
  backlit-portal-backend [--verify]

Flags:
  --verify  Fail if direct privileged capture is not denied or consented portal flows fail.
"
    );
}
