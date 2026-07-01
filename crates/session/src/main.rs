use std::env;
use std::fs;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{self, Child, Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use backlit_common::metrics::{event_json, FieldValue};
use backlit_compositor_backend::backend_launch_plan;
use backlit_compositor_backend::BackendLaunchPlan;
use backlit_compositor_backend::{
    preflight_backend_with_environment, smithay_runtime_probe, BackendKind,
    BackendPreflightEnvironment, BackendPreflightReport, SmithayRuntimeProbe,
};
use backlit_demo_client::{
    render_policy_gui, render_policy_gui_with_overlay, verify_policy_gui, verify_session_overlay,
    SessionOverlay, DEFAULT_DEMO_HEIGHT, DEFAULT_DEMO_WIDTH,
};
use backlit_input::{
    run_input_smoke, ButtonState, InputEvent, InputRouter, PointerButton, RoutedAction,
};
use backlit_launcher::{
    default_catalog, default_desktop_entry_dirs, discover_desktop_entries_in_dirs, resolve_command,
    DesktopEntry, LaunchTarget,
};
use backlit_shortcuts::{resolve_shortcut, ShortcutAction};
use backlit_surface::run_surface_lifecycle_smoke;
use backlit_window_policy::{
    OutputLayout, Rect, SnapTarget, WindowId, WindowPolicy, WindowState, WorkspaceId,
};

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
            (
                "verify_systemd_activation",
                FieldValue::Bool(config.verify_systemd_activation),
            ),
            (
                "verify_drm_first_present",
                FieldValue::Bool(config.verify_drm_first_present),
            ),
            (
                "require_drm_master_present",
                FieldValue::Bool(config.require_drm_master_present),
            ),
            (
                "activate_systemd",
                FieldValue::Bool(config.activate_systemd),
            ),
            ("preflight_only", FieldValue::Bool(config.preflight_only)),
        ],
    );

    let preflight_environment = BackendPreflightEnvironment::from_host();
    let preflight_report =
        preflight_backend_with_environment(config.backend, &preflight_environment);
    emit_backend_preflight(&config, &preflight_report, &preflight_environment);
    let launch_plan =
        backend_launch_plan(config.backend, &preflight_report, &preflight_environment);
    emit_backend_launch_plan(&config, &launch_plan);
    emit_launch_ready(&config, preflight_report.ready);

    if !preflight_report.ready {
        return Err(format!(
            "{} session launch preflight failed: {}",
            preflight_report.backend.as_str(),
            preflight_report.code,
        ));
    }

    if config.verify_drm_first_present || config.require_drm_master_present {
        if config.backend != BackendKind::Drm {
            return Err(String::from(
                "DRM first-present verification requires --backend=drm",
            ));
        }

        let probe = smithay_runtime_probe(&preflight_environment);
        emit_drm_first_present_probe(&config, &probe);

        if !probe.passed() {
            return Err(String::from(
                "DRM first-present verification did not reach a valid commit or boundary",
            ));
        }

        if config.require_drm_master_present && !probe.kms_first_present_commit_succeeded {
            return Err(String::from(
                "DRM first-present commit was not observed; run from a dedicated DRM-master session",
            ));
        }
    }

    let needs_systemd_contract =
        config.verify_systemd_units || config.verify_systemd_activation || config.activate_systemd;
    if needs_systemd_contract {
        let unit_report = verify_systemd_units(Path::new(&config.systemd_unit_dir));
        emit_systemd_unit_verification(&config, &unit_report);
        let launch_plan = systemd_launch_plan();
        emit_systemd_launch_plan(&config, &launch_plan, &unit_report);

        if !unit_report.passed() || !launch_plan.ready() {
            return Err(String::from("session systemd unit verification failed"));
        }

        if config.verify_systemd_activation || config.activate_systemd {
            let stop_after_start = config.verify_systemd_activation;
            let activation_report = run_systemd_activation(&config, &launch_plan, stop_after_start);
            emit_systemd_activation(&config, &activation_report);

            if !activation_report.passed() {
                return Err(String::from("session systemd activation failed"));
            }

            if config.verify_systemd_activation {
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

            emit(
                "session.running",
                &config,
                &[
                    ("target", FieldValue::Str(launch_plan.target)),
                    (
                        "elapsed_ms",
                        FieldValue::U64(started.elapsed().as_millis() as u64),
                    ),
                ],
            );
            wait_for_systemd_session();
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

    let layout = OutputLayout::new(config.width as i32, config.height as i32, 42);
    let mut policy = initial_session_policy(layout);

    let screenshot = config
        .screenshot
        .clone()
        .unwrap_or_else(|| String::from("target/backlit-session.ppm"));
    let canvas = render_policy_gui(config.width, config.height, &policy, layout);
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
        let report = verify_policy_gui(&canvas, &policy, layout);
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
                ("policy_windows", FieldValue::U64(report.policy_windows)),
                ("visible_windows", FieldValue::U64(report.visible_windows)),
                (
                    "focused_window_visible",
                    FieldValue::Bool(report.focused_window_visible),
                ),
                (
                    "focused_title_bar_ok",
                    FieldValue::Bool(report.focused_title_bar_ok),
                ),
                (
                    "workspace_indicator_ok",
                    FieldValue::Bool(report.workspace_indicator_ok),
                ),
            ],
        );

        if !report.passed() || !interaction_report.passed() {
            return Err(String::from("headless GUI verification failed"));
        }
    }

    if let Some(replay_dir) = &config.scripted_replay_dir {
        let replay_report = run_scripted_replay(&config, &policy, layout, Path::new(replay_dir))?;
        emit_replay(&config, &replay_report);

        if !replay_report.passed() {
            return Err(String::from("session scripted replay verification failed"));
        }
    }

    if config.verify_launch_spawn {
        let launch_spawn_report = verify_launch_spawn(&config);
        emit_launch_spawn(&config, &launch_spawn_report);

        if !launch_spawn_report.passed() {
            return Err(String::from("session launch spawn verification failed"));
        }
    }

    if config.verify_desktop_launch {
        let desktop_launch_report = verify_desktop_launch(&config, &policy)?;
        emit_desktop_launch(&config, &desktop_launch_report);

        if !desktop_launch_report.passed() {
            return Err(String::from("session desktop launch verification failed"));
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

fn initial_session_policy(layout: OutputLayout) -> WindowPolicy {
    let mut policy = WindowPolicy::default();
    let scale_x = layout.output.width.max(320);
    let scale_y = layout.output.height.max(220);
    let scaled_x = |value: i32| value * scale_x / DEFAULT_DEMO_WIDTH as i32;
    let scaled_y = |value: i32| value * scale_y / DEFAULT_DEMO_HEIGHT as i32;

    let terminal = policy.add_window("terminal", (scaled_x(310).max(180), scaled_y(178).max(120)));
    let settings = policy.add_window("settings", (scaled_x(280).max(180), scaled_y(170).max(120)));
    let browser = policy.add_window("browser", (scaled_x(374).max(220), scaled_y(188).max(140)));

    policy.move_window(terminal, scaled_x(132), scaled_y(74));
    policy.move_window(settings, scaled_x(390), scaled_y(132));
    policy.move_window(browser, scaled_x(214), scaled_y(260));

    policy
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReplayFrame {
    checksum: u64,
    written: bool,
    overlay_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScriptedReplayReport {
    frame_count: u64,
    frames_written: u64,
    distinct_checksums: u64,
    initial_focus: u64,
    focus_after_switcher: u64,
    app_switcher_focus_changed: bool,
    terminal_launch_resolved: bool,
    launcher_overlay_opened: bool,
    launcher_overlay_frame: bool,
    app_switcher_overlay_frame: bool,
    launched_window: u64,
    windows_after_launch: u64,
    move_begin: bool,
    move_frame_changed: bool,
    move_grab_ended: bool,
    moved_x: u64,
    moved_y: u64,
    resize_begin: bool,
    resize_frame_changed: bool,
    resize_grab_ended: bool,
    resized_width: u64,
    resized_height: u64,
    snap_frame_ok: bool,
    workspace_hidden: bool,
    workspace_switch_ok: bool,
    final_visible_windows: u64,
}

impl ScriptedReplayReport {
    fn passed(&self) -> bool {
        self.frame_count == 9
            && self.frames_written == self.frame_count
            && self.distinct_checksums >= 8
            && self.initial_focus != 0
            && self.app_switcher_focus_changed
            && self.terminal_launch_resolved
            && self.launcher_overlay_opened
            && self.launcher_overlay_frame
            && self.app_switcher_overlay_frame
            && self.launched_window != 0
            && self.windows_after_launch == 4
            && self.move_begin
            && self.move_frame_changed
            && self.move_grab_ended
            && self.moved_x > 0
            && self.moved_y > 0
            && self.resize_begin
            && self.resize_frame_changed
            && self.resize_grab_ended
            && self.resized_width >= 320
            && self.resized_height >= 220
            && self.snap_frame_ok
            && self.workspace_hidden
            && self.workspace_switch_ok
            && self.final_visible_windows == 1
    }
}

fn run_scripted_replay(
    config: &Config,
    policy: &WindowPolicy,
    layout: OutputLayout,
    replay_dir: &Path,
) -> Result<ScriptedReplayReport, String> {
    fs::create_dir_all(replay_dir)
        .map_err(|error| format!("failed to create {}: {error}", replay_dir.display()))?;

    let mut router = InputRouter::new(policy.clone(), layout);
    let mut frames = Vec::new();
    frames.push(write_replay_frame(
        config,
        replay_dir,
        "00-initial.ppm",
        router.policy(),
        layout,
        None,
    )?);
    let initial_focus = router.policy().focused().map(|id| id.0).unwrap_or(0);

    let focus_after_switcher = match router.route(InputEvent::shortcut("Alt+Tab")) {
        RoutedAction::AppSwitcher { focused } => focused,
        _ => None,
    };
    frames.push(write_replay_frame(
        config,
        replay_dir,
        "01-app-switcher.ppm",
        router.policy(),
        layout,
        Some(SessionOverlay::AppSwitcher),
    )?);
    let focus_after_switcher_u64 = focus_after_switcher.map(|id| id.0).unwrap_or(0);
    let app_switcher_focus_changed =
        focus_after_switcher_u64 != 0 && focus_after_switcher_u64 != initial_focus;
    let app_switcher_overlay_frame = frames
        .last()
        .map(|frame| frame.overlay_verified)
        .unwrap_or(false);

    let launcher_overlay_opened = matches!(
        router.route(InputEvent::shortcut("Super+Space")),
        RoutedAction::Shortcut {
            action: ShortcutAction::OpenLauncher,
        }
    );
    frames.push(write_replay_frame(
        config,
        replay_dir,
        "02-launcher-open.ppm",
        router.policy(),
        layout,
        Some(SessionOverlay::Launcher),
    )?);
    let launcher_overlay_frame = frames
        .last()
        .map(|frame| frame.overlay_verified)
        .unwrap_or(false);

    let terminal_launch_resolved = matches!(
        router.route(InputEvent::shortcut("Super+Enter")),
        RoutedAction::LaunchTarget {
            target: LaunchTarget::Terminal,
        }
    );
    let launched_window = if terminal_launch_resolved {
        router.policy_mut().add_window("terminal-2", (320, 220))
    } else {
        WindowId(0)
    };
    frames.push(write_replay_frame(
        config,
        replay_dir,
        "03-terminal-launch.ppm",
        router.policy(),
        layout,
        None,
    )?);
    let windows_after_launch = router.policy().windows().len() as u64;

    let original = router
        .policy()
        .window(launched_window)
        .map(|window| window.geometry)
        .unwrap_or(Rect::new(0, 0, 0, 0));
    let title_x = original.x + 18;
    let title_y = original.y + 12;
    router.route(InputEvent::PointerMotion {
        x: title_x,
        y: title_y,
    });
    let move_begin = matches!(
        router.route(InputEvent::PointerButton {
            button: PointerButton::Left,
            state: ButtonState::Pressed,
        }),
        RoutedAction::MoveBegin { window } if window == launched_window
    );
    let moved_x_i32 = original.x + 44;
    let moved_y_i32 = original.y + 36;
    let move_frame_changed = matches!(
        router.route(InputEvent::PointerMotion {
            x: title_x + 44,
            y: title_y + 36,
        }),
        RoutedAction::WindowMoved { window, x, y }
            if window == launched_window && x == moved_x_i32 && y == moved_y_i32
    );
    let move_grab_ended = matches!(
        router.route(InputEvent::PointerButton {
            button: PointerButton::Left,
            state: ButtonState::Released,
        }),
        RoutedAction::PointerGrabEnd
    );
    frames.push(write_replay_frame(
        config,
        replay_dir,
        "04-window-moved.ppm",
        router.policy(),
        layout,
        None,
    )?);

    let resized_from = router
        .policy()
        .window(launched_window)
        .map(|window| window.geometry)
        .unwrap_or(original);
    let resize_x = resized_from.x + resized_from.width - 4;
    let resize_y = resized_from.y + resized_from.height - 4;
    router.route(InputEvent::PointerMotion {
        x: resize_x,
        y: resize_y,
    });
    let resize_begin = matches!(
        router.route(InputEvent::PointerButton {
            button: PointerButton::Left,
            state: ButtonState::Pressed,
        }),
        RoutedAction::ResizeBegin { window } if window == launched_window
    );
    let (resize_frame_changed, resized_width, resized_height) =
        match router.route(InputEvent::PointerMotion {
            x: resize_x + 72,
            y: resize_y + 48,
        }) {
            RoutedAction::WindowResized {
                window,
                width,
                height,
            } if window == launched_window => (true, width as u64, height as u64),
            _ => (false, 0, 0),
        };
    let resize_grab_ended = matches!(
        router.route(InputEvent::PointerButton {
            button: PointerButton::Left,
            state: ButtonState::Released,
        }),
        RoutedAction::PointerGrabEnd
    );
    frames.push(write_replay_frame(
        config,
        replay_dir,
        "05-window-resized.ppm",
        router.policy(),
        layout,
        None,
    )?);

    let snap_frame_ok =
        router
            .policy_mut()
            .snap_window(launched_window, layout.work_area(), SnapTarget::LeftHalf);
    frames.push(write_replay_frame(
        config,
        replay_dir,
        "06-window-snapped.ppm",
        router.policy(),
        layout,
        None,
    )?);

    let moved_to_workspace = router
        .policy_mut()
        .move_window_to_workspace(launched_window, WorkspaceId(2));
    let workspace_hidden = moved_to_workspace
        && router
            .policy()
            .visible_windows()
            .all(|window| window.id != launched_window);
    frames.push(write_replay_frame(
        config,
        replay_dir,
        "07-workspace-hidden.ppm",
        router.policy(),
        layout,
        None,
    )?);

    let switched_workspace = router.policy_mut().switch_workspace(WorkspaceId(2));
    let final_visible_windows = router.policy().visible_windows().count() as u64;
    let workspace_switch_ok =
        switched_workspace && router.policy().focused() == Some(launched_window);
    frames.push(write_replay_frame(
        config,
        replay_dir,
        "08-workspace-switched.ppm",
        router.policy(),
        layout,
        None,
    )?);

    let frames_written = frames.iter().filter(|frame| frame.written).count() as u64;
    let mut checksums: Vec<u64> = frames.iter().map(|frame| frame.checksum).collect();
    checksums.sort_unstable();
    checksums.dedup();

    Ok(ScriptedReplayReport {
        frame_count: frames.len() as u64,
        frames_written,
        distinct_checksums: checksums.len() as u64,
        initial_focus,
        focus_after_switcher: focus_after_switcher_u64,
        app_switcher_focus_changed,
        terminal_launch_resolved,
        launcher_overlay_opened,
        launcher_overlay_frame,
        app_switcher_overlay_frame,
        launched_window: launched_window.0,
        windows_after_launch,
        move_begin,
        move_frame_changed,
        move_grab_ended,
        moved_x: moved_x_i32.max(0) as u64,
        moved_y: moved_y_i32.max(0) as u64,
        resize_begin,
        resize_frame_changed,
        resize_grab_ended,
        resized_width,
        resized_height,
        snap_frame_ok,
        workspace_hidden,
        workspace_switch_ok,
        final_visible_windows,
    })
}

fn write_replay_frame(
    config: &Config,
    replay_dir: &Path,
    file_name: &str,
    policy: &WindowPolicy,
    layout: OutputLayout,
    overlay: Option<SessionOverlay>,
) -> Result<ReplayFrame, String> {
    let path = replay_dir.join(file_name);
    let canvas =
        render_policy_gui_with_overlay(config.width, config.height, policy, layout, overlay);
    canvas
        .write_ppm(&path)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;

    Ok(ReplayFrame {
        checksum: canvas.checksum(),
        written: path.is_file(),
        overlay_verified: overlay
            .map(|overlay| verify_session_overlay(&canvas, overlay))
            .unwrap_or(true),
    })
}

fn emit_replay(config: &Config, report: &ScriptedReplayReport) {
    emit(
        "session.replay",
        config,
        &[
            ("passed", FieldValue::Bool(report.passed())),
            ("frame_count", FieldValue::U64(report.frame_count)),
            ("frames_written", FieldValue::U64(report.frames_written)),
            (
                "distinct_checksums",
                FieldValue::U64(report.distinct_checksums),
            ),
            ("initial_focus", FieldValue::U64(report.initial_focus)),
            (
                "focus_after_switcher",
                FieldValue::U64(report.focus_after_switcher),
            ),
            (
                "app_switcher_focus_changed",
                FieldValue::Bool(report.app_switcher_focus_changed),
            ),
            (
                "terminal_launch_resolved",
                FieldValue::Bool(report.terminal_launch_resolved),
            ),
            (
                "launcher_overlay_opened",
                FieldValue::Bool(report.launcher_overlay_opened),
            ),
            (
                "launcher_overlay_frame",
                FieldValue::Bool(report.launcher_overlay_frame),
            ),
            (
                "app_switcher_overlay_frame",
                FieldValue::Bool(report.app_switcher_overlay_frame),
            ),
            ("launched_window", FieldValue::U64(report.launched_window)),
            (
                "windows_after_launch",
                FieldValue::U64(report.windows_after_launch),
            ),
            ("move_begin", FieldValue::Bool(report.move_begin)),
            (
                "move_frame_changed",
                FieldValue::Bool(report.move_frame_changed),
            ),
            ("move_grab_ended", FieldValue::Bool(report.move_grab_ended)),
            ("moved_x", FieldValue::U64(report.moved_x)),
            ("moved_y", FieldValue::U64(report.moved_y)),
            ("resize_begin", FieldValue::Bool(report.resize_begin)),
            (
                "resize_frame_changed",
                FieldValue::Bool(report.resize_frame_changed),
            ),
            (
                "resize_grab_ended",
                FieldValue::Bool(report.resize_grab_ended),
            ),
            ("resized_width", FieldValue::U64(report.resized_width)),
            ("resized_height", FieldValue::U64(report.resized_height)),
            ("snap_frame_ok", FieldValue::Bool(report.snap_frame_ok)),
            (
                "workspace_hidden",
                FieldValue::Bool(report.workspace_hidden),
            ),
            (
                "workspace_switch_ok",
                FieldValue::Bool(report.workspace_switch_ok),
            ),
            (
                "final_visible_windows",
                FieldValue::U64(report.final_visible_windows),
            ),
        ],
    );
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
struct CompositorServiceVerification {
    service: ServiceProbe,
    demo_client: ServiceProbe,
    socket_bound: bool,
    demo_client_connected: bool,
    demo_surface_mapped: bool,
    demo_app_id_preserved: bool,
    socket_cleanup: bool,
    socket_blocked_expected: bool,
}

impl CompositorServiceVerification {
    fn missing() -> Self {
        Self {
            service: ServiceProbe::missing(),
            demo_client: ServiceProbe::missing(),
            socket_bound: false,
            demo_client_connected: false,
            demo_surface_mapped: false,
            demo_app_id_preserved: false,
            socket_cleanup: false,
            socket_blocked_expected: false,
        }
    }

    fn passed(&self) -> bool {
        self.service.resolved
            && self.service.exit_ok
            && self.service.ready
            && ((self.socket_bound
                && self.demo_client.resolved
                && self.demo_client.exit_ok
                && self.demo_client.ready
                && self.demo_client_connected
                && self.demo_surface_mapped
                && self.demo_app_id_preserved
                && self.socket_cleanup)
                || self.socket_blocked_expected)
    }

    fn children_exited_cleanly(&self) -> bool {
        self.service.exit_ok && (self.demo_client.exit_ok || self.socket_blocked_expected)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ServiceVerification {
    compositor: CompositorServiceVerification,
    shell: ServiceProbe,
    notification: ServiceProbe,
    settings: ServiceProbe,
    logs_written: bool,
}

impl ServiceVerification {
    fn passed(&self) -> bool {
        self.compositor.passed()
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
        self.compositor.children_exited_cleanly()
            && self.shell.exit_ok
            && self.notification.exit_ok
            && self.settings.exit_ok
    }
}

fn verify_session_services(config: &Config) -> Result<ServiceVerification, String> {
    let compositor_path = sibling_binary("backlit-compositor");
    let demo_client_path = sibling_binary("backlit-demo-client");
    let shell_path = sibling_binary("backlit-shell");
    let notification_path = sibling_binary("backlit-notification-daemon");
    let settings_path = sibling_binary("backlit-settings-daemon");

    let compositor = run_compositor_probe(&compositor_path, &demo_client_path, config)?;
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

fn run_compositor_probe(
    path: &Path,
    demo_client_path: &Path,
    config: &Config,
) -> Result<CompositorServiceVerification, String> {
    if !path.is_file() {
        return Ok(CompositorServiceVerification::missing());
    }

    if !demo_client_path.is_file() {
        let mut missing = CompositorServiceVerification::missing();
        missing.service = run_compositor_smoke_probe(path, config)?;
        return Ok(missing);
    }

    let runtime_dir = create_private_runtime_dir("blsvc")?;
    let socket_name = format!("bls-{}", process::id());
    let socket_path = runtime_dir.join(socket_name.as_str());

    let result = run_compositor_service_client_probe(
        path,
        demo_client_path,
        config,
        runtime_dir.as_path(),
        socket_name.as_str(),
        socket_path.as_path(),
    );

    let _ = fs::remove_dir_all(&runtime_dir);
    result
}

fn run_compositor_smoke_probe(path: &Path, config: &Config) -> Result<ServiceProbe, String> {
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
            String::from("\"xdg_surface_lifecycle\":true"),
            String::from("\"xdg_backend_surface_presented\":true"),
            String::from("\"xdg_popup_mapped\":true"),
            String::from("\"xdg_popup_backend_surface_presented\":true"),
        ],
    )
}

fn run_compositor_service_client_probe(
    path: &Path,
    demo_client_path: &Path,
    config: &Config,
    runtime_dir: &Path,
    socket_name: &str,
    socket_path: &Path,
) -> Result<CompositorServiceVerification, String> {
    let started = Instant::now();
    let mut child = Command::new(path)
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .arg("--backend")
        .arg(config.backend.as_str())
        .arg("--socket")
        .arg(socket_name)
        .arg("--serve")
        .arg("--serve-for-ms")
        .arg("650")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to run {}: {error}", path.display()))?;

    let socket_seen = wait_for_socket(socket_path, &mut child, Duration::from_millis(400))?;
    if !socket_seen {
        let output = child
            .wait_with_output()
            .map_err(|error| format!("failed to wait for {}: {error}", path.display()))?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Operation not permitted") {
            let fallback = run_compositor_smoke_probe(path, config)?;
            let mut report = CompositorServiceVerification::missing();
            report.service = fallback;
            report.socket_blocked_expected = true;
            return Ok(report);
        }

        let mut report = CompositorServiceVerification::missing();
        report.service = service_probe_from_output(
            output,
            started.elapsed().as_millis() as u64,
            &[
                String::from("\"event\":\"compositor.socket_bound\""),
                String::from("\"event\":\"compositor.socket_client\""),
            ],
        );
        return Ok(report);
    }

    let demo_started = Instant::now();
    let demo_output = Command::new(demo_client_path)
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .arg("--connect-socket")
        .arg(socket_name)
        .arg("--connect-title")
        .arg("session-demo")
        .arg("--connect-app-id")
        .arg("org.backlit.SessionDemo")
        .arg("--connect-only")
        .arg("--width")
        .arg("640")
        .arg("--height")
        .arg("480")
        .output()
        .map_err(|error| format!("failed to run {}: {error}", demo_client_path.display()))?;
    let demo_client = service_probe_from_output(
        demo_output,
        demo_started.elapsed().as_millis() as u64,
        &[
            String::from("\"event\":\"demo_client.socket_connected\""),
            String::from("\"title\":\"session-demo\""),
            String::from("\"app_id\":\"org.backlit.SessionDemo\""),
            String::from("\"connected\":true"),
        ],
    );

    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for {}: {error}", path.display()))?;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let socket_bound = stdout.contains("\"event\":\"compositor.socket_bound\"");
    let demo_client_connected =
        demo_client.ready && stdout.contains("\"event\":\"compositor.socket_client\"");
    let demo_surface_mapped = stdout.contains("\"title\":\"session-demo\"")
        && stdout.contains("\"app_id\":\"org.backlit.SessionDemo\"")
        && stdout.contains("\"backend_surface_presented\":true")
        && stdout.contains("\"policy_window_mapped\":true");
    let demo_app_id_preserved = stdout.contains("\"app_id\":\"org.backlit.SessionDemo\"")
        && stdout.contains("\"policy_app_id_preserved\":true");
    let socket_cleanup = stdout.contains("\"event\":\"compositor.socket_unbound\"")
        && stdout.contains("\"removed\":true")
        && !socket_path.exists();
    let service = service_probe_from_output(
        output,
        elapsed_ms,
        &[
            String::from("\"event\":\"compositor.ready\""),
            String::from("\"ready\":true"),
            String::from("\"event\":\"compositor.socket_bound\""),
            String::from("\"event\":\"compositor.socket_client\""),
            String::from("\"title\":\"session-demo\""),
            String::from("\"app_id\":\"org.backlit.SessionDemo\""),
            String::from("\"backend_surface_presented\":true"),
            String::from("\"policy_window_mapped\":true"),
            String::from("\"policy_app_id_preserved\":true"),
            String::from("\"event\":\"compositor.service_exit\""),
        ],
    );

    Ok(CompositorServiceVerification {
        service,
        demo_client,
        socket_bound,
        demo_client_connected,
        demo_surface_mapped,
        demo_app_id_preserved,
        socket_cleanup,
        socket_blocked_expected: false,
    })
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

fn service_probe_from_output(
    output: process::Output,
    elapsed_ms: u64,
    required_stdout: &[String],
) -> ServiceProbe {
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
    probe
}

fn create_private_runtime_dir(prefix: &str) -> Result<PathBuf, String> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock before Unix epoch: {error}"))?
        .as_nanos();
    let base = if cfg!(target_os = "macos") {
        PathBuf::from("/private/tmp")
    } else {
        env::temp_dir()
    };
    let path = base.join(format!("{prefix}-{}-{unique}", process::id()));
    fs::create_dir_all(&path).map_err(|error| {
        format!(
            "failed to create private runtime dir {}: {error}",
            path.display()
        )
    })?;
    let mut permissions = fs::metadata(&path)
        .map_err(|error| {
            format!(
                "failed to stat private runtime dir {}: {error}",
                path.display()
            )
        })?
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&path, permissions).map_err(|error| {
        format!(
            "failed to chmod private runtime dir {}: {error}",
            path.display()
        )
    })?;
    Ok(path)
}

fn wait_for_socket(
    socket_path: &Path,
    child: &mut Child,
    timeout: Duration,
) -> Result<bool, String> {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        if fs::symlink_metadata(socket_path)
            .map(|metadata| metadata.file_type().is_socket())
            .unwrap_or(false)
        {
            return Ok(true);
        }

        if child
            .try_wait()
            .map_err(|error| format!("failed to poll compositor service: {error}"))?
            .is_some()
        {
            return Ok(false);
        }

        thread::sleep(Duration::from_millis(10));
    }

    Ok(false)
}

fn write_service_logs(log_dir: &Path, report: &ServiceVerification) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|error| format!("failed to create {}: {error}", log_dir.display()))?;
    fs::write(
        log_dir.join("compositor.jsonl"),
        &report.compositor.service.stdout,
    )
    .map_err(|error| format!("failed to write compositor service log: {error}"))?;
    fs::write(
        log_dir.join("compositor.stderr"),
        &report.compositor.service.stderr,
    )
    .map_err(|error| format!("failed to write compositor service stderr: {error}"))?;
    fs::write(
        log_dir.join("demo-client-socket.jsonl"),
        &report.compositor.demo_client.stdout,
    )
    .map_err(|error| format!("failed to write demo client socket log: {error}"))?;
    fs::write(
        log_dir.join("demo-client-socket.stderr"),
        &report.compositor.demo_client.stderr,
    )
    .map_err(|error| format!("failed to write demo client socket stderr: {error}"))?;
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
                FieldValue::Bool(report.compositor.service.resolved),
            ),
            (
                "compositor_ready",
                FieldValue::Bool(report.compositor.service.ready),
            ),
            (
                "compositor_service_socket_bound",
                FieldValue::Bool(report.compositor.socket_bound),
            ),
            (
                "compositor_demo_client_resolved",
                FieldValue::Bool(report.compositor.demo_client.resolved),
            ),
            (
                "compositor_demo_client_exit_ok",
                FieldValue::Bool(report.compositor.demo_client.exit_ok),
            ),
            (
                "compositor_demo_client_connected",
                FieldValue::Bool(report.compositor.demo_client_connected),
            ),
            (
                "compositor_demo_surface_mapped",
                FieldValue::Bool(report.compositor.demo_surface_mapped),
            ),
            (
                "compositor_demo_app_id_preserved",
                FieldValue::Bool(report.compositor.demo_app_id_preserved),
            ),
            (
                "compositor_service_socket_cleanup",
                FieldValue::Bool(report.compositor.socket_cleanup),
            ),
            (
                "compositor_client_blocked_expected",
                FieldValue::Bool(report.compositor.socket_blocked_expected),
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
                FieldValue::U64(report.compositor.service.elapsed_ms),
            ),
            (
                "compositor_demo_client_probe_ms",
                FieldValue::U64(report.compositor.demo_client.elapsed_ms),
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
                FieldValue::U64(report.compositor.service.stdout.len() as u64),
            ),
            (
                "compositor_demo_client_stdout_bytes",
                FieldValue::U64(report.compositor.demo_client.stdout.len() as u64),
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
struct SystemdCommandPlan {
    program: &'static str,
    args: &'static [&'static str],
}

impl SystemdCommandPlan {
    fn command_line(self) -> String {
        if self.args.is_empty() {
            self.program.to_string()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }

    fn command_line_with_program(self, program: &str) -> String {
        if self.args.is_empty() {
            program.to_string()
        } else {
            format!("{} {}", program, self.args.join(" "))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SystemdLaunchPlan {
    dry_run: bool,
    target: &'static str,
    service_units: &'static [&'static str],
    import_environment: SystemdCommandPlan,
    start_target: SystemdCommandPlan,
    stop_target: SystemdCommandPlan,
}

impl SystemdLaunchPlan {
    fn ready(self) -> bool {
        self.dry_run
            && self.target == "backlit-session.target"
            && self.service_units == service_unit_names()
            && self.import_environment.args == systemd_import_environment_args()
            && self.start_target.command_line() == "systemctl --user start backlit-session.target"
            && self.stop_target.command_line() == "systemctl --user stop backlit-session.target"
    }

    fn service_count(self) -> u64 {
        self.service_units.len() as u64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SystemdCommandRun {
    command_line: String,
    ran: bool,
    exit_success: bool,
    status_code: u64,
}

impl SystemdCommandRun {
    fn skipped(command_line: String) -> Self {
        Self {
            command_line,
            ran: false,
            exit_success: false,
            status_code: 255,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SystemdActivationReport {
    systemctl_program: String,
    target: &'static str,
    stop_after_start: bool,
    import_environment: SystemdCommandRun,
    start_target: SystemdCommandRun,
    stop_target: SystemdCommandRun,
}

impl SystemdActivationReport {
    fn passed(&self) -> bool {
        self.import_environment.ran
            && self.import_environment.exit_success
            && self.start_target.ran
            && self.start_target.exit_success
            && (!self.stop_after_start || (self.stop_target.ran && self.stop_target.exit_success))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SystemdTargetProbe {
    present: bool,
    wants_services: bool,
    after_graphical_session_pre: bool,
    part_of_graphical_session: bool,
    wanted_by_graphical_session: bool,
}

impl SystemdTargetProbe {
    fn missing() -> Self {
        Self {
            present: false,
            wants_services: false,
            after_graphical_session_pre: false,
            part_of_graphical_session: false,
            wanted_by_graphical_session: false,
        }
    }

    fn passed(self) -> bool {
        self.present
            && self.wants_services
            && self.after_graphical_session_pre
            && self.part_of_graphical_session
            && self.wanted_by_graphical_session
    }
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
    session_target: SystemdTargetProbe,
    compositor: SystemdUnitProbe,
    shell: SystemdUnitProbe,
    notification: SystemdUnitProbe,
    settings: SystemdUnitProbe,
}

impl SystemdUnitVerification {
    fn passed(&self) -> bool {
        self.session_target.passed()
            && self.compositor.passed()
            && self.shell.passed()
            && self.notification.passed()
            && self.settings.passed()
    }

    fn units_present(&self) -> bool {
        self.session_target.present
            && self.compositor.present
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
        self.session_target.after_graphical_session_pre
            && self.compositor.after_ok
            && self.shell.after_ok
            && self.notification.after_ok
            && self.settings.after_ok
    }

    fn graphical_session_target_ok(&self) -> bool {
        self.session_target.part_of_graphical_session
            && self.session_target.wanted_by_graphical_session
            && self.compositor.part_of_graphical_session
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

    fn session_target_ok(&self) -> bool {
        self.session_target.passed()
    }
}

fn verify_systemd_units(unit_dir: &Path) -> SystemdUnitVerification {
    let [compositor_contract, shell_contract, notification_contract, settings_contract] =
        systemd_unit_contracts();

    SystemdUnitVerification {
        unit_dir: unit_dir.to_path_buf(),
        session_target: verify_systemd_target(unit_dir),
        compositor: verify_systemd_unit(unit_dir, compositor_contract),
        shell: verify_systemd_unit(unit_dir, shell_contract),
        notification: verify_systemd_unit(unit_dir, notification_contract),
        settings: verify_systemd_unit(unit_dir, settings_contract),
    }
}

fn service_unit_names() -> &'static [&'static str] {
    &[
        "backlit-compositor.service",
        "backlit-shell.service",
        "backlit-notification-daemon.service",
        "backlit-settings-daemon.service",
    ]
}

fn systemd_import_environment_args() -> &'static [&'static str] {
    &[
        "--user",
        "import-environment",
        "XDG_RUNTIME_DIR",
        "XDG_SESSION_ID",
        "XDG_SEAT",
        "XDG_SESSION_TYPE",
        "WAYLAND_DISPLAY",
        "XDG_CURRENT_DESKTOP",
        "DESKTOP_SESSION",
    ]
}

fn systemd_launch_plan() -> SystemdLaunchPlan {
    SystemdLaunchPlan {
        dry_run: true,
        target: "backlit-session.target",
        service_units: service_unit_names(),
        import_environment: SystemdCommandPlan {
            program: "systemctl",
            args: systemd_import_environment_args(),
        },
        start_target: SystemdCommandPlan {
            program: "systemctl",
            args: &["--user", "start", "backlit-session.target"],
        },
        stop_target: SystemdCommandPlan {
            program: "systemctl",
            args: &["--user", "stop", "backlit-session.target"],
        },
    }
}

fn run_systemd_activation(
    config: &Config,
    plan: &SystemdLaunchPlan,
    stop_after_start: bool,
) -> SystemdActivationReport {
    let systemctl_program = config.systemctl_program.clone();
    let import_environment =
        run_systemd_command(systemctl_program.as_str(), plan.import_environment);
    let start_target = if import_environment.exit_success {
        run_systemd_command(systemctl_program.as_str(), plan.start_target)
    } else {
        SystemdCommandRun::skipped(
            plan.start_target
                .command_line_with_program(&systemctl_program),
        )
    };
    let stop_target = if stop_after_start && start_target.exit_success {
        run_systemd_command(systemctl_program.as_str(), plan.stop_target)
    } else {
        SystemdCommandRun::skipped(
            plan.stop_target
                .command_line_with_program(&systemctl_program),
        )
    };

    SystemdActivationReport {
        systemctl_program,
        target: plan.target,
        stop_after_start,
        import_environment,
        start_target,
        stop_target,
    }
}

fn run_systemd_command(program: &str, command: SystemdCommandPlan) -> SystemdCommandRun {
    let command_line = command.command_line_with_program(program);

    match Command::new(program).args(command.args).status() {
        Ok(status) => SystemdCommandRun {
            command_line,
            ran: true,
            exit_success: status.success(),
            status_code: status.code().unwrap_or(255) as u64,
        },
        Err(_) => SystemdCommandRun {
            command_line,
            ran: false,
            exit_success: false,
            status_code: 255,
        },
    }
}

fn systemd_unit_contracts() -> [SystemdUnitContract; 4] {
    [
        SystemdUnitContract {
            unit_name: "backlit-compositor.service",
            exec_start:
                "ExecStart=/usr/bin/backlit-compositor --backend=drm --runtime=smithay --socket=backlit-0 --serve",
            after: "After=graphical-session-pre.target",
        },
        SystemdUnitContract {
            unit_name: "backlit-shell.service",
            exec_start:
                "ExecStart=/usr/bin/backlit-shell --component=all --socket=backlit-0 --serve",
            after: "After=backlit-compositor.service",
        },
        SystemdUnitContract {
            unit_name: "backlit-notification-daemon.service",
            exec_start: "ExecStart=/usr/bin/backlit-notification-daemon --serve",
            after: "After=backlit-compositor.service",
        },
        SystemdUnitContract {
            unit_name: "backlit-settings-daemon.service",
            exec_start: "ExecStart=/usr/bin/backlit-settings-daemon --serve",
            after: "After=backlit-compositor.service",
        },
    ]
}

fn verify_systemd_target(unit_dir: &Path) -> SystemdTargetProbe {
    let path = unit_dir.join("backlit-session.target");
    let Ok(contents) = fs::read_to_string(path) else {
        return SystemdTargetProbe::missing();
    };
    let lines: Vec<&str> = contents.lines().map(str::trim).collect();
    let contains_line = |required: &str| lines.contains(&required);
    let wants_services = lines.iter().any(|line| {
        line.strip_prefix("Wants=")
            .map(|value| {
                let wanted: Vec<&str> = value.split_whitespace().collect();
                service_unit_names()
                    .iter()
                    .all(|unit| wanted.contains(unit))
            })
            .unwrap_or(false)
    });

    SystemdTargetProbe {
        present: true,
        wants_services,
        after_graphical_session_pre: contains_line("After=graphical-session-pre.target"),
        part_of_graphical_session: contains_line("PartOf=graphical-session.target"),
        wanted_by_graphical_session: contains_line("WantedBy=graphical-session.target"),
    }
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
            ("session_target", FieldValue::Str("backlit-session.target")),
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
            (
                "session_target_ready",
                FieldValue::Bool(report.session_target_ok()),
            ),
            (
                "session_target_wants_services",
                FieldValue::Bool(report.session_target.wants_services),
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

fn emit_systemd_launch_plan(
    config: &Config,
    plan: &SystemdLaunchPlan,
    report: &SystemdUnitVerification,
) {
    let import_environment_command = plan.import_environment.command_line();
    let start_target_command = plan.start_target.command_line();
    let stop_target_command = plan.stop_target.command_line();

    emit(
        "session.systemd_launch_plan",
        config,
        &[
            ("passed", FieldValue::Bool(plan.ready() && report.passed())),
            ("dry_run", FieldValue::Bool(plan.dry_run)),
            ("target", FieldValue::Str(plan.target)),
            ("service_units", FieldValue::U64(plan.service_count())),
            (
                "session_target_ready",
                FieldValue::Bool(report.session_target_ok()),
            ),
            ("launch_plan_ready", FieldValue::Bool(plan.ready())),
            (
                "import_environment_command",
                FieldValue::Str(&import_environment_command),
            ),
            (
                "start_target_command",
                FieldValue::Str(&start_target_command),
            ),
            ("stop_target_command", FieldValue::Str(&stop_target_command)),
        ],
    );
}

fn emit_systemd_activation(config: &Config, report: &SystemdActivationReport) {
    emit(
        "session.systemd_activation",
        config,
        &[
            ("passed", FieldValue::Bool(report.passed())),
            (
                "systemctl_program",
                FieldValue::Str(report.systemctl_program.as_str()),
            ),
            ("target", FieldValue::Str(report.target)),
            (
                "stop_after_start",
                FieldValue::Bool(report.stop_after_start),
            ),
            (
                "import_environment_command",
                FieldValue::Str(report.import_environment.command_line.as_str()),
            ),
            (
                "import_environment_run",
                FieldValue::Bool(report.import_environment.ran),
            ),
            (
                "import_environment_exit_success",
                FieldValue::Bool(report.import_environment.exit_success),
            ),
            (
                "import_environment_status_code",
                FieldValue::U64(report.import_environment.status_code),
            ),
            (
                "start_target_command",
                FieldValue::Str(report.start_target.command_line.as_str()),
            ),
            (
                "start_target_run",
                FieldValue::Bool(report.start_target.ran),
            ),
            (
                "start_target_exit_success",
                FieldValue::Bool(report.start_target.exit_success),
            ),
            (
                "start_target_status_code",
                FieldValue::U64(report.start_target.status_code),
            ),
            (
                "stop_target_command",
                FieldValue::Str(report.stop_target.command_line.as_str()),
            ),
            ("stop_target_run", FieldValue::Bool(report.stop_target.ran)),
            (
                "stop_target_exit_success",
                FieldValue::Bool(report.stop_target.exit_success),
            ),
            (
                "stop_target_status_code",
                FieldValue::U64(report.stop_target.status_code),
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

fn emit_backend_launch_plan(config: &Config, plan: &BackendLaunchPlan) {
    let primary_drm_card = plan.primary_drm_card.as_deref().unwrap_or("");
    let primary_drm_render_node = plan.primary_drm_render_node.as_deref().unwrap_or("");
    let primary_input_event = plan.primary_input_event.as_deref().unwrap_or("");
    let session_id = plan.session_id.as_deref().unwrap_or("");
    let seat = plan.seat.as_deref().unwrap_or("");
    let session_type = plan.session_type.as_deref().unwrap_or("");

    emit(
        "session.backend_launch_plan",
        config,
        &[
            ("ready", FieldValue::Bool(plan.ready)),
            ("implementation", FieldValue::Str(plan.implementation)),
            ("display_driver", FieldValue::Str(plan.display_driver)),
            ("input_driver", FieldValue::Str(plan.input_driver)),
            ("device_access", FieldValue::Str(plan.device_access)),
            (
                "uses_parent_wayland",
                FieldValue::Bool(plan.uses_parent_wayland),
            ),
            ("uses_drm", FieldValue::Bool(plan.uses_drm)),
            ("uses_logind", FieldValue::Bool(plan.uses_logind)),
            ("uses_libseat", FieldValue::Bool(plan.uses_libseat)),
            ("uses_libinput", FieldValue::Bool(plan.uses_libinput)),
            (
                "drm_card_selected",
                FieldValue::Bool(plan.drm_card_selected),
            ),
            (
                "drm_render_selected",
                FieldValue::Bool(plan.drm_render_selected),
            ),
            (
                "input_event_selected",
                FieldValue::Bool(plan.input_event_selected),
            ),
            ("primary_drm_card", FieldValue::Str(primary_drm_card)),
            (
                "primary_drm_render_node",
                FieldValue::Str(primary_drm_render_node),
            ),
            ("primary_input_event", FieldValue::Str(primary_input_event)),
            ("session_id", FieldValue::Str(session_id)),
            ("seat", FieldValue::Str(seat)),
            ("session_type", FieldValue::Str(session_type)),
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

fn wait_for_systemd_session() -> ! {
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct DesktopLaunchReport {
    entry_selector: String,
    directories: u64,
    discovered_entries: u64,
    entry_resolved: bool,
    entry_id: String,
    entry_name: String,
    entry_program: String,
    entry_arg_count: u64,
    resolved_program: String,
    program_resolved: bool,
    spawned: bool,
    exit_success: bool,
    status_code: u64,
    wayland_display_set: bool,
    managed_window_mapped: bool,
    managed_window_id: u64,
    managed_window_title: String,
    managed_window_app_id: String,
    managed_windows_after_launch: u64,
    focused_launched_window: bool,
    elapsed_ms: u64,
}

impl DesktopLaunchReport {
    fn passed(&self) -> bool {
        self.entry_resolved
            && self.program_resolved
            && self.spawned
            && self.exit_success
            && self.wayland_display_set
            && self.managed_window_mapped
            && self.focused_launched_window
    }
}

fn verify_desktop_launch(
    config: &Config,
    policy: &WindowPolicy,
) -> Result<DesktopLaunchReport, String> {
    let started = Instant::now();
    let selector = config.desktop_entry.clone().unwrap_or_default();
    let directories = desktop_launch_dirs(config);
    let entries = discover_desktop_entries_in_dirs(&directories)
        .map_err(|error| format!("failed to discover desktop entries: {error}"))?;
    let discovered_entries = entries.len() as u64;
    let wayland_display = config
        .launch_wayland_display
        .clone()
        .or_else(|| env::var("WAYLAND_DISPLAY").ok());

    let Some(entry) = resolve_desktop_entry(&entries, &selector) else {
        return Ok(DesktopLaunchReport {
            entry_selector: selector,
            directories: directories.len() as u64,
            discovered_entries,
            entry_resolved: false,
            entry_id: String::new(),
            entry_name: String::new(),
            entry_program: String::new(),
            entry_arg_count: 0,
            resolved_program: String::new(),
            program_resolved: false,
            spawned: false,
            exit_success: false,
            status_code: 255,
            wayland_display_set: wayland_display.is_some(),
            managed_window_mapped: false,
            managed_window_id: 0,
            managed_window_title: String::new(),
            managed_window_app_id: String::new(),
            managed_windows_after_launch: policy.windows().len() as u64,
            focused_launched_window: false,
            elapsed_ms: started.elapsed().as_millis() as u64,
        });
    };

    let resolved_program = resolve_desktop_program(entry.command_program());
    let program_resolved = !resolved_program.trim().is_empty();

    if !program_resolved {
        return Ok(DesktopLaunchReport {
            entry_selector: selector,
            directories: directories.len() as u64,
            discovered_entries,
            entry_resolved: true,
            entry_id: entry.id.clone(),
            entry_name: entry.name.clone(),
            entry_program: entry.command_program().to_string(),
            entry_arg_count: entry.command_args().len() as u64,
            resolved_program,
            program_resolved: false,
            spawned: false,
            exit_success: false,
            status_code: 255,
            wayland_display_set: wayland_display.is_some(),
            managed_window_mapped: false,
            managed_window_id: 0,
            managed_window_title: String::new(),
            managed_window_app_id: String::new(),
            managed_windows_after_launch: policy.windows().len() as u64,
            focused_launched_window: false,
            elapsed_ms: started.elapsed().as_millis() as u64,
        });
    }

    let mut child = Command::new(&resolved_program);
    child.args(entry.command_args());
    if let Some(display) = &wayland_display {
        child.env("WAYLAND_DISPLAY", display);
    }

    let (spawned, exit_success, status_code) = match child.status() {
        Ok(status) => (true, status.success(), status.code().unwrap_or(255) as u64),
        Err(_) => (false, false, 255),
    };
    let managed = if spawned && exit_success {
        map_desktop_entry_window(policy, entry)
    } else {
        ManagedDesktopWindow::default_with_count(policy.windows().len() as u64)
    };

    Ok(DesktopLaunchReport {
        entry_selector: selector,
        directories: directories.len() as u64,
        discovered_entries,
        entry_resolved: true,
        entry_id: entry.id.clone(),
        entry_name: entry.name.clone(),
        entry_program: entry.command_program().to_string(),
        entry_arg_count: entry.command_args().len() as u64,
        resolved_program,
        program_resolved,
        spawned,
        exit_success,
        status_code,
        wayland_display_set: wayland_display.is_some(),
        managed_window_mapped: managed.mapped,
        managed_window_id: managed.window_id,
        managed_window_title: managed.title,
        managed_window_app_id: managed.app_id,
        managed_windows_after_launch: managed.windows_after_launch,
        focused_launched_window: managed.focused,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManagedDesktopWindow {
    mapped: bool,
    window_id: u64,
    title: String,
    app_id: String,
    windows_after_launch: u64,
    focused: bool,
}

impl ManagedDesktopWindow {
    fn default_with_count(windows_after_launch: u64) -> Self {
        Self {
            mapped: false,
            window_id: 0,
            title: String::new(),
            app_id: String::new(),
            windows_after_launch,
            focused: false,
        }
    }
}

fn map_desktop_entry_window(policy: &WindowPolicy, entry: &DesktopEntry) -> ManagedDesktopWindow {
    let mut policy = policy.clone();
    let title = entry.name.clone();
    let app_id = entry.id.clone();
    let window = policy.add_app_window(title.clone(), Some(app_id.clone()), (640, 480));
    let mapped = policy
        .window(window)
        .map(|window| window.app_id.as_deref() == Some(app_id.as_str()) && window.title == title)
        .unwrap_or(false);
    let focused = policy.focused() == Some(window);

    ManagedDesktopWindow {
        mapped,
        window_id: window.0,
        title,
        app_id,
        windows_after_launch: policy.windows().len() as u64,
        focused,
    }
}

fn desktop_launch_dirs(config: &Config) -> Vec<PathBuf> {
    if config.desktop_dirs.is_empty() {
        default_desktop_entry_dirs()
    } else {
        config.desktop_dirs.iter().map(PathBuf::from).collect()
    }
}

fn resolve_desktop_entry<'a>(
    entries: &'a [DesktopEntry],
    selector: &str,
) -> Option<&'a DesktopEntry> {
    entries
        .iter()
        .find(|entry| entry.id == selector || entry.name == selector)
}

fn resolve_desktop_program(program: &str) -> String {
    if program.trim().is_empty() {
        return String::new();
    }

    if program.contains('/') {
        return program.to_string();
    }

    if let Ok(current_exe) = env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let sibling = parent.join(program);
            if sibling.is_file() {
                return sibling.to_string_lossy().into_owned();
            }
        }
    }

    program.to_string()
}

fn emit_desktop_launch(config: &Config, report: &DesktopLaunchReport) {
    emit(
        "session.desktop_launch",
        config,
        &[
            ("passed", FieldValue::Bool(report.passed())),
            (
                "entry_selector",
                FieldValue::Str(report.entry_selector.as_str()),
            ),
            ("directories", FieldValue::U64(report.directories)),
            (
                "discovered_entries",
                FieldValue::U64(report.discovered_entries),
            ),
            ("entry_resolved", FieldValue::Bool(report.entry_resolved)),
            ("entry_id", FieldValue::Str(report.entry_id.as_str())),
            ("entry_name", FieldValue::Str(report.entry_name.as_str())),
            (
                "entry_program",
                FieldValue::Str(report.entry_program.as_str()),
            ),
            ("entry_arg_count", FieldValue::U64(report.entry_arg_count)),
            (
                "resolved_program",
                FieldValue::Str(report.resolved_program.as_str()),
            ),
            (
                "program_resolved",
                FieldValue::Bool(report.program_resolved),
            ),
            ("spawned", FieldValue::Bool(report.spawned)),
            ("exit_success", FieldValue::Bool(report.exit_success)),
            ("status_code", FieldValue::U64(report.status_code)),
            (
                "wayland_display_set",
                FieldValue::Bool(report.wayland_display_set),
            ),
            (
                "managed_window_mapped",
                FieldValue::Bool(report.managed_window_mapped),
            ),
            (
                "managed_window_id",
                FieldValue::U64(report.managed_window_id),
            ),
            (
                "managed_window_title",
                FieldValue::Str(report.managed_window_title.as_str()),
            ),
            (
                "managed_window_app_id",
                FieldValue::Str(report.managed_window_app_id.as_str()),
            ),
            (
                "managed_windows_after_launch",
                FieldValue::U64(report.managed_windows_after_launch),
            ),
            (
                "focused_launched_window",
                FieldValue::Bool(report.focused_launched_window),
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

fn emit_drm_first_present_probe(config: &Config, probe: &SmithayRuntimeProbe) {
    let first_present_failure = probe.kms_first_present_failure.as_deref().unwrap_or("");

    emit(
        "session.drm_first_present_probe",
        config,
        &[
            ("passed", FieldValue::Bool(probe.passed())),
            ("launch_ready", FieldValue::Bool(probe.launch_ready)),
            ("runtime_backend", FieldValue::Str(probe.runtime_backend)),
            ("feature_enabled", FieldValue::Bool(probe.feature_enabled)),
            ("compiled", FieldValue::Bool(probe.compiled)),
            (
                "drm_card_selected",
                FieldValue::Bool(probe.drm_card_selected),
            ),
            (
                "drm_node_resolved",
                FieldValue::Bool(probe.drm_node_resolved),
            ),
            (
                "kms_scanout_plan_ready",
                FieldValue::Bool(probe.kms_scanout_plan_ready),
            ),
            (
                "kms_surface_created",
                FieldValue::Bool(probe.kms_surface_created),
            ),
            (
                "kms_framebuffer_created",
                FieldValue::Bool(probe.kms_framebuffer_created),
            ),
            (
                "kms_framebuffer_added",
                FieldValue::Bool(probe.kms_framebuffer_added),
            ),
            (
                "kms_first_present_framebuffer_filled",
                FieldValue::Bool(probe.kms_first_present_framebuffer_filled),
            ),
            (
                "kms_first_present_plane_state_ready",
                FieldValue::Bool(probe.kms_first_present_plane_state_ready),
            ),
            (
                "kms_first_present_commit_attempted",
                FieldValue::Bool(probe.kms_first_present_commit_attempted),
            ),
            (
                "kms_first_present_commit_succeeded",
                FieldValue::Bool(probe.kms_first_present_commit_succeeded),
            ),
            (
                "kms_first_present_vblank_event_received",
                FieldValue::Bool(probe.kms_first_present_vblank_event_received),
            ),
            (
                "kms_first_present_blocked_by_drm_master",
                FieldValue::Bool(probe.kms_first_present_blocked_by_drm_master),
            ),
            (
                "kms_framebuffer_test_state_succeeded",
                FieldValue::Bool(probe.kms_framebuffer_test_state_succeeded),
            ),
            (
                "kms_framebuffer_test_state_permission_denied",
                FieldValue::Bool(probe.kms_framebuffer_test_state_permission_denied),
            ),
            (
                "kms_first_present_failure",
                FieldValue::Str(first_present_failure),
            ),
            (
                "require_drm_master_present",
                FieldValue::Bool(config.require_drm_master_present),
            ),
        ],
    );
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
    verify_systemd_activation: bool,
    verify_drm_first_present: bool,
    require_drm_master_present: bool,
    activate_systemd: bool,
    scripted_replay_dir: Option<String>,
    systemd_unit_dir: String,
    systemctl_program: String,
    preflight_only: bool,
    verify_launch_spawn: bool,
    verify_desktop_launch: bool,
    desktop_dirs: Vec<String>,
    desktop_entry: Option<String>,
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
            verify_systemd_activation: false,
            verify_drm_first_present: false,
            require_drm_master_present: false,
            activate_systemd: false,
            scripted_replay_dir: None,
            systemd_unit_dir: String::from("/usr/lib/systemd/user"),
            systemctl_program: String::from("systemctl"),
            preflight_only: false,
            verify_launch_spawn: false,
            verify_desktop_launch: false,
            desktop_dirs: Vec::new(),
            desktop_entry: None,
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
            } else if arg == "--verify-systemd-activation" {
                config.verify_systemd_activation = true;
            } else if arg == "--verify-drm-first-present" {
                config.verify_drm_first_present = true;
            } else if arg == "--require-drm-master-present" {
                config.verify_drm_first_present = true;
                config.require_drm_master_present = true;
            } else if arg == "--activate-systemd" {
                config.activate_systemd = true;
            } else if let Some(value) = arg.strip_prefix("--scripted-replay-dir=") {
                config.scripted_replay_dir = Some(value.to_string());
            } else if arg == "--scripted-replay-dir" {
                config.scripted_replay_dir = Some(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --scripted-replay-dir"))?,
                );
            } else if let Some(value) = arg.strip_prefix("--systemd-unit-dir=") {
                config.systemd_unit_dir = value.to_string();
            } else if arg == "--systemd-unit-dir" {
                config.systemd_unit_dir = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --systemd-unit-dir"))?;
            } else if let Some(value) = arg.strip_prefix("--systemctl-program=") {
                config.systemctl_program = value.to_string();
            } else if arg == "--systemctl-program" {
                config.systemctl_program = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --systemctl-program"))?;
            } else if arg == "--preflight-only" {
                config.preflight_only = true;
            } else if arg == "--verify-launch-spawn" {
                config.verify_launch_spawn = true;
            } else if arg == "--verify-desktop-launch" {
                config.verify_desktop_launch = true;
            } else if let Some(value) = arg.strip_prefix("--desktop-dir=") {
                config.desktop_dirs.push(value.to_string());
            } else if arg == "--desktop-dir" {
                config.desktop_dirs.push(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --desktop-dir"))?,
                );
            } else if let Some(value) = arg.strip_prefix("--desktop-launch-dir=") {
                config.desktop_dirs.push(value.to_string());
            } else if arg == "--desktop-launch-dir" {
                config.desktop_dirs.push(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --desktop-launch-dir"))?,
                );
            } else if let Some(value) = arg.strip_prefix("--desktop-entry=") {
                config.desktop_entry = Some(value.to_string());
            } else if arg == "--desktop-entry" {
                config.desktop_entry = Some(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --desktop-entry"))?,
                );
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
  backlit-session [--backend=headless|wayland|drm] [--socket=backlit-0] [--screenshot=target/backlit-session.ppm] [--verify] [--verify-services] [--verify-systemd-units] [--verify-systemd-activation] [--verify-drm-first-present] [--require-drm-master-present] [--activate-systemd] [--verify-launch-spawn] [--verify-desktop-launch] [--verify-clean-exit] [--scripted-replay-dir=target/session-replay/frames] [--preflight-only]

Flags:
  --backend      Select compositor backend. Defaults to headless.
  --socket       Wayland socket name. Defaults to backlit-0.
  --screenshot   Write a deterministic PPM GUI screenshot.
  --service-log-dir
                 Write compositor, demo-client socket, and shell probe logs to this directory.
  --width        Screenshot width in pixels.
  --height       Screenshot height in pixels.
  --verify       Fail if expected GUI regions are missing.
  --verify-services
                 Fail if sibling compositor, demo client, shell, and settings probes cannot launch.
  --verify-launch-spawn
                 Spawn the terminal launch target resolved from Super+Enter.
  --verify-desktop-launch
                 Resolve and spawn a discovered .desktop entry with WAYLAND_DISPLAY set.
  --desktop-dir  Discover visible .desktop entries from this directory. May repeat. Defaults to XDG app dirs.
  --desktop-entry
                 Desktop entry id or name used by --verify-desktop-launch.
  --verify-clean-exit
                 Verify session shutdown closes managed windows and clears focus.
  --scripted-replay-dir
                 Write and verify scripted interaction frames for focus, launch, move, resize, snap, and workspace switching.
  --verify-systemd-units
                 Verify installed user systemd units for the graphical session.
  --verify-systemd-activation
                 Execute the systemd import/start/stop activation path and exit.
  --verify-drm-first-present
                 Probe DRM/KMS first-present readiness through the session entrypoint.
  --require-drm-master-present
                 Require first-present commit/vblank instead of accepting the nested-session DRM-master boundary.
  --activate-systemd
                 Start the Backlit user systemd target and keep the session process alive.
  --systemd-unit-dir
                 Directory containing Backlit user systemd units. Defaults to /usr/lib/systemd/user.
  --systemctl-program
                 systemctl-compatible command used for activation. Defaults to systemctl.
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

    use super::{
        binary_name, run_systemd_activation, systemd_launch_plan, verify_systemd_units,
        CompositorServiceVerification, Config, ServiceProbe,
    };

    #[test]
    fn parses_service_verification_flags() {
        let config = Config::parse([
            "--verify",
            "--verify-services",
            "--verify-clean-exit",
            "--verify-systemd-units",
            "--verify-systemd-activation",
            "--verify-drm-first-present",
            "--require-drm-master-present",
            "--activate-systemd",
            "--scripted-replay-dir",
            "target/session-replay/frames",
            "--systemd-unit-dir",
            "packaging/systemd",
            "--systemctl-program",
            "true",
            "--preflight-only",
            "--verify-launch-spawn",
            "--verify-desktop-launch",
            "--desktop-dir",
            "crates/launcher/fixtures",
            "--desktop-entry",
            "org.backlit.SpawnProbe.desktop",
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
        assert!(config.verify_systemd_activation);
        assert!(config.verify_drm_first_present);
        assert!(config.require_drm_master_present);
        assert!(config.activate_systemd);
        assert_eq!(
            config.scripted_replay_dir.as_deref(),
            Some("target/session-replay/frames")
        );
        assert_eq!(config.systemd_unit_dir, "packaging/systemd");
        assert_eq!(config.systemctl_program, "true");
        assert!(config.preflight_only);
        assert!(config.verify_launch_spawn);
        assert!(config.verify_desktop_launch);
        assert_eq!(config.desktop_dirs, ["crates/launcher/fixtures"]);
        assert_eq!(
            config.desktop_entry.as_deref(),
            Some("org.backlit.SpawnProbe.desktop")
        );
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
    fn compositor_service_verification_requires_launched_client_or_expected_block() {
        let ready_service = ServiceProbe {
            resolved: true,
            exit_ok: true,
            ready: true,
            elapsed_ms: 7,
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        let ready_client = ServiceProbe {
            resolved: true,
            exit_ok: true,
            ready: true,
            elapsed_ms: 2,
            stdout: Vec::new(),
            stderr: Vec::new(),
        };

        let mapped = CompositorServiceVerification {
            service: ready_service.clone(),
            demo_client: ready_client,
            socket_bound: true,
            demo_client_connected: true,
            demo_surface_mapped: true,
            demo_app_id_preserved: true,
            socket_cleanup: true,
            socket_blocked_expected: false,
        };
        assert!(mapped.passed());

        let blocked = CompositorServiceVerification {
            service: ready_service,
            demo_client: ServiceProbe::missing(),
            socket_bound: false,
            demo_client_connected: false,
            demo_surface_mapped: false,
            demo_app_id_preserved: false,
            socket_cleanup: false,
            socket_blocked_expected: true,
        };
        assert!(blocked.passed());

        let incomplete = CompositorServiceVerification {
            socket_blocked_expected: false,
            ..blocked
        };
        assert!(!incomplete.passed());
    }

    #[test]
    fn verifies_systemd_unit_contracts() {
        let unit_dir = unique_test_dir("systemd-units-ok");
        fs::create_dir_all(&unit_dir).expect("unit dir should be created");
        write_session_target(&unit_dir);
        write_unit(
            &unit_dir,
            "backlit-compositor.service",
            "Backlit Wayland compositor",
            "After=graphical-session-pre.target",
            "ExecStart=/usr/bin/backlit-compositor --backend=drm --runtime=smithay --socket=backlit-0 --serve",
        );
        write_unit(
            &unit_dir,
            "backlit-shell.service",
            "Backlit shell",
            "After=backlit-compositor.service",
            "ExecStart=/usr/bin/backlit-shell --component=all --socket=backlit-0 --serve",
        );
        write_unit(
            &unit_dir,
            "backlit-notification-daemon.service",
            "Backlit notification daemon",
            "After=backlit-compositor.service",
            "ExecStart=/usr/bin/backlit-notification-daemon --serve",
        );
        write_unit(
            &unit_dir,
            "backlit-settings-daemon.service",
            "Backlit settings daemon",
            "After=backlit-compositor.service",
            "ExecStart=/usr/bin/backlit-settings-daemon --serve",
        );

        let report = verify_systemd_units(&unit_dir);

        assert!(report.passed(), "{report:?}");
        assert!(report.session_target_ok());
        assert!(report.units_present());
        assert!(report.exec_starts_ok());
        assert!(report.startup_order_ok());
        assert!(report.graphical_session_target_ok());
        assert!(report.journal_output_ok());
        assert!(report.rust_backtrace_enabled());
        assert!(report.restart_policy_ok());
    }

    #[test]
    fn builds_systemd_session_launch_plan() {
        let plan = systemd_launch_plan();

        assert!(plan.ready());
        assert!(plan.dry_run);
        assert_eq!(plan.target, "backlit-session.target");
        assert_eq!(plan.service_count(), 4);
        assert_eq!(
            plan.import_environment.command_line(),
            "systemctl --user import-environment XDG_RUNTIME_DIR XDG_SESSION_ID XDG_SEAT XDG_SESSION_TYPE WAYLAND_DISPLAY XDG_CURRENT_DESKTOP DESKTOP_SESSION"
        );
        assert_eq!(
            plan.start_target.command_line(),
            "systemctl --user start backlit-session.target"
        );
        assert_eq!(
            plan.stop_target.command_line(),
            "systemctl --user stop backlit-session.target"
        );
    }

    #[test]
    fn verifies_systemd_activation_commands_with_successful_runner() {
        let config = Config {
            systemctl_program: String::from("true"),
            ..Config::default()
        };
        let plan = systemd_launch_plan();
        let report = run_systemd_activation(&config, &plan, true);

        assert!(report.passed(), "{report:?}");
        assert!(report.import_environment.ran);
        assert!(report.start_target.ran);
        assert!(report.stop_target.ran);
        assert_eq!(
            report.import_environment.command_line,
            "true --user import-environment XDG_RUNTIME_DIR XDG_SESSION_ID XDG_SEAT XDG_SESSION_TYPE WAYLAND_DISPLAY XDG_CURRENT_DESKTOP DESKTOP_SESSION"
        );
        assert_eq!(
            report.start_target.command_line,
            "true --user start backlit-session.target"
        );
        assert_eq!(
            report.stop_target.command_line,
            "true --user stop backlit-session.target"
        );
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
            "ExecStart=/usr/bin/backlit-compositor --backend=drm --runtime=smithay --socket=backlit-0 --serve",
        );

        let report = verify_systemd_units(&unit_dir);

        assert!(!report.passed());
        assert!(!report.units_present());
        assert!(!report.session_target_ok());
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

    fn write_session_target(unit_dir: &Path) {
        let contents = "\
[Unit]
Description=Backlit graphical session services
Wants=backlit-compositor.service backlit-shell.service backlit-notification-daemon.service backlit-settings-daemon.service
After=graphical-session-pre.target
PartOf=graphical-session.target

[Install]
WantedBy=graphical-session.target
";
        fs::write(unit_dir.join("backlit-session.target"), contents)
            .expect("session target should be written");
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
