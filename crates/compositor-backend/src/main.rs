use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_compositor_backend::{preflight_backend, BackendKind};

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

    let wayland_display = env::var("WAYLAND_DISPLAY").ok();
    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").ok();
    let report = preflight_backend(
        config.backend,
        wayland_display.as_deref(),
        xdg_runtime_dir.as_deref(),
        env::consts::OS,
    );

    println!(
        "{}",
        event_json(
            "backend.preflight",
            &[
                ("backend", FieldValue::Str(report.backend.as_str())),
                ("ready", FieldValue::Bool(report.ready)),
                ("code", FieldValue::Str(report.code)),
                ("detail", FieldValue::Str(report.detail.as_str())),
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
"
    );
}
