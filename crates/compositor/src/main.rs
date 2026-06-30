use std::env;
use std::process;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_compositor_backend::{
    parse_args, preflight_backend_with_environment, BackendPreflightEnvironment,
    BackendPreflightReport, HeadlessCompositor, RunConfig, SurfaceOptions,
};
use backlit_window_policy::WindowPolicy;

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
        &[("smoke_test", FieldValue::Bool(config.smoke_test))],
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

    if config.smoke_test {
        run_smoke_test(&config);
    } else {
        emit(
            "compositor.stub_ready",
            &config,
            &[("accepting_clients", FieldValue::Bool(false))],
        );
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
            ("total_surface_pixels", FieldValue::U64(frame.total_pixels)),
            ("first_window", FieldValue::U64(first.0)),
            ("focused_window", FieldValue::U64(second.0)),
        ],
    );
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
            ("session_id", FieldValue::Str(session_id)),
            ("seat", FieldValue::Str(seat)),
            ("session_type", FieldValue::Str(session_type)),
        ],
    );
}

fn print_help() {
    println!(
        "\
backlit-compositor

Usage:
  backlit-compositor [--backend=headless|wayland|drm] [--socket=backlit-0] [--smoke-test] [--idle-probe-ms=1000]

Flags:
  --backend      Select compositor backend. Defaults to headless.
  --socket       Wayland socket name to create or target. Defaults to backlit-0.
  --smoke-test   Run the current MVP 0 policy/metrics smoke test and exit.
  --idle-probe-ms
                 Stay alive without doing work for bounded resource sampling.
  --help         Show this help text.

Backend launch preflight runs before smoke or service readiness events.
"
    );
}
