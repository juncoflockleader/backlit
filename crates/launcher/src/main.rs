use std::env;
use std::process::{self, Command};

use backlit_common::metrics::{event_json, FieldValue};
use backlit_launcher::{
    default_catalog, default_desktop_entry_dirs, discover_desktop_entries_in_dirs, resolve_command,
    verify_catalog, LaunchCommand, LaunchTarget,
};

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
        let command = resolve_command(&catalog, target)
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

    if config.spawn_smoke {
        let target = config.target.unwrap_or(LaunchTarget::Terminal);
        let command = resolve_command(&catalog, target)
            .ok_or_else(|| format!("missing launch target {}", target.as_str()))?;
        let report = run_spawn_smoke(command, &config)?;

        println!(
            "{}",
            event_json(
                "launcher.spawn",
                &[
                    ("target", FieldValue::Str(target.as_str())),
                    ("dry_run", FieldValue::Bool(false)),
                    ("program", FieldValue::Str(report.program.as_str())),
                    ("spawned", FieldValue::Bool(report.spawned)),
                    ("exit_success", FieldValue::Bool(report.exit_success)),
                    ("status_code", FieldValue::U64(report.status_code)),
                    (
                        "wayland_display_set",
                        FieldValue::Bool(report.wayland_display_set),
                    ),
                ],
            )
        );

        if config.verify && !report.passed() {
            return Err(String::from("launcher spawn smoke failed"));
        }
    }

    let (desktop_entries, desktop_dirs, default_desktop_dirs) = if config.no_desktop_discovery {
        (0, 0, false)
    } else {
        let (dirs, default_desktop_dirs) = config.desktop_dirs();
        let desktop_dirs = dirs.len();
        let entries = discover_desktop_entries_in_dirs(&dirs)
            .map_err(|error| format!("failed to discover desktop entries: {error}"))?;

        for entry in &entries {
            println!(
                "{}",
                event_json(
                    "launcher.desktop_entry",
                    &[
                        ("id", FieldValue::Str(entry.id.as_str())),
                        ("name", FieldValue::Str(entry.name.as_str())),
                        ("program", FieldValue::Str(entry.command_program())),
                        ("terminal", FieldValue::Bool(entry.terminal)),
                    ],
                )
            );
        }

        (entries.len(), desktop_dirs, default_desktop_dirs)
    };

    println!(
        "{}",
        event_json(
            "launcher.desktop_discovery",
            &[
                ("enabled", FieldValue::Bool(!config.no_desktop_discovery),),
                ("default_dirs", FieldValue::Bool(default_desktop_dirs)),
                ("dirs", FieldValue::U64(desktop_dirs as u64)),
                ("entries", FieldValue::U64(desktop_entries as u64)),
                ("required", FieldValue::Bool(config.require_desktop_entries),),
            ],
        )
    );

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
                ("desktop_entries", FieldValue::U64(desktop_entries as u64)),
                ("desktop_dirs", FieldValue::U64(desktop_dirs as u64)),
                (
                    "host_desktop_discovery",
                    FieldValue::Bool(default_desktop_dirs),
                ),
            ],
        )
    );

    if config.verify && (!report.passed() || config.require_desktop_entries && desktop_entries == 0)
    {
        return Err(String::from("launcher catalog verification failed"));
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Config {
    target: Option<LaunchTarget>,
    desktop_dirs: Vec<String>,
    spawn_program: Option<String>,
    spawn_args: Vec<String>,
    wayland_display: Option<String>,
    no_desktop_discovery: bool,
    require_desktop_entries: bool,
    spawn_smoke: bool,
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
            } else if arg == "--spawn-smoke" {
                config.spawn_smoke = true;
            } else if let Some(value) = arg.strip_prefix("--target=") {
                config.target = Some(parse_target(value)?);
            } else if arg == "--target" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --target"))?;
                config.target = Some(parse_target(&value)?);
            } else if let Some(value) = arg.strip_prefix("--desktop-dir=") {
                config.desktop_dirs.push(value.to_string());
            } else if arg == "--desktop-dir" {
                config.desktop_dirs.push(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --desktop-dir"))?,
                );
            } else if arg == "--no-desktop-discovery" {
                config.no_desktop_discovery = true;
            } else if arg == "--require-desktop-entries" {
                config.require_desktop_entries = true;
            } else if let Some(value) = arg.strip_prefix("--spawn-program=") {
                config.spawn_program = Some(value.to_string());
            } else if arg == "--spawn-program" {
                config.spawn_program = Some(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --spawn-program"))?,
                );
            } else if let Some(value) = arg.strip_prefix("--spawn-arg=") {
                config.spawn_args.push(value.to_string());
            } else if arg == "--spawn-arg" {
                config.spawn_args.push(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --spawn-arg"))?,
                );
            } else if let Some(value) = arg.strip_prefix("--wayland-display=") {
                config.wayland_display = Some(value.to_string());
            } else if arg == "--wayland-display" {
                config.wayland_display = Some(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --wayland-display"))?,
                );
            } else {
                return Err(format!("unknown flag: {arg}"));
            }
        }

        Ok(config)
    }

    fn desktop_dirs(&self) -> (Vec<std::path::PathBuf>, bool) {
        if self.desktop_dirs.is_empty() {
            (default_desktop_entry_dirs(), true)
        } else {
            (
                self.desktop_dirs
                    .iter()
                    .map(std::path::PathBuf::from)
                    .collect(),
                false,
            )
        }
    }
}

fn parse_target(value: &str) -> Result<LaunchTarget, String> {
    value.parse()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpawnReport {
    program: String,
    spawned: bool,
    exit_success: bool,
    status_code: u64,
    wayland_display_set: bool,
}

impl SpawnReport {
    fn passed(&self) -> bool {
        self.spawned && self.exit_success
    }
}

fn run_spawn_smoke(command: &LaunchCommand, config: &Config) -> Result<SpawnReport, String> {
    let program = config
        .spawn_program
        .clone()
        .unwrap_or_else(|| command.program.to_string());
    let args: Vec<String> = if config.spawn_program.is_some() {
        config.spawn_args.clone()
    } else {
        command.args.iter().map(|arg| (*arg).to_string()).collect()
    };
    let wayland_display = config
        .wayland_display
        .clone()
        .or_else(|| env::var("WAYLAND_DISPLAY").ok());

    let mut child = Command::new(&program);
    child.args(&args);
    if let Some(display) = &wayland_display {
        child.env("WAYLAND_DISPLAY", display);
    }

    let status = child
        .status()
        .map_err(|error| format!("failed to spawn {program}: {error}"))?;

    Ok(SpawnReport {
        program,
        spawned: true,
        exit_success: status.success(),
        status_code: status.code().unwrap_or(255) as u64,
        wayland_display_set: wayland_display.is_some(),
    })
}

fn print_help() {
    println!(
        "\
backlit-launcher

Usage:
  backlit-launcher [--verify] [--list] [--target=terminal|browser|settings] [--desktop-dir=DIR] [--spawn-smoke]

Flags:
  --verify  Fail if the required launch catalog is incomplete.
  --list    Emit the required launch catalog as JSON.
  --target  Resolve a single target in dry-run mode.
  --desktop-dir  Discover visible .desktop application entries from DIR. May repeat. Defaults to XDG app dirs.
  --no-desktop-discovery  Skip .desktop application discovery.
  --require-desktop-entries  Fail verification if no visible .desktop entries are discovered.
  --spawn-smoke  Spawn the selected launch target or override program and verify it exits successfully.
  --spawn-program  Program override for deterministic spawn verification.
  --spawn-arg  Argument for the spawn program override. May be passed more than once.
  --wayland-display  WAYLAND_DISPLAY value to pass to the spawned process.
"
    );
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn parses_spawn_smoke_flags() {
        let config = Config::parse([
            "--target",
            "terminal",
            "--spawn-smoke",
            "--spawn-program",
            "true",
            "--desktop-dir",
            "crates/launcher/fixtures",
            "--require-desktop-entries",
            "--spawn-arg",
            "--help",
            "--wayland-display",
            "wayland-1",
        ])
        .expect("config should parse");

        assert!(config.spawn_smoke);
        assert_eq!(config.spawn_program.as_deref(), Some("true"));
        assert_eq!(config.desktop_dirs, ["crates/launcher/fixtures"]);
        assert!(config.require_desktop_entries);
        assert_eq!(config.spawn_args, ["--help"]);
        assert_eq!(config.wayland_display.as_deref(), Some("wayland-1"));
    }
}
