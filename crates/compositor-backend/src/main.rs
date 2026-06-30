use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_compositor_backend::{
    preflight_backend_with_environment, BackendKind, BackendPreflightEnvironment,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-compositor-backend: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    let environment = BackendPreflightEnvironment::from_host();
    let report = preflight_backend_with_environment(config.backend, &environment);
    let wayland_display = environment.wayland_display.as_deref().unwrap_or("");
    let xdg_runtime_dir = environment.xdg_runtime_dir.as_deref().unwrap_or("");
    let session_id = environment.session_id.as_deref().unwrap_or("");
    let seat = environment.seat.as_deref().unwrap_or("");
    let session_type = environment.session_type.as_deref().unwrap_or("");

    println!(
        "{}",
        event_json(
            "backend.preflight",
            &[
                ("backend", FieldValue::Str(report.backend.as_str())),
                ("ready", FieldValue::Bool(report.ready)),
                ("code", FieldValue::Str(report.code)),
                ("detail", FieldValue::Str(report.detail.as_str())),
                ("target_os", FieldValue::Str(environment.target_os.as_str())),
                ("wayland_display", FieldValue::Str(wayland_display)),
                ("xdg_runtime_dir", FieldValue::Str(xdg_runtime_dir)),
                (
                    "xdg_runtime_dir_present",
                    FieldValue::Bool(environment.xdg_runtime_dir_present),
                ),
                (
                    "xdg_runtime_dir_owned_by_user",
                    FieldValue::Bool(environment.xdg_runtime_dir_owned_by_user),
                ),
                (
                    "drm_card_nodes",
                    FieldValue::U64(environment.drm_card_nodes)
                ),
                (
                    "drm_render_nodes",
                    FieldValue::U64(environment.drm_render_nodes),
                ),
                (
                    "input_event_nodes",
                    FieldValue::U64(environment.input_event_nodes),
                ),
                ("session_id", FieldValue::Str(session_id)),
                ("seat", FieldValue::Str(seat)),
                ("session_type", FieldValue::Str(session_type)),
            ],
        )
    );

    if config.verify && !report.ready {
        return Err(format!(
            "{} backend preflight failed: {}",
            report.backend.as_str(),
            report.code,
        ));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Config {
    backend: BackendKind,
    verify: bool,
    help: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend: BackendKind::Headless,
            verify: false,
            help: false,
        }
    }
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
            } else if let Some(value) = arg.strip_prefix("--backend=") {
                config.backend = parse_backend(value)?;
            } else if arg == "--backend" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --backend"))?;
                config.backend = parse_backend(&value)?;
            } else {
                return Err(format!("unknown flag: {arg}"));
            }
        }

        Ok(config)
    }
}

fn parse_backend(value: &str) -> Result<BackendKind, String> {
    value
        .parse()
        .map_err(|_| format!("invalid backend: {value}"))
}

fn print_help() {
    println!(
        "\
backlit-compositor-backend

Usage:
  backlit-compositor-backend [--backend=headless|wayland|drm] [--verify]

Flags:
  --backend  Backend to preflight. Defaults to headless.
  --verify   Exit non-zero when the requested backend is not ready.

The JSON event includes runtime, DRM, input, and session hints used by
launch-readiness verification.
"
    );
}
