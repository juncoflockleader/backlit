use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::str::FromStr;
use std::time::Instant;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_compositor_backend::{
    preflight_backend_with_environment, BackendKind, BackendPreflightEnvironment,
    BackendPreflightReport,
};
use backlit_demo_client::{
    render_demo_gui, verify_demo_gui, DEFAULT_DEMO_HEIGHT, DEFAULT_DEMO_WIDTH,
};
use backlit_input::run_input_smoke;
use backlit_launcher::{default_catalog, resolve_command, LaunchTarget};
use backlit_shortcuts::{resolve_shortcut, ShortcutAction};
use backlit_surface::run_surface_lifecycle_smoke;
use backlit_window_policy::{OutputLayout, SnapTarget, WindowPolicy, WindowState, WorkspaceId};

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

    let started = Instant::now();
    emit(
        "session.launch",
        &config,
        &[
            ("verify", FieldValue::Bool(config.verify)),
            ("verify_services", FieldValue::Bool(config.verify_services)),
            (
                "verify_clean_exit",
                FieldValue::Bool(config.verify_clean_exit),
            ),
            (
                "verify_systemd_units",
                FieldValue::Bool(config.verify_systemd_units),
            ),
            ("preflight_only", FieldValue::Bool(config.preflight_only)),
        ],
    );

    let preflight_environment = BackendPreflightEnvironment::from_host();
    let preflight_report =
        preflight_backend_with_environment(config.backend, &preflight_environment);
    emit_backend_preflight(&config, &preflight_report, &preflight_environment);
    emit_launch_ready(&config, preflight_report.ready);

    if !preflight_report.ready {
        return Err(format!(
            "{} session launch preflight failed: {}",
            preflight_report.backend.as_str(),
            preflight_report.code,
        ));
    }

    if config.verify_systemd_units {
        let unit_report = verify_systemd_units(Path::new(&config.systemd_unit_dir));
        emit_systemd_unit_verification(&config, &unit_report);

        if !unit_report.passed() {
            return Err(String::from("session systemd unit verification failed"));
        }
    }

    if config.preflight_only {
        emit(
            "session.exit",
            &config,
            &[(
                "elapsed_ms",
                FieldValue::U64(started.elapsed().as_millis() as u64),
            )],
        );
        return Ok(());
    }

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
            (
                "elapsed_ms",
                FieldValue::U64(started.elapsed().as_millis() as u64),
            ),
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
                    "workspace_switch_ok",
                    FieldValue::Bool(interaction_report.workspace_switch_ok),
                ),
                (
                    "workspace_hidden_windows",
                    FieldValue::U64(interaction_report.workspace_hidden_windows),
                ),
                (
                    "workspace_restored_focus",
                    FieldValue::Bool(interaction_report.workspace_restored_focus),
                ),
                (
                    "snap_left_ok",
                    FieldValue::Bool(interaction_report.snap_left_ok),
                ),
                (
                    "snap_right_ok",
                    FieldValue::Bool(interaction_report.snap_right_ok),
                ),
                (
                    "close_fallback_focus_ok",
                    FieldValue::Bool(interaction_report.close_fallback_focus_ok),
                ),
                (
                    "keyboard_input_ok",
                    FieldValue::Bool(interaction_report.keyboard_input_ok),
                ),
                (
                    "pointer_input_ok",
                    FieldValue::Bool(interaction_report.pointer_input_ok),
                ),
                (
                    "input_windows_after_terminal_launch",
                    FieldValue::U64(interaction_report.input_windows_after_terminal_launch),
                ),
                (
                    "input_final_width",
                    FieldValue::U64(interaction_report.input_final_width),
                ),
                (
                    "input_final_height",
                    FieldValue::U64(interaction_report.input_final_height),
                ),
                (
                    "surface_lifecycle_ok",
                    FieldValue::Bool(interaction_report.surface_lifecycle_ok),
                ),
                (
                    "surface_windows_after_close",
                    FieldValue::U64(interaction_report.surface_windows_after_close),
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

    if config.verify_launch_spawn {
        let launch_spawn_report = verify_launch_spawn(&config);
        emit_launch_spawn(&config, &launch_spawn_report);

        if !launch_spawn_report.passed() {
            return Err(String::from("session launch spawn verification failed"));
        }
    }

    if config.verify_services {
        let service_report = verify_session_services(&config)?;
        emit_service_verification(
            &config,
            &service_report,
            started.elapsed().as_millis() as u64,
        );

        if !service_report.passed() {
            return Err(String::from("session service verification failed"));
        }
    }

    if config.verify_clean_exit {
        let clean_exit_report = verify_session_clean_exit(&mut policy);
        emit_clean_exit(&config, &clean_exit_report);

        if !clean_exit_report.passed() {
            return Err(String::from("session clean exit verification failed"));
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
    elapsed_ms: u64,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl ServiceProbe {
    fn missing() -> Self {
        Self {
            resolved: false,
            exit_ok: false,
            ready: false,
            elapsed_ms: 0,
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
    notification: ServiceProbe,
    settings: ServiceProbe,
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
            && self.notification.resolved
            && self.notification.exit_ok
            && self.notification.ready
            && self.settings.resolved
            && self.settings.exit_ok
            && self.settings.ready
    }

    fn children_exited_cleanly(&self) -> bool {
        self.compositor.exit_ok
            && self.shell.exit_ok
            && self.notification.exit_ok
            && self.settings.exit_ok
    }
}

fn verify_session_services(config: &Config) -> Result<ServiceVerification, String> {
    let compositor_path = sibling_binary("backlit-compositor");
    let shell_path = sibling_binary("backlit-shell");
    let notification_path = sibling_binary("backlit-notification-daemon");
    let settings_path = sibling_binary("backlit-settings-daemon");

    let compositor = run_compositor_probe(&compositor_path, config)?;
    let shell = run_shell_probe(&shell_path, config)?;
    let notification = run_notification_probe(&notification_path)?;
    let settings = run_settings_probe(&settings_path)?;
    let mut report = ServiceVerification {
        compositor,
        shell,
        notification,
        settings,
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
            String::from("\"required_components\":5"),
            String::from("\"lock_screen_ready\":true"),
        ],
    )
}

fn run_notification_probe(path: &Path) -> Result<ServiceProbe, String> {
    run_service_probe(
        path,
        &["--verify"],
        &[
            String::from("\"event\":\"notification_daemon.smoke\""),
            String::from("\"passed\":true"),
            String::from("\"notify_calls\":3"),
            String::from("\"replacement_preserved_id\":true"),
            String::from("\"critical_persistent\":true"),
        ],
    )
}

fn run_settings_probe(path: &Path) -> Result<ServiceProbe, String> {
    run_service_probe(
        path,
        &["--verify"],
        &[
            String::from("\"event\":\"settings_daemon.verified\""),
            String::from("\"passed\":true"),
            String::from("\"display_validated\":true"),
            String::from("\"input_validated\":true"),
            String::from("\"power_validated\":true"),
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

    let started = Instant::now();
    let output = Command::new(path)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run {}: {error}", path.display()))?;
    let elapsed_ms = started.elapsed().as_millis() as u64;

    let mut probe = ServiceProbe {
        resolved: true,
        exit_ok: output.status.success(),
        ready: false,
        elapsed_ms,
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
    fs::write(
        log_dir.join("notification-daemon.jsonl"),
        &report.notification.stdout,
    )
    .map_err(|error| format!("failed to write notification daemon service log: {error}"))?;
    fs::write(
        log_dir.join("notification-daemon.stderr"),
        &report.notification.stderr,
    )
    .map_err(|error| format!("failed to write notification daemon service stderr: {error}"))?;
    fs::write(
        log_dir.join("settings-daemon.jsonl"),
        &report.settings.stdout,
    )
    .map_err(|error| format!("failed to write settings daemon service log: {error}"))?;
    fs::write(
        log_dir.join("settings-daemon.stderr"),
        &report.settings.stderr,
    )
    .map_err(|error| format!("failed to write settings daemon service stderr: {error}"))?;
    Ok(())
}

fn emit_service_verification(config: &Config, report: &ServiceVerification, elapsed_ms: u64) {
    emit(
        "session.services_verified",
        config,
        &[
            ("passed", FieldValue::Bool(report.passed())),
            ("elapsed_ms", FieldValue::U64(elapsed_ms)),
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
                "notification_resolved",
                FieldValue::Bool(report.notification.resolved),
            ),
            (
                "notification_ready",
                FieldValue::Bool(report.notification.ready),
            ),
            (
                "settings_resolved",
                FieldValue::Bool(report.settings.resolved),
            ),
            ("settings_ready", FieldValue::Bool(report.settings.ready)),
            (
                "children_exited_cleanly",
                FieldValue::Bool(report.children_exited_cleanly()),
            ),
            ("logs_written", FieldValue::Bool(report.logs_written)),
            (
                "compositor_probe_ms",
                FieldValue::U64(report.compositor.elapsed_ms),
            ),
            ("shell_probe_ms", FieldValue::U64(report.shell.elapsed_ms)),
            (
                "notification_probe_ms",
                FieldValue::U64(report.notification.elapsed_ms),
            ),
            (
                "settings_probe_ms",
                FieldValue::U64(report.settings.elapsed_ms),
            ),
            (
                "compositor_stdout_bytes",
                FieldValue::U64(report.compositor.stdout.len() as u64),
            ),
            (
                "shell_stdout_bytes",
                FieldValue::U64(report.shell.stdout.len() as u64),
            ),
            (
                "notification_stdout_bytes",
                FieldValue::U64(report.notification.stdout.len() as u64),
            ),
            (
                "settings_stdout_bytes",
                FieldValue::U64(report.settings.stdout.len() as u64),
            ),
        ],
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SystemdUnitContract {
    unit_name: &'static str,
    exec_start: &'static str,
    after: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SystemdUnitProbe {
    present: bool,
    exec_start_ok: bool,
    after_ok: bool,
    part_of_graphical_session: bool,
    wanted_by_graphical_session: bool,
    rust_backtrace_enabled: bool,
    journal_stdout: bool,
    journal_stderr: bool,
    restart_on_failure: bool,
}

impl SystemdUnitProbe {
    fn missing() -> Self {
        Self {
            present: false,
            exec_start_ok: false,
            after_ok: false,
            part_of_graphical_session: false,
            wanted_by_graphical_session: false,
            rust_backtrace_enabled: false,
            journal_stdout: false,
            journal_stderr: false,
            restart_on_failure: false,
        }
    }

    fn passed(self) -> bool {
        self.present
            && self.exec_start_ok
            && self.after_ok
            && self.part_of_graphical_session
            && self.wanted_by_graphical_session
            && self.rust_backtrace_enabled
            && self.journal_stdout
            && self.journal_stderr
            && self.restart_on_failure
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SystemdUnitVerification {
    unit_dir: PathBuf,
    compositor: SystemdUnitProbe,
    shell: SystemdUnitProbe,
    notification: SystemdUnitProbe,
    settings: SystemdUnitProbe,
}

impl SystemdUnitVerification {
    fn passed(&self) -> bool {
        self.compositor.passed()
            && self.shell.passed()
            && self.notification.passed()
            && self.settings.passed()
    }

    fn units_present(&self) -> bool {
        self.compositor.present
            && self.shell.present
            && self.notification.present
            && self.settings.present
    }

    fn exec_starts_ok(&self) -> bool {
        self.compositor.exec_start_ok
            && self.shell.exec_start_ok
            && self.notification.exec_start_ok
            && self.settings.exec_start_ok
    }

    fn startup_order_ok(&self) -> bool {
        self.compositor.after_ok
            && self.shell.after_ok
            && self.notification.after_ok
            && self.settings.after_ok
    }

    fn graphical_session_target_ok(&self) -> bool {
        self.compositor.part_of_graphical_session
            && self.shell.part_of_graphical_session
            && self.notification.part_of_graphical_session
            && self.settings.part_of_graphical_session
            && self.compositor.wanted_by_graphical_session
            && self.shell.wanted_by_graphical_session
            && self.notification.wanted_by_graphical_session
            && self.settings.wanted_by_graphical_session
    }

    fn journal_output_ok(&self) -> bool {
        self.compositor.journal_stdout
            && self.compositor.journal_stderr
            && self.shell.journal_stdout
            && self.shell.journal_stderr
            && self.notification.journal_stdout
            && self.notification.journal_stderr
            && self.settings.journal_stdout
            && self.settings.journal_stderr
    }

    fn rust_backtrace_enabled(&self) -> bool {
        self.compositor.rust_backtrace_enabled
            && self.shell.rust_backtrace_enabled
            && self.notification.rust_backtrace_enabled
            && self.settings.rust_backtrace_enabled
    }

    fn restart_policy_ok(&self) -> bool {
        self.compositor.restart_on_failure
            && self.shell.restart_on_failure
            && self.notification.restart_on_failure
            && self.settings.restart_on_failure
    }
}

fn verify_systemd_units(unit_dir: &Path) -> SystemdUnitVerification {
    let [compositor_contract, shell_contract, notification_contract, settings_contract] =
        systemd_unit_contracts();

    SystemdUnitVerification {
        unit_dir: unit_dir.to_path_buf(),
        compositor: verify_systemd_unit(unit_dir, compositor_contract),
        shell: verify_systemd_unit(unit_dir, shell_contract),
        notification: verify_systemd_unit(unit_dir, notification_contract),
        settings: verify_systemd_unit(unit_dir, settings_contract),
    }
}

fn systemd_unit_contracts() -> [SystemdUnitContract; 4] {
    [
        SystemdUnitContract {
            unit_name: "backlit-compositor.service",
            exec_start: "ExecStart=/usr/bin/backlit-compositor --backend=drm --socket=backlit-0",
            after: "After=graphical-session-pre.target",
        },
        SystemdUnitContract {
            unit_name: "backlit-shell.service",
            exec_start: "ExecStart=/usr/bin/backlit-shell --component=all --socket=backlit-0",
            after: "After=backlit-compositor.service",
        },
        SystemdUnitContract {
            unit_name: "backlit-notification-daemon.service",
            exec_start: "ExecStart=/usr/bin/backlit-notification-daemon",
            after: "After=backlit-compositor.service",
        },
        SystemdUnitContract {
            unit_name: "backlit-settings-daemon.service",
            exec_start: "ExecStart=/usr/bin/backlit-settings-daemon",
            after: "After=backlit-compositor.service",
        },
    ]
}

fn verify_systemd_unit(unit_dir: &Path, contract: SystemdUnitContract) -> SystemdUnitProbe {
    let path = unit_dir.join(contract.unit_name);
    let Ok(contents) = fs::read_to_string(path) else {
        return SystemdUnitProbe::missing();
    };
    let lines: Vec<&str> = contents.lines().map(str::trim).collect();
    let contains_line = |required: &str| lines.contains(&required);

    SystemdUnitProbe {
        present: true,
        exec_start_ok: contains_line(contract.exec_start),
        after_ok: contains_line(contract.after),
        part_of_graphical_session: contains_line("PartOf=graphical-session.target"),
        wanted_by_graphical_session: contains_line("WantedBy=graphical-session.target"),
        rust_backtrace_enabled: contains_line("Environment=RUST_BACKTRACE=1"),
        journal_stdout: contains_line("StandardOutput=journal"),
        journal_stderr: contains_line("StandardError=journal"),
        restart_on_failure: contains_line("Restart=on-failure"),
    }
}

fn emit_systemd_unit_verification(config: &Config, report: &SystemdUnitVerification) {
    let unit_dir = report.unit_dir.to_string_lossy();

    emit(
        "session.systemd_units_verified",
        config,
        &[
            ("passed", FieldValue::Bool(report.passed())),
            ("unit_dir", FieldValue::Str(unit_dir.as_ref())),
            (
                "compositor_unit",
                FieldValue::Str("backlit-compositor.service"),
            ),
            ("shell_unit", FieldValue::Str("backlit-shell.service")),
            (
                "notification_unit",
                FieldValue::Str("backlit-notification-daemon.service"),
            ),
            (
                "settings_unit",
                FieldValue::Str("backlit-settings-daemon.service"),
            ),
            ("units_present", FieldValue::Bool(report.units_present())),
            ("exec_starts", FieldValue::Bool(report.exec_starts_ok())),
            ("startup_order", FieldValue::Bool(report.startup_order_ok())),
            (
                "graphical_session_target",
                FieldValue::Bool(report.graphical_session_target_ok()),
            ),
            (
                "journal_output",
                FieldValue::Bool(report.journal_output_ok()),
            ),
            (
                "rust_backtrace_enabled",
                FieldValue::Bool(report.rust_backtrace_enabled()),
            ),
            (
                "restart_policy",
                FieldValue::Bool(report.restart_policy_ok()),
            ),
        ],
    );
}

fn emit_backend_preflight(
    config: &Config,
    report: &BackendPreflightReport,
    environment: &BackendPreflightEnvironment,
) {
    let wayland_display = environment.wayland_display.as_deref().unwrap_or("");
    let xdg_runtime_dir = environment.xdg_runtime_dir.as_deref().unwrap_or("");
    let session_id = environment.session_id.as_deref().unwrap_or("");
    let seat = environment.seat.as_deref().unwrap_or("");
    let session_type = environment.session_type.as_deref().unwrap_or("");
    let session_state = environment.session_state.as_deref().unwrap_or("");

    emit(
        "session.backend_preflight",
        config,
        &[
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
                FieldValue::U64(environment.drm_card_nodes),
            ),
            (
                "drm_render_nodes",
                FieldValue::U64(environment.drm_render_nodes),
            ),
            (
                "input_event_nodes",
                FieldValue::U64(environment.input_event_nodes),
            ),
            (
                "drm_card_readable",
                FieldValue::U64(environment.drm_card_readable),
            ),
            (
                "drm_card_writable",
                FieldValue::U64(environment.drm_card_writable),
            ),
            (
                "drm_render_readable",
                FieldValue::U64(environment.drm_render_readable),
            ),
            (
                "drm_render_writable",
                FieldValue::U64(environment.drm_render_writable),
            ),
            (
                "input_event_readable",
                FieldValue::U64(environment.input_event_readable),
            ),
            (
                "drm_card_access_ready",
                FieldValue::Bool(environment.drm_card_access_ready()),
            ),
            (
                "input_requires_logind_broker",
                FieldValue::Bool(environment.input_requires_logind_broker()),
            ),
            (
                "logind_available",
                FieldValue::Bool(environment.logind_available),
            ),
            (
                "libseat_available",
                FieldValue::Bool(environment.libseat_available),
            ),
            (
                "libinput_available",
                FieldValue::Bool(environment.libinput_available),
            ),
            (
                "input_broker_ready",
                FieldValue::Bool(environment.input_broker_ready()),
            ),
            (
                "input_broker_mode",
                FieldValue::Str(environment.input_broker_mode()),
            ),
            ("session_id", FieldValue::Str(session_id)),
            ("seat", FieldValue::Str(seat)),
            ("session_type", FieldValue::Str(session_type)),
            ("session_state", FieldValue::Str(session_state)),
            (
                "logind_session_verified",
                FieldValue::Bool(environment.logind_session_verified),
            ),
            (
                "session_active",
                FieldValue::Bool(environment.session_active),
            ),
            (
                "session_remote",
                FieldValue::Bool(environment.session_remote),
            ),
        ],
    );
}

fn emit_launch_ready(config: &Config, passed: bool) {
    emit(
        "session.launch_ready",
        config,
        &[
            ("passed", FieldValue::Bool(passed)),
            ("preflight_only", FieldValue::Bool(config.preflight_only)),
        ],
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaunchSpawnReport {
    shortcut_resolved: bool,
    target_resolved: bool,
    program: String,
    spawned: bool,
    exit_success: bool,
    status_code: u64,
    wayland_display_set: bool,
    elapsed_ms: u64,
}

impl LaunchSpawnReport {
    fn passed(&self) -> bool {
        self.shortcut_resolved
            && self.target_resolved
            && self.spawned
            && self.exit_success
            && self.wayland_display_set
    }
}

fn verify_launch_spawn(config: &Config) -> LaunchSpawnReport {
    let started = Instant::now();
    let shortcut_resolved = matches!(
        resolve_shortcut("Super+Enter"),
        Some(ShortcutAction::Launch(LaunchTarget::Terminal))
    );
    let catalog = default_catalog();
    let command = resolve_command(&catalog, LaunchTarget::Terminal);
    let target_resolved = command.is_some();
    let program = config
        .launch_spawn_program
        .clone()
        .or_else(|| command.map(|command| command.program.to_string()))
        .unwrap_or_default();
    let args: Vec<String> = if config.launch_spawn_program.is_some() {
        config.launch_spawn_args.clone()
    } else {
        command
            .map(|command| command.args.iter().map(|arg| (*arg).to_string()).collect())
            .unwrap_or_default()
    };
    let wayland_display = config
        .launch_wayland_display
        .clone()
        .or_else(|| env::var("WAYLAND_DISPLAY").ok());

    if !shortcut_resolved || !target_resolved || program.trim().is_empty() {
        return LaunchSpawnReport {
            shortcut_resolved,
            target_resolved,
            program,
            spawned: false,
            exit_success: false,
            status_code: 255,
            wayland_display_set: wayland_display.is_some(),
            elapsed_ms: started.elapsed().as_millis() as u64,
        };
    }

    let mut child = Command::new(&program);
    child.args(args);
    if let Some(display) = &wayland_display {
        child.env("WAYLAND_DISPLAY", display);
    }

    match child.status() {
        Ok(status) => LaunchSpawnReport {
            shortcut_resolved,
            target_resolved,
            program,
            spawned: true,
            exit_success: status.success(),
            status_code: status.code().unwrap_or(255) as u64,
            wayland_display_set: wayland_display.is_some(),
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
        Err(_) => LaunchSpawnReport {
            shortcut_resolved,
            target_resolved,
            program,
            spawned: false,
            exit_success: false,
            status_code: 255,
            wayland_display_set: wayland_display.is_some(),
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    }
}

fn emit_launch_spawn(config: &Config, report: &LaunchSpawnReport) {
    emit(
        "session.launch_spawn",
        config,
        &[
            ("target", FieldValue::Str(LaunchTarget::Terminal.as_str())),
            (
                "shortcut",
                FieldValue::Str(LaunchTarget::Terminal.shortcut()),
            ),
            (
                "shortcut_resolved",
                FieldValue::Bool(report.shortcut_resolved),
            ),
            ("target_resolved", FieldValue::Bool(report.target_resolved)),
            ("program", FieldValue::Str(report.program.as_str())),
            ("spawned", FieldValue::Bool(report.spawned)),
            ("exit_success", FieldValue::Bool(report.exit_success)),
            ("status_code", FieldValue::U64(report.status_code)),
            (
                "wayland_display_set",
                FieldValue::Bool(report.wayland_display_set),
            ),
            ("elapsed_ms", FieldValue::U64(report.elapsed_ms)),
        ],
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CleanExitReport {
    requested: bool,
    windows_before_shutdown: u64,
    windows_closed: u64,
    windows_after_shutdown: u64,
    focus_cleared: bool,
}

impl CleanExitReport {
    fn passed(self) -> bool {
        self.requested
            && self.windows_before_shutdown > 0
            && self.windows_closed == self.windows_before_shutdown
            && self.windows_after_shutdown == 0
            && self.focus_cleared
    }
}

fn verify_session_clean_exit(policy: &mut WindowPolicy) -> CleanExitReport {
    let windows_before_shutdown = policy.windows().len() as u64;
    let windows_closed = policy.close_all_windows() as u64;
    let windows_after_shutdown = policy.windows().len() as u64;

    CleanExitReport {
        requested: true,
        windows_before_shutdown,
        windows_closed,
        windows_after_shutdown,
        focus_cleared: policy.focused().is_none(),
    }
}

fn emit_clean_exit(config: &Config, report: &CleanExitReport) {
    emit(
        "session.clean_exit",
        config,
        &[
            ("passed", FieldValue::Bool(report.passed())),
            ("requested", FieldValue::Bool(report.requested)),
            (
                "windows_before_shutdown",
                FieldValue::U64(report.windows_before_shutdown),
            ),
            ("windows_closed", FieldValue::U64(report.windows_closed)),
            (
                "windows_after_shutdown",
                FieldValue::U64(report.windows_after_shutdown),
            ),
            ("focus_cleared", FieldValue::Bool(report.focus_cleared)),
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
    workspace_switch_ok: bool,
    workspace_hidden_windows: u64,
    workspace_restored_focus: bool,
    snap_left_ok: bool,
    snap_right_ok: bool,
    close_fallback_focus_ok: bool,
    windows_after_close: u64,
    keyboard_input_ok: bool,
    pointer_input_ok: bool,
    input_windows_after_terminal_launch: u64,
    input_final_width: u64,
    input_final_height: u64,
    surface_lifecycle_ok: bool,
    surface_windows_after_close: u64,
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
            && self.workspace_switch_ok
            && self.workspace_hidden_windows == 1
            && self.workspace_restored_focus
            && self.snap_left_ok
            && self.snap_right_ok
            && self.close_fallback_focus_ok
            && self.windows_after_close == 3
            && self.keyboard_input_ok
            && self.pointer_input_ok
            && self.input_windows_after_terminal_launch == 4
            && self.surface_lifecycle_ok
            && self.surface_windows_after_close == 0
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

    let workspace_window = focused;
    let (workspace_switch_ok, workspace_hidden_windows, workspace_restored_focus) =
        workspace_window
            .map(|id| {
                let moved = policy.move_window_to_workspace(id, WorkspaceId(2));
                let hidden_windows = policy.windows_on_workspace(WorkspaceId(2)) as u64;
                let switched = policy.switch_workspace(WorkspaceId(2));
                let focused_on_workspace = policy.focused() == Some(id);
                let switched_back = policy.switch_workspace(WorkspaceId(1));
                let restored_focus = switched_back
                    && policy.focused().is_some()
                    && policy
                        .focused()
                        .and_then(|focused| policy.window(focused))
                        .map(|window| {
                            window.workspace == WorkspaceId(1)
                                && window.state != WindowState::Minimized
                        })
                        .unwrap_or(false);

                (
                    moved && switched && focused_on_workspace,
                    hidden_windows,
                    restored_focus,
                )
            })
            .unwrap_or((false, 0, false));

    let snap_window = policy.focused();
    let left_half = backlit_window_policy::Rect::new(
        layout.work_area().x,
        layout.work_area().y,
        layout.work_area().width / 2,
        layout.work_area().height,
    );
    let right_half = backlit_window_policy::Rect::new(
        layout.work_area().x + layout.work_area().width / 2,
        layout.work_area().y,
        layout.work_area().width - layout.work_area().width / 2,
        layout.work_area().height,
    );
    let (snap_left_ok, snap_right_ok) = snap_window
        .map(|id| {
            let left = policy.snap_window(id, layout.work_area(), SnapTarget::LeftHalf)
                && policy.window(id).map(|window| window.geometry) == Some(left_half)
                && policy.window(id).map(|window| window.state) == Some(WindowState::Snapped);
            let right = policy.snap_window(id, layout.work_area(), SnapTarget::RightHalf)
                && policy.window(id).map(|window| window.geometry) == Some(right_half)
                && policy.window(id).map(|window| window.state) == Some(WindowState::Snapped);

            (left, right)
        })
        .unwrap_or((false, false));

    let close_fallback_focus_ok = policy.close_focused_window().is_some()
        && policy.focused().is_some()
        && policy
            .focused()
            .and_then(|id| policy.window(id))
            .map(|window| window.state != WindowState::Minimized)
            .unwrap_or(false);
    let windows_after_close = policy.windows().len() as u64;
    let input_report = run_input_smoke();
    let keyboard_input_ok = input_report.terminal_launch_resolved
        && input_report.app_switcher_changed_focus
        && input_report.windows_after_terminal_launch == 4;
    let pointer_input_ok = input_report.pointer_focus_window
        && input_report.pointer_move_window
        && input_report.pointer_resize_window
        && input_report.pointer_grab_ended;
    let surface_report = run_surface_lifecycle_smoke();

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
        workspace_switch_ok,
        workspace_hidden_windows,
        workspace_restored_focus,
        snap_left_ok,
        snap_right_ok,
        close_fallback_focus_ok,
        windows_after_close,
        keyboard_input_ok,
        pointer_input_ok,
        input_windows_after_terminal_launch: input_report.windows_after_terminal_launch,
        input_final_width: input_report.final_width,
        input_final_height: input_report.final_height,
        surface_lifecycle_ok: surface_report.passed(),
        surface_windows_after_close: surface_report.windows_after_close,
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
    verify_clean_exit: bool,
    verify_systemd_units: bool,
    systemd_unit_dir: String,
    preflight_only: bool,
    verify_launch_spawn: bool,
    launch_spawn_program: Option<String>,
    launch_spawn_args: Vec<String>,
    launch_wayland_display: Option<String>,
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
            verify_clean_exit: false,
            verify_systemd_units: false,
            systemd_unit_dir: String::from("/usr/lib/systemd/user"),
            preflight_only: false,
            verify_launch_spawn: false,
            launch_spawn_program: None,
            launch_spawn_args: Vec::new(),
            launch_wayland_display: None,
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
            } else if arg == "--verify-clean-exit" {
                config.verify_clean_exit = true;
            } else if arg == "--verify-systemd-units" {
                config.verify_systemd_units = true;
            } else if let Some(value) = arg.strip_prefix("--systemd-unit-dir=") {
                config.systemd_unit_dir = value.to_string();
            } else if arg == "--systemd-unit-dir" {
                config.systemd_unit_dir = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --systemd-unit-dir"))?;
            } else if arg == "--preflight-only" {
                config.preflight_only = true;
            } else if arg == "--verify-launch-spawn" {
                config.verify_launch_spawn = true;
            } else if let Some(value) = arg.strip_prefix("--launch-spawn-program=") {
                config.launch_spawn_program = Some(value.to_string());
            } else if arg == "--launch-spawn-program" {
                config.launch_spawn_program = Some(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --launch-spawn-program"))?,
                );
            } else if let Some(value) = arg.strip_prefix("--launch-spawn-arg=") {
                config.launch_spawn_args.push(value.to_string());
            } else if arg == "--launch-spawn-arg" {
                config.launch_spawn_args.push(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --launch-spawn-arg"))?,
                );
            } else if let Some(value) = arg.strip_prefix("--wayland-display=") {
                config.launch_wayland_display = Some(value.to_string());
            } else if arg == "--wayland-display" {
                config.launch_wayland_display = Some(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --wayland-display"))?,
                );
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
  backlit-session [--backend=headless|wayland|drm] [--socket=backlit-0] [--screenshot=target/backlit-session.ppm] [--verify] [--verify-services] [--verify-systemd-units] [--verify-launch-spawn] [--verify-clean-exit] [--preflight-only]

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
                 Fail if sibling compositor, shell, and settings probes cannot launch.
  --verify-launch-spawn
                 Spawn the terminal launch target resolved from Super+Enter.
  --verify-clean-exit
                 Verify session shutdown closes managed windows and clears focus.
  --verify-systemd-units
                 Verify installed user systemd units for the graphical session.
  --systemd-unit-dir
                 Directory containing Backlit user systemd units. Defaults to /usr/lib/systemd/user.
  --launch-spawn-program
                 Override terminal program for deterministic spawn verification.
  --launch-spawn-arg
                 Argument for the launch spawn program override. May repeat.
  --wayland-display
                 WAYLAND_DISPLAY value to pass to the launched terminal target.
  --preflight-only
                 Verify backend launch prerequisites and exit before rendering.
"
    );
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{binary_name, verify_systemd_units, Config};

    #[test]
    fn parses_service_verification_flags() {
        let config = Config::parse([
            "--verify",
            "--verify-services",
            "--verify-clean-exit",
            "--verify-systemd-units",
            "--systemd-unit-dir",
            "packaging/systemd",
            "--preflight-only",
            "--verify-launch-spawn",
            "--launch-spawn-program",
            "true",
            "--launch-spawn-arg",
            "--help",
            "--wayland-display",
            "wayland-1",
            "--service-log-dir",
            "target/session-services",
        ])
        .expect("config should parse");

        assert!(config.verify);
        assert!(config.verify_services);
        assert!(config.verify_clean_exit);
        assert!(config.verify_systemd_units);
        assert_eq!(config.systemd_unit_dir, "packaging/systemd");
        assert!(config.preflight_only);
        assert!(config.verify_launch_spawn);
        assert_eq!(config.launch_spawn_program.as_deref(), Some("true"));
        assert_eq!(config.launch_spawn_args, ["--help"]);
        assert_eq!(config.launch_wayland_display.as_deref(), Some("wayland-1"));
        assert_eq!(
            config.service_log_dir.as_deref(),
            Some("target/session-services")
        );
    }

    #[test]
    fn binary_name_uses_platform_suffix() {
        assert!(binary_name("backlit-compositor").starts_with("backlit-compositor"));
    }

    #[test]
    fn verifies_systemd_unit_contracts() {
        let unit_dir = unique_test_dir("systemd-units-ok");
        fs::create_dir_all(&unit_dir).expect("unit dir should be created");
        write_unit(
            &unit_dir,
            "backlit-compositor.service",
            "Backlit Wayland compositor",
            "After=graphical-session-pre.target",
            "ExecStart=/usr/bin/backlit-compositor --backend=drm --socket=backlit-0",
        );
        write_unit(
            &unit_dir,
            "backlit-shell.service",
            "Backlit shell",
            "After=backlit-compositor.service",
            "ExecStart=/usr/bin/backlit-shell --component=all --socket=backlit-0",
        );
        write_unit(
            &unit_dir,
            "backlit-notification-daemon.service",
            "Backlit notification daemon",
            "After=backlit-compositor.service",
            "ExecStart=/usr/bin/backlit-notification-daemon",
        );
        write_unit(
            &unit_dir,
            "backlit-settings-daemon.service",
            "Backlit settings daemon",
            "After=backlit-compositor.service",
            "ExecStart=/usr/bin/backlit-settings-daemon",
        );

        let report = verify_systemd_units(&unit_dir);

        assert!(report.passed(), "{report:?}");
        assert!(report.units_present());
        assert!(report.exec_starts_ok());
        assert!(report.startup_order_ok());
        assert!(report.graphical_session_target_ok());
        assert!(report.journal_output_ok());
        assert!(report.rust_backtrace_enabled());
        assert!(report.restart_policy_ok());
    }

    #[test]
    fn rejects_incomplete_systemd_unit_contracts() {
        let unit_dir = unique_test_dir("systemd-units-missing");
        fs::create_dir_all(&unit_dir).expect("unit dir should be created");
        write_unit(
            &unit_dir,
            "backlit-compositor.service",
            "Backlit Wayland compositor",
            "After=graphical-session-pre.target",
            "ExecStart=/usr/bin/backlit-compositor --backend=drm --socket=backlit-0",
        );

        let report = verify_systemd_units(&unit_dir);

        assert!(!report.passed());
        assert!(!report.units_present());
        assert!(report.compositor.passed());
        assert!(!report.shell.present);
    }

    fn unique_test_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("backlit-{name}-{}-{nanos}", std::process::id()))
    }

    fn write_unit(
        unit_dir: &Path,
        unit_name: &str,
        description: &str,
        after: &str,
        exec_start: &str,
    ) {
        let contents = format!(
            "\
[Unit]
Description={description}
{after}
PartOf=graphical-session.target

[Service]
Type=simple
{exec_start}
Environment=RUST_BACKTRACE=1
SyslogIdentifier={unit_name}
StandardOutput=journal
StandardError=journal
Restart=on-failure

[Install]
WantedBy=graphical-session.target
"
        );
        fs::write(unit_dir.join(unit_name), contents).expect("unit should be written");
    }
}
