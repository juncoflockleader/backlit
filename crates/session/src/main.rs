use std::env;
use std::process;
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
        &[("verify", FieldValue::Bool(config.verify))],
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
    width: u32,
    height: u32,
    verify: bool,
    help: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend: BackendKind::Headless,
            socket: String::from("backlit-0"),
            screenshot: None,
            width: DEFAULT_DEMO_WIDTH,
            height: DEFAULT_DEMO_HEIGHT,
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
  backlit-session [--backend=headless|wayland|drm] [--socket=backlit-0] [--screenshot=target/backlit-session.ppm] [--verify]

Flags:
  --backend      Select compositor backend. Defaults to headless.
  --socket       Wayland socket name. Defaults to backlit-0.
  --screenshot   Write a deterministic PPM GUI screenshot.
  --width        Screenshot width in pixels.
  --height       Screenshot height in pixels.
  --verify       Fail if expected GUI regions are missing.
"
    );
}
