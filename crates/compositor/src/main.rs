use std::env;
use std::path::Path;
use std::process;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_compositor_backend::{
    parse_args, preflight_backend_with_environment, BackendPreflightEnvironment,
    BackendPreflightReport, HeadlessCompositor, RunConfig, SurfaceOptions,
};
use backlit_demo_client::{render_policy_gui, verify_policy_gui};
use backlit_surface::{SurfaceManager, SurfacePhase, SurfaceRole};
use backlit_window_policy::{OutputLayout, WindowPolicy, WindowState};

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-compositor: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = parse_args(env::args().skip(1)).map_err(|error| error.to_string())?;

    if config.help {
        print_help();
        return Ok(());
    }

    let started = Instant::now();
    emit(
        "compositor.start",
        &config,
        &[
            ("smoke_test", FieldValue::Bool(config.smoke_test)),
            ("scripted_client", FieldValue::Bool(config.scripted_client)),
        ],
    );

    let preflight_environment = BackendPreflightEnvironment::from_host();
    let preflight_report =
        preflight_backend_with_environment(config.backend, &preflight_environment);
    emit_backend_preflight(&config, &preflight_report, &preflight_environment);

    if !preflight_report.ready {
        return Err(format!(
            "{} compositor backend preflight failed: {}",
            preflight_report.backend.as_str(),
            preflight_report.code,
        ));
    }

    if config.scripted_client {
        let runtime = run_scripted_client_runtime(config.scripted_client_preview.as_deref())?;
        emit(
            "compositor.scripted_client",
            &config,
            &[
                ("passed", FieldValue::Bool(runtime.passed())),
                (
                    "client_connected",
                    FieldValue::Bool(runtime.client_connected),
                ),
                (
                    "surfaces_after_map",
                    FieldValue::U64(runtime.surfaces_after_map),
                ),
                (
                    "first_frame_damaged_surfaces",
                    FieldValue::U64(runtime.first_frame_damaged_surfaces),
                ),
                (
                    "idle_frame_damaged_surfaces",
                    FieldValue::U64(runtime.idle_frame_damaged_surfaces),
                ),
                (
                    "damage_frame_damaged_surfaces",
                    FieldValue::U64(runtime.damage_frame_damaged_surfaces),
                ),
                (
                    "post_damage_idle_surfaces",
                    FieldValue::U64(runtime.post_damage_idle_surfaces),
                ),
                (
                    "close_frame_damaged_surfaces",
                    FieldValue::U64(runtime.close_frame_damaged_surfaces),
                ),
                (
                    "disconnect_frame_damaged_surfaces",
                    FieldValue::U64(runtime.disconnect_frame_damaged_surfaces),
                ),
                (
                    "final_idle_damaged_surfaces",
                    FieldValue::U64(runtime.final_idle_damaged_surfaces),
                ),
                (
                    "surfaces_after_close",
                    FieldValue::U64(runtime.surfaces_after_close),
                ),
                (
                    "surfaces_after_disconnect",
                    FieldValue::U64(runtime.surfaces_after_disconnect),
                ),
                (
                    "clients_after_disconnect",
                    FieldValue::U64(runtime.clients_after_disconnect),
                ),
                ("frames", FieldValue::U64(runtime.frames)),
                (
                    "presented_pixels",
                    FieldValue::U64(runtime.presented_pixels),
                ),
                ("no_idle_redraw", FieldValue::Bool(runtime.no_idle_redraw)),
                (
                    "targeted_damage_ok",
                    FieldValue::Bool(runtime.targeted_damage_ok),
                ),
                ("close_damage_ok", FieldValue::Bool(runtime.close_damage_ok)),
                (
                    "disconnect_damage_ok",
                    FieldValue::Bool(runtime.disconnect_damage_ok),
                ),
                (
                    "clean_disconnect",
                    FieldValue::Bool(runtime.clean_disconnect),
                ),
                (
                    "policy_windows_after_map",
                    FieldValue::U64(runtime.policy_windows_after_map),
                ),
                (
                    "policy_visible_windows_after_map",
                    FieldValue::U64(runtime.policy_visible_windows_after_map),
                ),
                (
                    "policy_focused_after_map",
                    FieldValue::Bool(runtime.policy_focused_after_map),
                ),
                (
                    "policy_preview_requested",
                    FieldValue::Bool(runtime.policy_preview_requested),
                ),
                (
                    "policy_preview_written",
                    FieldValue::Bool(runtime.policy_preview_written),
                ),
                (
                    "policy_preview_verified",
                    FieldValue::Bool(runtime.policy_preview_verified),
                ),
                (
                    "policy_preview_non_background_pixels",
                    FieldValue::U64(runtime.policy_preview_non_background_pixels),
                ),
                (
                    "policy_preview_checksum",
                    FieldValue::U64(runtime.policy_preview_checksum),
                ),
            ],
        );

        if !runtime.passed() {
            return Err(String::from("scripted compositor client runtime failed"));
        }
    }

    if config.smoke_test {
        run_smoke_test(&config);
    } else {
        let readiness = run_service_ready();
        emit(
            "compositor.ready",
            &config,
            &[
                ("ready", FieldValue::Bool(readiness.passed())),
                (
                    "accepting_clients",
                    FieldValue::Bool(readiness.accepting_clients),
                ),
                (
                    "bootstrap_client_connected",
                    FieldValue::Bool(readiness.bootstrap_client_connected),
                ),
                (
                    "bootstrap_surface_presented",
                    FieldValue::Bool(readiness.bootstrap_surface_presented),
                ),
                ("clients", FieldValue::U64(readiness.clients)),
                ("surfaces", FieldValue::U64(readiness.surfaces)),
                ("frames", FieldValue::U64(readiness.frames)),
                (
                    "damaged_surfaces",
                    FieldValue::U64(readiness.damaged_surfaces),
                ),
                (
                    "presented_pixels",
                    FieldValue::U64(readiness.presented_pixels),
                ),
            ],
        );

        if !readiness.passed() {
            return Err(String::from("compositor service readiness failed"));
        }
    }

    if let Some(duration_ms) = config.idle_probe_ms {
        emit(
            "compositor.idle_probe_start",
            &config,
            &[("duration_ms", FieldValue::U64(duration_ms))],
        );
        thread::sleep(Duration::from_millis(duration_ms));
        emit(
            "compositor.idle_probe_complete",
            &config,
            &[("duration_ms", FieldValue::U64(duration_ms))],
        );
    }

    if config.serve {
        emit(
            "compositor.service_running",
            &config,
            &[
                ("bounded", FieldValue::Bool(config.serve_for_ms.is_some())),
                (
                    "serve_for_ms",
                    FieldValue::U64(config.serve_for_ms.unwrap_or(0)),
                ),
            ],
        );

        if let Some(duration_ms) = config.serve_for_ms {
            thread::sleep(Duration::from_millis(duration_ms));
            emit(
                "compositor.service_exit",
                &config,
                &[
                    ("bounded", FieldValue::Bool(true)),
                    ("serve_for_ms", FieldValue::U64(duration_ms)),
                ],
            );
        } else {
            loop {
                thread::sleep(Duration::from_secs(3600));
            }
        }
    }

    emit(
        "compositor.exit",
        &config,
        &[(
            "elapsed_ms",
            FieldValue::U64(started.elapsed().as_millis() as u64),
        )],
    );

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScriptedClientRuntime {
    client_connected: bool,
    surfaces_after_map: u64,
    first_frame_damaged_surfaces: u64,
    idle_frame_damaged_surfaces: u64,
    damage_frame_damaged_surfaces: u64,
    post_damage_idle_surfaces: u64,
    close_frame_damaged_surfaces: u64,
    disconnect_frame_damaged_surfaces: u64,
    final_idle_damaged_surfaces: u64,
    surfaces_after_close: u64,
    surfaces_after_disconnect: u64,
    clients_after_disconnect: u64,
    frames: u64,
    presented_pixels: u64,
    no_idle_redraw: bool,
    targeted_damage_ok: bool,
    close_damage_ok: bool,
    disconnect_damage_ok: bool,
    clean_disconnect: bool,
    policy_windows_after_map: u64,
    policy_visible_windows_after_map: u64,
    policy_focused_after_map: bool,
    policy_preview_requested: bool,
    policy_preview_written: bool,
    policy_preview_verified: bool,
    policy_preview_non_background_pixels: u64,
    policy_preview_checksum: u64,
}

impl ScriptedClientRuntime {
    fn passed(self) -> bool {
        self.client_connected
            && self.surfaces_after_map == 2
            && self.first_frame_damaged_surfaces == 2
            && self.idle_frame_damaged_surfaces == 0
            && self.damage_frame_damaged_surfaces == 1
            && self.post_damage_idle_surfaces == 0
            && self.close_frame_damaged_surfaces == 1
            && self.disconnect_frame_damaged_surfaces == 1
            && self.final_idle_damaged_surfaces == 0
            && self.surfaces_after_close == 1
            && self.surfaces_after_disconnect == 0
            && self.clients_after_disconnect == 0
            && self.frames == 7
            && self.presented_pixels == 800 * 600 + 1024 * 768
            && self.no_idle_redraw
            && self.targeted_damage_ok
            && self.close_damage_ok
            && self.disconnect_damage_ok
            && self.clean_disconnect
            && self.policy_windows_after_map == 2
            && self.policy_visible_windows_after_map == 2
            && self.policy_focused_after_map
            && (!self.policy_preview_requested || self.policy_preview_written)
            && self.policy_preview_verified
            && self.policy_preview_non_background_pixels > 10_000
    }
}

fn run_scripted_client_runtime(
    policy_preview_path: Option<&str>,
) -> Result<ScriptedClientRuntime, String> {
    let mut backend = HeadlessCompositor::default();
    let layout = OutputLayout::new(1400, 900, 42);
    let mut manager = SurfaceManager::new(layout);
    let client = backend.connect_client("scripted-terminal-client");
    let terminal = backend
        .submit_surface(client, "scripted-terminal", 800, 600)
        .map_err(|error| error.to_string())?;
    let terminal_policy_surface =
        map_scripted_toplevel(&mut manager, "scripted-terminal", 800, 600)?;
    let browser = backend
        .submit_surface(client, "scripted-browser", 1024, 768)
        .map_err(|error| error.to_string())?;
    let browser_policy_surface =
        map_scripted_toplevel(&mut manager, "scripted-browser", 1024, 768)?;
    let first_frame = backend.present();
    let idle_frame = backend.present();
    let browser_window = manager
        .surface(browser_policy_surface)
        .and_then(|surface| surface.window_id);
    let policy_windows_after_map = manager.policy().windows().len() as u64;
    let policy_visible_windows_after_map = manager.policy().visible_windows().count() as u64;
    let policy_focused_after_map =
        browser_window.is_some() && manager.policy().focused() == browser_window;
    let policy_preview = render_policy_gui(1400, 900, manager.policy(), layout);
    let policy_preview_report = verify_policy_gui(&policy_preview, manager.policy(), layout);
    let policy_preview_requested = policy_preview_path.is_some();
    let policy_preview_written = if let Some(path) = policy_preview_path {
        policy_preview
            .write_ppm(path)
            .map_err(|error| format!("failed to write scripted-client preview {path}: {error}"))?;
        Path::new(path).is_file()
    } else {
        true
    };
    let policy_preview_verified = policy_preview_report.passed()
        && manager
            .surface(terminal_policy_surface)
            .and_then(|surface| surface.window_id)
            .is_some();

    backend
        .mark_damaged(terminal)
        .map_err(|error| error.to_string())?;
    let damage_frame = backend.present();
    let post_damage_idle_frame = backend.present();

    backend
        .close_surface(browser)
        .map_err(|error| error.to_string())?;
    let close_frame = backend.present();

    backend
        .disconnect_client(client)
        .map_err(|error| error.to_string())?;
    let disconnect_frame = backend.present();
    let final_idle_frame = backend.present();

    let no_idle_redraw = idle_frame.damaged_surfaces == 0
        && post_damage_idle_frame.damaged_surfaces == 0
        && final_idle_frame.damaged_surfaces == 0;
    let targeted_damage_ok = damage_frame.damaged_surfaces == 1;
    let close_damage_ok = close_frame.damaged_surfaces == 1;
    let disconnect_damage_ok = disconnect_frame.damaged_surfaces == 1;
    let clean_disconnect =
        disconnect_frame.client_count == 0 && disconnect_frame.surface_count == 0;

    Ok(ScriptedClientRuntime {
        client_connected: first_frame.client_count == 1,
        surfaces_after_map: first_frame.surface_count,
        first_frame_damaged_surfaces: first_frame.damaged_surfaces,
        idle_frame_damaged_surfaces: idle_frame.damaged_surfaces,
        damage_frame_damaged_surfaces: damage_frame.damaged_surfaces,
        post_damage_idle_surfaces: post_damage_idle_frame.damaged_surfaces,
        close_frame_damaged_surfaces: close_frame.damaged_surfaces,
        disconnect_frame_damaged_surfaces: disconnect_frame.damaged_surfaces,
        final_idle_damaged_surfaces: final_idle_frame.damaged_surfaces,
        surfaces_after_close: close_frame.surface_count,
        surfaces_after_disconnect: disconnect_frame.surface_count,
        clients_after_disconnect: disconnect_frame.client_count,
        frames: final_idle_frame.frame,
        presented_pixels: first_frame.total_pixels,
        no_idle_redraw,
        targeted_damage_ok,
        close_damage_ok,
        disconnect_damage_ok,
        clean_disconnect,
        policy_windows_after_map,
        policy_visible_windows_after_map,
        policy_focused_after_map,
        policy_preview_requested,
        policy_preview_written,
        policy_preview_verified,
        policy_preview_non_background_pixels: policy_preview_report.non_background_pixels,
        policy_preview_checksum: policy_preview_report.checksum,
    })
}

fn map_scripted_toplevel(
    manager: &mut SurfaceManager,
    title: &str,
    width: i32,
    height: i32,
) -> Result<backlit_surface::SurfaceId, String> {
    let surface = manager.create_toplevel(title, (width, height));
    let configure = manager
        .send_initial_configure(surface)
        .ok_or_else(|| format!("failed to configure scripted surface {title}"))?;
    if configure.width != width || configure.height != height {
        return Err(format!(
            "scripted surface {title} configured as {}x{} instead of {width}x{height}",
            configure.width, configure.height
        ));
    }
    if !manager.ack_configure(surface, configure.serial) {
        return Err(format!("failed to ack scripted surface {title}"));
    }
    if !manager.commit(surface) {
        return Err(format!("failed to map scripted surface {title}"));
    }
    Ok(surface)
}

fn run_smoke_test(config: &RunConfig) {
    let mut policy = WindowPolicy::default();
    let first = policy.add_window("demo-terminal", (800, 600));
    let second = policy.add_window("demo-browser", (1200, 800));
    policy.focus(first);
    policy.cycle_focus_forward();

    let mut backend = HeadlessCompositor::default();
    let client = backend.connect_client("backlit-demo-client");
    let terminal = backend
        .submit_surface(client, "demo-terminal", 800, 600)
        .expect("smoke client should be registered");
    let _browser = backend
        .submit_surface(client, "demo-browser", 1200, 800)
        .expect("smoke client should be registered");
    let frame = backend.present();
    let idle_frame = backend.present();
    backend
        .mark_damaged(terminal)
        .expect("smoke surface should be registered");
    let damage_frame = backend.present();
    let post_damage_idle_frame = backend.present();
    let no_idle_redraw =
        idle_frame.damaged_surfaces == 0 && post_damage_idle_frame.damaged_surfaces == 0;
    let targeted_damage_ok = damage_frame.damaged_surfaces == 1;
    let scanout_smoke = run_direct_scanout_smoke();
    let surface_smoke = run_compositor_surface_smoke();

    emit(
        "compositor.smoke_test",
        config,
        &[
            ("windows", FieldValue::U64(policy.windows().len() as u64)),
            ("clients", FieldValue::U64(frame.client_count)),
            ("surfaces", FieldValue::U64(frame.surface_count)),
            ("damaged_surfaces", FieldValue::U64(frame.damaged_surfaces)),
            (
                "idle_damaged_surfaces",
                FieldValue::U64(idle_frame.damaged_surfaces),
            ),
            (
                "targeted_damage_surfaces",
                FieldValue::U64(damage_frame.damaged_surfaces),
            ),
            (
                "post_damage_idle_surfaces",
                FieldValue::U64(post_damage_idle_frame.damaged_surfaces),
            ),
            ("no_idle_redraw", FieldValue::Bool(no_idle_redraw)),
            ("targeted_damage_ok", FieldValue::Bool(targeted_damage_ok)),
            ("frames", FieldValue::U64(post_damage_idle_frame.frame)),
            (
                "direct_scanout_eligible",
                FieldValue::Bool(scanout_smoke.eligible),
            ),
            (
                "direct_scanout_dmabuf",
                FieldValue::Bool(scanout_smoke.dmabuf),
            ),
            (
                "direct_scanout_fullscreen",
                FieldValue::Bool(scanout_smoke.fullscreen),
            ),
            (
                "direct_scanout_overlay_blocked",
                FieldValue::Bool(scanout_smoke.overlay_blocked),
            ),
            (
                "direct_scanout_shm_blocked",
                FieldValue::Bool(scanout_smoke.shm_blocked),
            ),
            (
                "xdg_shell_registered",
                FieldValue::Bool(surface_smoke.xdg_shell_registered),
            ),
            (
                "xdg_surface_lifecycle",
                FieldValue::Bool(surface_smoke.passed()),
            ),
            (
                "xdg_toplevel_created",
                FieldValue::Bool(surface_smoke.created_toplevel),
            ),
            (
                "xdg_initial_configured",
                FieldValue::Bool(surface_smoke.initial_configured),
            ),
            (
                "xdg_ack_configure_ok",
                FieldValue::Bool(surface_smoke.ack_configure_ok),
            ),
            (
                "xdg_mapped_window",
                FieldValue::Bool(surface_smoke.mapped_window),
            ),
            (
                "xdg_backend_surface_presented",
                FieldValue::Bool(surface_smoke.backend_surface_presented),
            ),
            (
                "xdg_popup_created",
                FieldValue::Bool(surface_smoke.popup_created),
            ),
            (
                "xdg_popup_mapped",
                FieldValue::Bool(surface_smoke.popup_mapped),
            ),
            (
                "xdg_popup_backend_surface_presented",
                FieldValue::Bool(surface_smoke.popup_backend_surface_presented),
            ),
            (
                "xdg_popup_position_constrained",
                FieldValue::Bool(surface_smoke.popup_position_constrained),
            ),
            (
                "xdg_popup_did_not_create_window",
                FieldValue::Bool(surface_smoke.popup_did_not_create_window),
            ),
            (
                "xdg_popup_closed",
                FieldValue::Bool(surface_smoke.popup_closed),
            ),
            (
                "xdg_presented_surfaces",
                FieldValue::U64(surface_smoke.presented_surfaces),
            ),
            (
                "xdg_presented_pixels",
                FieldValue::U64(surface_smoke.presented_pixels),
            ),
            (
                "xdg_focused_after_map",
                FieldValue::Bool(surface_smoke.focused_after_map),
            ),
            (
                "xdg_maximize_uses_work_area",
                FieldValue::Bool(surface_smoke.maximize_uses_work_area),
            ),
            (
                "xdg_fullscreen_uses_output",
                FieldValue::Bool(surface_smoke.fullscreen_uses_output),
            ),
            (
                "xdg_close_requested",
                FieldValue::Bool(surface_smoke.close_requested),
            ),
            (
                "xdg_window_removed",
                FieldValue::Bool(surface_smoke.window_removed),
            ),
            (
                "xdg_windows_after_close",
                FieldValue::U64(surface_smoke.windows_after_close),
            ),
            ("total_surface_pixels", FieldValue::U64(frame.total_pixels)),
            ("first_window", FieldValue::U64(first.0)),
            ("focused_window", FieldValue::U64(second.0)),
        ],
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompositorReadyReport {
    accepting_clients: bool,
    bootstrap_client_connected: bool,
    bootstrap_surface_presented: bool,
    clients: u64,
    surfaces: u64,
    frames: u64,
    damaged_surfaces: u64,
    presented_pixels: u64,
}

impl CompositorReadyReport {
    fn passed(self) -> bool {
        self.accepting_clients
            && self.bootstrap_client_connected
            && self.bootstrap_surface_presented
            && self.clients == 1
            && self.surfaces == 1
            && self.frames == 1
            && self.damaged_surfaces == 1
            && self.presented_pixels == 1
    }
}

fn run_service_ready() -> CompositorReadyReport {
    let mut backend = HeadlessCompositor::default();
    let client = backend.connect_client("backlit-session-service");
    let bootstrap_surface_presented = backend
        .submit_surface(client, "backlit-bootstrap", 1, 1)
        .is_ok();
    let frame = backend.present();

    CompositorReadyReport {
        accepting_clients: !backend.clients().is_empty(),
        bootstrap_client_connected: backend.clients().len() == 1,
        bootstrap_surface_presented,
        clients: frame.client_count,
        surfaces: frame.surface_count,
        frames: frame.frame,
        damaged_surfaces: frame.damaged_surfaces,
        presented_pixels: frame.total_pixels,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompositorSurfaceSmoke {
    xdg_shell_registered: bool,
    created_toplevel: bool,
    initial_configured: bool,
    ack_configure_ok: bool,
    mapped_window: bool,
    backend_surface_presented: bool,
    popup_created: bool,
    popup_mapped: bool,
    popup_backend_surface_presented: bool,
    popup_position_constrained: bool,
    popup_did_not_create_window: bool,
    popup_closed: bool,
    presented_surfaces: u64,
    presented_pixels: u64,
    focused_after_map: bool,
    maximize_uses_work_area: bool,
    fullscreen_uses_output: bool,
    close_requested: bool,
    window_removed: bool,
    windows_after_close: u64,
}

impl CompositorSurfaceSmoke {
    fn passed(self) -> bool {
        self.xdg_shell_registered
            && self.created_toplevel
            && self.initial_configured
            && self.ack_configure_ok
            && self.mapped_window
            && self.backend_surface_presented
            && self.popup_created
            && self.popup_mapped
            && self.popup_backend_surface_presented
            && self.popup_position_constrained
            && self.popup_did_not_create_window
            && self.popup_closed
            && self.presented_surfaces == 2
            && self.presented_pixels == 640 * 480 + 240 * 160
            && self.focused_after_map
            && self.maximize_uses_work_area
            && self.fullscreen_uses_output
            && self.close_requested
            && self.window_removed
            && self.windows_after_close == 0
    }
}

fn run_compositor_surface_smoke() -> CompositorSurfaceSmoke {
    let mut manager = SurfaceManager::new(OutputLayout::new(800, 520, 42));
    let mut backend = HeadlessCompositor::default();
    let client = backend.connect_client("xdg-demo-client");
    let surface = manager.create_toplevel("xdg-terminal", (640, 480));
    let xdg_shell_registered = backlit_protocols::lookup_protocol("xdg_wm_base")
        .map(|protocol| protocol.mvp_required)
        .unwrap_or(false);
    let created_toplevel = manager
        .surface(surface)
        .map(|surface| {
            surface.role == SurfaceRole::XdgToplevel && surface.phase == SurfacePhase::Created
        })
        .unwrap_or(false);

    let initial_configure = manager.send_initial_configure(surface);
    let initial_configured = initial_configure
        .map(|configure| configure.width == 640 && configure.height == 480)
        .unwrap_or(false);
    let ack_configure_ok = initial_configure
        .map(|configure| manager.ack_configure(surface, configure.serial))
        .unwrap_or(false);
    let mapped_window = manager.commit(surface);
    let window_id = manager
        .surface(surface)
        .and_then(|surface| surface.window_id);
    let focused_after_map = window_id
        .map(|window_id| manager.policy().focused() == Some(window_id))
        .unwrap_or(false);

    let backend_surface_presented = if mapped_window {
        backend
            .submit_surface(client, "xdg-terminal", 640, 480)
            .map(|_| true)
            .unwrap_or(false)
    } else {
        false
    };
    let popup = manager.create_popup(surface, "xdg-terminal-menu", (240, 160), (32, 36));
    let popup_created = popup
        .and_then(|popup| manager.surface(popup))
        .map(|popup_surface| {
            popup_surface.role == SurfaceRole::XdgPopup
                && popup_surface.parent == Some(surface)
                && popup_surface.phase == SurfacePhase::Created
        })
        .unwrap_or(false);
    let popup_configure = popup.and_then(|popup| manager.send_initial_configure(popup));
    let popup_ack_configure_ok = match (popup, popup_configure) {
        (Some(popup), Some(configure)) => manager.ack_configure(popup, configure.serial),
        _ => false,
    };
    let popup_mapped = popup.map(|popup| manager.commit(popup)).unwrap_or(false);
    let popup_position_constrained = popup_configure
        .map(|configure| {
            configure.width == 240
                && configure.height == 160
                && configure.x >= manager.layout().output.x
                && configure.y >= manager.layout().output.y
                && configure.x + configure.width
                    <= manager.layout().output.x + manager.layout().output.width
                && configure.y + configure.height
                    <= manager.layout().output.y + manager.layout().output.height
        })
        .unwrap_or(false);
    let popup_did_not_create_window = manager.policy().windows().len() == 1;
    let popup_backend_surface_presented = if popup_mapped {
        backend
            .submit_surface(client, "xdg-terminal-menu", 240, 160)
            .map(|_| true)
            .unwrap_or(false)
    } else {
        false
    };
    let frame = backend.present();
    let popup_closed = popup
        .map(|popup| {
            manager.close(popup)
                && manager
                    .surface(popup)
                    .map(|surface| surface.phase == SurfacePhase::Closed)
                    .unwrap_or(false)
        })
        .unwrap_or(false);

    let maximize_uses_work_area = manager
        .request_maximize(surface)
        .and(window_id)
        .and_then(|window_id| manager.policy().window(window_id))
        .map(|window| {
            window.state == WindowState::Maximized
                && window.geometry == manager.layout().work_area()
        })
        .unwrap_or(false);
    let fullscreen_uses_output = manager
        .request_fullscreen(surface)
        .and(window_id)
        .and_then(|window_id| manager.policy().window(window_id))
        .map(|window| {
            window.state == WindowState::Fullscreen && window.geometry == manager.layout().output
        })
        .unwrap_or(false);
    let close_requested = manager
        .request_close(surface)
        .map(|configure| configure.close_requested)
        .unwrap_or(false);
    let close_ok = manager.close(surface);
    let window_removed = close_ok
        && window_id
            .map(|window_id| manager.policy().window(window_id).is_none())
            .unwrap_or(false)
        && manager
            .surface(surface)
            .map(|surface| surface.phase == SurfacePhase::Closed)
            .unwrap_or(false);

    CompositorSurfaceSmoke {
        xdg_shell_registered,
        created_toplevel,
        initial_configured,
        ack_configure_ok,
        mapped_window,
        backend_surface_presented,
        popup_created,
        popup_mapped: popup_mapped && popup_ack_configure_ok,
        popup_backend_surface_presented,
        popup_position_constrained,
        popup_did_not_create_window,
        popup_closed,
        presented_surfaces: frame.surface_count,
        presented_pixels: frame.total_pixels,
        focused_after_map,
        maximize_uses_work_area,
        fullscreen_uses_output,
        close_requested,
        window_removed,
        windows_after_close: manager.policy().windows().len() as u64,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirectScanoutSmoke {
    eligible: bool,
    dmabuf: bool,
    fullscreen: bool,
    overlay_blocked: bool,
    shm_blocked: bool,
}

fn run_direct_scanout_smoke() -> DirectScanoutSmoke {
    let mut compositor = HeadlessCompositor::default();
    let client = compositor.connect_client("scanout-video-client");
    let video = compositor
        .submit_surface_with_options(
            client,
            "fullscreen-video",
            1920,
            1080,
            SurfaceOptions::dmabuf_fullscreen(),
        )
        .expect("scanout client should be registered");
    let eligible = compositor
        .direct_scanout_candidate(video, 1920, 1080)
        .expect("scanout surface should exist");

    compositor
        .submit_surface(client, "panel-overlay", 1920, 42)
        .expect("scanout client should be registered");
    let overlay_blocked = compositor
        .direct_scanout_candidate(video, 1920, 1080)
        .map(|report| !report.eligible && report.reason == "occluded-by-other-surface")
        .unwrap_or(false);

    let mut shm_compositor = HeadlessCompositor::default();
    let client = shm_compositor.connect_client("scanout-shm-client");
    let shm_video = shm_compositor
        .submit_surface_with_options(
            client,
            "fullscreen-shm-video",
            1920,
            1080,
            SurfaceOptions {
                fullscreen: true,
                ..SurfaceOptions::default()
            },
        )
        .expect("scanout client should be registered");
    let shm_blocked = shm_compositor
        .direct_scanout_candidate(shm_video, 1920, 1080)
        .map(|report| !report.eligible && report.reason == "not-dmabuf")
        .unwrap_or(false);

    DirectScanoutSmoke {
        eligible: eligible.eligible,
        dmabuf: eligible.buffer_kind.as_str() == "dmabuf",
        fullscreen: eligible.reason == "eligible",
        overlay_blocked,
        shm_blocked,
    }
}

fn emit(event: &str, config: &RunConfig, fields: &[(&str, FieldValue<'_>)]) {
    let mut combined = Vec::with_capacity(fields.len() + 2);
    combined.push(("backend", FieldValue::Str(config.backend.as_str())));
    combined.push(("socket", FieldValue::Str(config.socket.as_str())));
    combined.extend_from_slice(fields);
    println!("{}", event_json(event, &combined));
}

fn emit_backend_preflight(
    config: &RunConfig,
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
        "compositor.backend_preflight",
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

fn print_help() {
    println!(
        "\
backlit-compositor

Usage:
  backlit-compositor [--backend=headless|wayland|drm] [--socket=backlit-0] [--smoke-test] [--scripted-client] [--scripted-client-preview=path] [--serve] [--serve-for-ms=1000] [--idle-probe-ms=1000]

Flags:
  --backend      Select compositor backend. Defaults to headless.
  --socket       Wayland socket name to create or target. Defaults to backlit-0.
  --smoke-test   Run the current MVP 0 policy/metrics smoke test and exit.
  --scripted-client
                 Run a deterministic app-client lifecycle through the compositor runtime.
  --scripted-client-preview
                 Write the scripted client policy preview frame to a PPM file.
  --serve        Stay alive after readiness for systemd session service mode.
  --serve-for-ms Stay alive for a bounded service-lifecycle probe duration.
  --idle-probe-ms
                 Stay alive without doing work for bounded resource sampling.
  --help         Show this help text.

Backend launch preflight runs before smoke or service readiness events.
"
    );
}

#[cfg(test)]
mod tests {
    use super::{run_compositor_surface_smoke, run_scripted_client_runtime};

    #[test]
    fn compositor_surface_smoke_maps_xdg_toplevel_into_backend_frame() {
        let report = run_compositor_surface_smoke();

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.presented_surfaces, 2);
        assert_eq!(report.presented_pixels, 640 * 480 + 240 * 160);
        assert_eq!(report.windows_after_close, 0);
    }

    #[test]
    fn compositor_service_ready_accepts_client_and_presents_bootstrap_surface() {
        let report = super::run_service_ready();

        assert!(report.passed(), "{report:?}");
        assert!(report.accepting_clients);
        assert_eq!(report.clients, 1);
        assert_eq!(report.surfaces, 1);
        assert_eq!(report.presented_pixels, 1);
    }

    #[test]
    fn scripted_client_runtime_maps_damages_and_disconnects() {
        let report = run_scripted_client_runtime(None).unwrap();

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.surfaces_after_map, 2);
        assert_eq!(report.surfaces_after_disconnect, 0);
        assert_eq!(report.clients_after_disconnect, 0);
        assert_eq!(report.policy_windows_after_map, 2);
        assert!(report.policy_preview_verified);
    }
}
