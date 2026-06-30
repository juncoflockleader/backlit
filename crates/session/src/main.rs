use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::str::FromStr;
use std::time::Instant;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_compositor_backend::BackendKind;
use backlit_demo_client::{
    render_demo_gui, verify_demo_gui, DEFAULT_DEMO_HEIGHT, DEFAULT_DEMO_WIDTH,
};
use backlit_launcher::{default_catalog, LaunchTarget};
use backlit_shortcuts::{resolve_shortcut, ShortcutAction};
use backlit_window_policy::{OutputLayout, WindowPolicy, WindowState};

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-session: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    if config.backend == BackendKind::Drm && !cfg!(target_os = "linux") {
        return Err(String::from(
            "the drm backend requires Linux with a real graphics/input stack",
        ));
    }

    let started = Instant::now();
    emit(
        "session.launch",
        &config,
        &[
            ("verify", FieldValue::Bool(config.verify)),
            ("verify_services", FieldValue::Bool(config.verify_services)),
        ],
    );

    let mut policy = WindowPolicy::default();
    policy.add_window("terminal", (800, 600));
    policy.add_window("settings", (720, 560));
    policy.add_window("browser", (1100, 720));
    let layout = OutputLayout::new(config.width as i32, config.height as i32, 42);

    let screenshot = config
        .screenshot
        .clone()
        .unwrap_or_else(|| String::from("target/backlit-session.ppm"));
    let canvas = render_demo_gui(config.width, config.height);
    canvas
        .write_ppm(&screenshot)
        .map_err(|error| format!("failed to write {screenshot}: {error}"))?;

    emit(
        "session.gui_ready",
        &config,
        &[
            ("screenshot", FieldValue::Str(screenshot.as_str())),
            ("windows", FieldValue::U64(policy.windows().len() as u64)),
            ("width", FieldValue::U64(canvas.width() as u64)),
            ("height", FieldValue::U64(canvas.height() as u64)),
            ("work_area_y", FieldValue::U64(layout.work_area().y as u64)),
            (
                "work_area_height",
                FieldValue::U64(layout.work_area().height as u64),
            ),
            ("checksum", FieldValue::U64(canvas.checksum())),
        ],
    );

    if config.verify {
        let report = verify_demo_gui(&canvas);
        let interaction_report = verify_session_interactions(&policy, layout);

        emit(
            "session.interactions",
            &config,
            &[
                ("passed", FieldValue::Bool(interaction_report.passed())),
                (
                    "initial_focus",
                    FieldValue::U64(interaction_report.initial_focus),
                ),
                (
                    "focus_after_switcher",
                    FieldValue::U64(interaction_report.focus_after_switcher),
                ),
                (
                    "windows_after_launch",
                    FieldValue::U64(interaction_report.windows_after_launch),
                ),
                (
                    "terminal_launch_resolved",
                    FieldValue::Bool(interaction_report.terminal_launch_resolved),
                ),
                (
                    "move_resize_ok",
                    FieldValue::Bool(interaction_report.move_resize_ok),
                ),
                (
                    "minimize_skips_focus",
                    FieldValue::Bool(interaction_report.minimize_skips_focus),
                ),
                (
                    "focus_after_minimize",
                    FieldValue::U64(interaction_report.focus_after_minimize),
                ),
                ("moved_x", FieldValue::U64(interaction_report.moved_x)),
                (
                    "resized_width",
                    FieldValue::U64(interaction_report.resized_width),
                ),
                (
                    "maximize_uses_work_area",
                    FieldValue::Bool(interaction_report.maximize_uses_work_area),
                ),
                (
                    "fullscreen_uses_output",
                    FieldValue::Bool(interaction_report.fullscreen_uses_output),
                ),
                (
                    "close_fallback_focus_ok",
                    FieldValue::Bool(interaction_report.close_fallback_focus_ok),
                ),
                (
                    "windows_after_close",
                    FieldValue::U64(interaction_report.windows_after_close),
                ),
            ],
        );

        emit(
            "session.verified",
            &config,
            &[
                ("passed", FieldValue::Bool(report.passed())),
                (
                    "non_background_pixels",
                    FieldValue::U64(report.non_background_pixels),
                ),
                ("checksum", FieldValue::U64(report.checksum)),
                ("golden_ok", FieldValue::Bool(report.golden_ok)),
                ("panel_ok", FieldValue::Bool(report.panel_ok)),
                ("launcher_ok", FieldValue::Bool(report.launcher_ok)),
                ("window_ok", FieldValue::Bool(report.window_ok)),
                ("pointer_ok", FieldValue::Bool(report.pointer_ok)),
            ],
        );

        if !report.passed() || !interaction_report.passed() {
            return Err(String::from("headless GUI verification failed"));
        }
    }

    if config.verify_services {
        let service_report = verify_session_services(&config)?;
        emit_service_verification(&config, &service_report);

        if !service_report.passed() {
            return Err(String::from("session service verification failed"));
        }
    }

    emit(
        "session.exit",
        &config,
        &[(
            "elapsed_ms",
            FieldValue::U64(started.elapsed().as_millis() as u64),
        )],
    );

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ServiceProbe {
    resolved: bool,
    exit_ok: bool,
    ready: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl ServiceProbe {
    fn missing() -> Self {
        Self {
            resolved: false,
            exit_ok: false,
            ready: false,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    fn stdout_contains(&self, needle: &str) -> bool {
        String::from_utf8_lossy(&self.stdout).contains(needle)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ServiceVerification {
    compositor: ServiceProbe,
    shell: ServiceProbe,
    logs_written: bool,
}

impl ServiceVerification {
    fn passed(&self) -> bool {
        self.compositor.resolved
            && self.compositor.exit_ok
            && self.compositor.ready
            && self.shell.resolved
            && self.shell.exit_ok
            && self.shell.ready
    }

    fn children_exited_cleanly(&self) -> bool {
        self.compositor.exit_ok && self.shell.exit_ok
    }
}

fn verify_session_services(config: &Config) -> Result<ServiceVerification, String> {
    let compositor_path = sibling_binary("backlit-compositor");
    let shell_path = sibling_binary("backlit-shell");

    let compositor = run_compositor_probe(&compositor_path, config)?;
    let shell = run_shell_probe(&shell_path, config)?;
    let mut report = ServiceVerification {
        compositor,
        shell,
        logs_written: false,
    };

    if let Some(log_dir) = &config.service_log_dir {
        write_service_logs(Path::new(log_dir), &report)?;
        report.logs_written = true;
    }

    Ok(report)
}

fn sibling_binary(name: &str) -> PathBuf {
    let binary_name = binary_name(name);

    if let Ok(current_exe) = env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            return parent.join(binary_name);
        }
    }

    PathBuf::from(binary_name)
}

fn binary_name(name: &str) -> String {
    format!("{name}{}", env::consts::EXE_SUFFIX)
}

fn run_compositor_probe(path: &Path, config: &Config) -> Result<ServiceProbe, String> {
    let backend_event = format!("\"backend\":\"{}\"", config.backend.as_str());

    run_service_probe(
        path,
        &[
            "--backend",
            config.backend.as_str(),
            "--socket",
            config.socket.as_str(),
            "--smoke-test",
        ],
        &[
            String::from("\"event\":\"compositor.smoke_test\""),
            backend_event,
        ],
    )
}

fn run_shell_probe(path: &Path, config: &Config) -> Result<ServiceProbe, String> {
    run_service_probe(
        path,
        &[
            "--component",
            "all",
            "--socket",
            config.socket.as_str(),
            "--verify",
        ],
        &[
            String::from("\"event\":\"shell.verified\""),
            String::from("\"passed\":true"),
            String::from("\"required_components\":4"),
        ],
    )
}

fn run_service_probe(
    path: &Path,
    args: &[&str],
    required_stdout: &[String],
) -> Result<ServiceProbe, String> {
    if !path.is_file() {
        return Ok(ServiceProbe::missing());
    }

    let output = Command::new(path)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run {}: {error}", path.display()))?;

    let mut probe = ServiceProbe {
        resolved: true,
        exit_ok: output.status.success(),
        ready: false,
        stdout: output.stdout,
        stderr: output.stderr,
    };
    probe.ready = probe.exit_ok
        && required_stdout
            .iter()
            .all(|needle| probe.stdout_contains(needle.as_str()));

    Ok(probe)
}

fn write_service_logs(log_dir: &Path, report: &ServiceVerification) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|error| format!("failed to create {}: {error}", log_dir.display()))?;
    fs::write(log_dir.join("compositor.jsonl"), &report.compositor.stdout)
        .map_err(|error| format!("failed to write compositor service log: {error}"))?;
    fs::write(log_dir.join("compositor.stderr"), &report.compositor.stderr)
        .map_err(|error| format!("failed to write compositor service stderr: {error}"))?;
    fs::write(log_dir.join("shell.jsonl"), &report.shell.stdout)
        .map_err(|error| format!("failed to write shell service log: {error}"))?;
    fs::write(log_dir.join("shell.stderr"), &report.shell.stderr)
        .map_err(|error| format!("failed to write shell service stderr: {error}"))?;
    Ok(())
}

fn emit_service_verification(config: &Config, report: &ServiceVerification) {
    emit(
        "session.services_verified",
        config,
        &[
            ("passed", FieldValue::Bool(report.passed())),
            (
                "compositor_resolved",
                FieldValue::Bool(report.compositor.resolved),
            ),
            (
                "compositor_ready",
                FieldValue::Bool(report.compositor.ready),
            ),
            ("shell_resolved", FieldValue::Bool(report.shell.resolved)),
            ("shell_ready", FieldValue::Bool(report.shell.ready)),
            (
                "children_exited_cleanly",
                FieldValue::Bool(report.children_exited_cleanly()),
            ),
            ("logs_written", FieldValue::Bool(report.logs_written)),
            (
                "compositor_stdout_bytes",
                FieldValue::U64(report.compositor.stdout.len() as u64),
            ),
            (
                "shell_stdout_bytes",
                FieldValue::U64(report.shell.stdout.len() as u64),
            ),
        ],
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InteractionReport {
    initial_focus: u64,
    focus_after_switcher: u64,
    windows_after_launch: u64,
    terminal_launch_resolved: bool,
    move_resize_ok: bool,
    minimize_skips_focus: bool,
    focus_after_minimize: u64,
    moved_x: u64,
    resized_width: u64,
    maximize_uses_work_area: bool,
    fullscreen_uses_output: bool,
    close_fallback_focus_ok: bool,
    windows_after_close: u64,
}

impl InteractionReport {
    fn passed(self) -> bool {
        self.initial_focus != 0
            && self.focus_after_switcher != 0
            && self.focus_after_switcher != self.initial_focus
            && self.windows_after_launch == 4
            && self.terminal_launch_resolved
            && self.move_resize_ok
            && self.minimize_skips_focus
            && self.maximize_uses_work_area
            && self.fullscreen_uses_output
            && self.close_fallback_focus_ok
            && self.windows_after_close == 3
    }
}

fn verify_session_interactions(policy: &WindowPolicy, layout: OutputLayout) -> InteractionReport {
    let mut policy = policy.clone();
    let initial_focus = policy.focused().map(|id| id.0).unwrap_or(0);

    let focus_after_switcher = match resolve_shortcut("Alt+Tab") {
        Some(ShortcutAction::AppSwitcherNext) => policy.cycle_focus_forward().map(|id| id.0),
        _ => None,
    }
    .unwrap_or(0);

    let terminal_launch_resolved = match resolve_shortcut("Super+Enter") {
        Some(ShortcutAction::Launch(LaunchTarget::Terminal)) => default_catalog()
            .iter()
            .any(|command| command.target == LaunchTarget::Terminal),
        _ => false,
    };

    if terminal_launch_resolved {
        policy.add_window("terminal-2", (800, 600));
    }
    let windows_after_launch = policy.windows().len() as u64;

    let minimized_window = policy.focused();
    let focus_after_minimize = minimized_window
        .and_then(|id| {
            if policy.minimize_window(id) {
                policy.focused()
            } else {
                None
            }
        })
        .map(|id| id.0)
        .unwrap_or(0);
    let minimize_skips_focus = minimized_window
        .map(|id| focus_after_minimize != 0 && focus_after_minimize != id.0)
        .unwrap_or(false);

    let focused = policy.focused();
    let (move_resize_ok, moved_x, resized_width) = focused
        .map(|id| {
            let moved = policy.move_window(id, 96, 84);
            let resized = policy.resize_window(id, 920, 640);
            let geometry = policy.window(id).map(|window| window.geometry);
            let ok = moved
                && resized
                && geometry
                    .map(|geometry| (geometry.x, geometry.y, geometry.width, geometry.height))
                    == Some((96, 84, 920, 640));

            (ok, 96, 920)
        })
        .unwrap_or((false, 0, 0));

    let maximize_uses_work_area = focused
        .map(|id| {
            policy.maximize_window(id, layout.work_area())
                && policy.window(id).map(|window| window.geometry) == Some(layout.work_area())
        })
        .unwrap_or(false);

    let fullscreen_uses_output = focused
        .map(|id| {
            policy.fullscreen_window(id, layout.output)
                && policy.window(id).map(|window| window.geometry) == Some(layout.output)
        })
        .unwrap_or(false);

    let close_fallback_focus_ok = policy.close_focused_window().is_some()
        && policy.focused().is_some()
        && policy
            .focused()
            .and_then(|id| policy.window(id))
            .map(|window| window.state != WindowState::Minimized)
            .unwrap_or(false);
    let windows_after_close = policy.windows().len() as u64;

    InteractionReport {
        initial_focus,
        focus_after_switcher,
        windows_after_launch,
        terminal_launch_resolved,
        move_resize_ok,
        minimize_skips_focus,
        focus_after_minimize,
        moved_x,
        resized_width,
        maximize_uses_work_area,
        fullscreen_uses_output,
        close_fallback_focus_ok,
        windows_after_close,
    }
}

fn emit(event: &str, config: &Config, fields: &[(&str, FieldValue<'_>)]) {
    let mut combined = Vec::with_capacity(fields.len() + 2);
    combined.push(("backend", FieldValue::Str(config.backend.as_str())));
    combined.push(("socket", FieldValue::Str(config.socket.as_str())));
    combined.extend_from_slice(fields);
    println!("{}", event_json(event, &combined));
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Config {
    backend: BackendKind,
    socket: String,
    screenshot: Option<String>,
    service_log_dir: Option<String>,
    width: u32,
    height: u32,
    verify: bool,
    verify_services: bool,
    help: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend: BackendKind::Headless,
            socket: String::from("backlit-0"),
            screenshot: None,
            service_log_dir: None,
            width: DEFAULT_DEMO_WIDTH,
            height: DEFAULT_DEMO_HEIGHT,
            verify: false,
            verify_services: false,
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
            } else if arg == "--verify-services" {
                config.verify_services = true;
            } else if let Some(value) = arg.strip_prefix("--backend=") {
                config.backend = parse_backend(value)?;
            } else if arg == "--backend" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --backend"))?;
                config.backend = parse_backend(&value)?;
            } else if let Some(value) = arg.strip_prefix("--socket=") {
                config.socket = value.to_string();
            } else if arg == "--socket" {
                config.socket = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --socket"))?;
            } else if let Some(value) = arg.strip_prefix("--screenshot=") {
                config.screenshot = Some(value.to_string());
            } else if arg == "--screenshot" {
                config.screenshot = Some(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --screenshot"))?,
                );
            } else if let Some(value) = arg.strip_prefix("--service-log-dir=") {
                config.service_log_dir = Some(value.to_string());
            } else if arg == "--service-log-dir" {
                config.service_log_dir = Some(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --service-log-dir"))?,
                );
            } else if let Some(value) = arg.strip_prefix("--width=") {
                config.width = parse_dimension("--width", value)?;
            } else if arg == "--width" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --width"))?;
                config.width = parse_dimension("--width", &value)?;
            } else if let Some(value) = arg.strip_prefix("--height=") {
                config.height = parse_dimension("--height", value)?;
            } else if arg == "--height" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --height"))?;
                config.height = parse_dimension("--height", &value)?;
            } else {
                return Err(format!("unknown flag: {arg}"));
            }
        }

        Ok(config)
    }
}

fn parse_backend(value: &str) -> Result<BackendKind, String> {
    BackendKind::from_str(value).map_err(|_| format!("invalid backend: {value}"))
}

fn parse_dimension(flag: &str, value: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn print_help() {
    println!(
        "\
backlit-session

Usage:
  backlit-session [--backend=headless|wayland|drm] [--socket=backlit-0] [--screenshot=target/backlit-session.ppm] [--verify] [--verify-services]

Flags:
  --backend      Select compositor backend. Defaults to headless.
  --socket       Wayland socket name. Defaults to backlit-0.
  --screenshot   Write a deterministic PPM GUI screenshot.
  --service-log-dir
                 Write compositor and shell probe logs to this directory.
  --width        Screenshot width in pixels.
  --height       Screenshot height in pixels.
  --verify       Fail if expected GUI regions are missing.
  --verify-services
                 Fail if sibling compositor and shell probes cannot launch.
"
    );
}

#[cfg(test)]
mod tests {
    use super::{binary_name, Config};

    #[test]
    fn parses_service_verification_flags() {
        let config = Config::parse([
            "--verify",
            "--verify-services",
            "--service-log-dir",
            "target/session-services",
        ])
        .expect("config should parse");

        assert!(config.verify);
        assert!(config.verify_services);
        assert_eq!(
            config.service_log_dir.as_deref(),
            Some("target/session-services")
        );
    }

    #[test]
    fn binary_name_uses_platform_suffix() {
        assert!(binary_name("backlit-compositor").starts_with("backlit-compositor"));
    }
}
