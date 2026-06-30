use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_shell_protocol::ShellSurfaceRole;

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-shell: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut socket = String::from("backlit-0");
    let mut component = ShellSurfaceRole::Panel;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            print_help();
            return Ok(());
        } else if let Some(value) = arg.strip_prefix("--socket=") {
            socket = value.to_string();
        } else if arg == "--socket" {
            socket = args
                .next()
                .ok_or_else(|| String::from("missing value for --socket"))?;
        } else if let Some(value) = arg.strip_prefix("--component=") {
            component = value.parse().map_err(|error: String| error)?;
        } else if arg == "--component" {
            let value = args
                .next()
                .ok_or_else(|| String::from("missing value for --component"))?;
            component = value.parse().map_err(|error: String| error)?;
        } else {
            return Err(format!("unknown flag: {arg}"));
        }
    }

    println!(
        "{}",
        event_json(
            "shell.stub_ready",
            &[
                ("component", FieldValue::Str(component.as_str())),
                ("socket", FieldValue::Str(socket.as_str())),
                ("connected", FieldValue::Bool(false)),
            ],
        )
    );

    Ok(())
}

fn print_help() {
    println!(
        "\
backlit-shell

Usage:
  backlit-shell [--component=panel|launcher|wallpaper|notification-host|lock-screen] [--socket=backlit-0]
"
    );
}
