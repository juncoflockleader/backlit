use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_compositor_backend::{
    backend_launch_plan, preflight_backend_with_environment, smithay_runtime_bootstrap,
    smithay_runtime_probe, BackendKind, BackendLaunchPlan, BackendPreflightEnvironment,
    SmithayRuntimeBootstrap, SmithayRuntimeProbe,
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
    let launch_plan = backend_launch_plan(config.backend, &report, &environment);
    let wayland_display = environment.wayland_display.as_deref().unwrap_or("");
    let xdg_runtime_dir = environment.xdg_runtime_dir.as_deref().unwrap_or("");
    let session_id = environment.session_id.as_deref().unwrap_or("");
    let seat = environment.seat.as_deref().unwrap_or("");
    let session_type = environment.session_type.as_deref().unwrap_or("");
    let session_state = environment.session_state.as_deref().unwrap_or("");

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
                    FieldValue::Bool(environment.session_active)
                ),
                (
                    "session_remote",
                    FieldValue::Bool(environment.session_remote)
                ),
            ],
        )
    );
    emit_launch_plan(&launch_plan);

    if config.verify_smithay_runtime {
        let probe = smithay_runtime_probe(&environment);
        let bootstrap = smithay_runtime_bootstrap();
        emit_smithay_runtime_probe(&probe);
        emit_smithay_runtime_bootstrap(&bootstrap);
        if config.verify && report.ready && (!probe.passed() || !bootstrap.passed()) {
            return Err(String::from(
                "DRM backend preflight is ready but Smithay runtime probe/bootstrap did not pass",
            ));
        }
    }

    if config.verify && !report.ready {
        return Err(format!(
            "{} backend preflight failed: {}",
            report.backend.as_str(),
            report.code,
        ));
    }

    Ok(())
}

fn emit_smithay_runtime_bootstrap(bootstrap: &SmithayRuntimeBootstrap) {
    println!(
        "{}",
        event_json(
            "backend.smithay_runtime_bootstrap",
            &[
                (
                    "feature_enabled",
                    FieldValue::Bool(bootstrap.feature_enabled),
                ),
                ("compiled", FieldValue::Bool(bootstrap.compiled)),
                ("passed", FieldValue::Bool(bootstrap.passed())),
                (
                    "runtime_backend",
                    FieldValue::Str(bootstrap.runtime_backend),
                ),
                (
                    "display_created",
                    FieldValue::Bool(bootstrap.display_created),
                ),
                (
                    "display_handle_created",
                    FieldValue::Bool(bootstrap.display_handle_created),
                ),
                (
                    "listening_socket_bound",
                    FieldValue::Bool(bootstrap.listening_socket_bound),
                ),
                (
                    "socket_name",
                    FieldValue::Str(bootstrap.socket_name.as_str())
                ),
                (
                    "socket_connect_succeeded",
                    FieldValue::Bool(bootstrap.socket_connect_succeeded),
                ),
                (
                    "socket_accept_succeeded",
                    FieldValue::Bool(bootstrap.socket_accept_succeeded),
                ),
                (
                    "client_inserted",
                    FieldValue::Bool(bootstrap.client_inserted),
                ),
                (
                    "display_clients_dispatched",
                    FieldValue::Bool(bootstrap.display_clients_dispatched),
                ),
                (
                    "display_dispatch_count",
                    FieldValue::U64(bootstrap.display_dispatch_count),
                ),
                (
                    "display_clients_flushed",
                    FieldValue::Bool(bootstrap.display_clients_flushed),
                ),
                (
                    "event_loop_created",
                    FieldValue::Bool(bootstrap.event_loop_created),
                ),
                (
                    "event_loop_dispatched",
                    FieldValue::Bool(bootstrap.event_loop_dispatched),
                ),
                ("failure", FieldValue::Str(bootstrap.failure.as_str())),
            ],
        )
    );
}

fn emit_smithay_runtime_probe(probe: &SmithayRuntimeProbe) {
    let primary_drm_card = probe.primary_drm_card.as_deref().unwrap_or("");
    let primary_drm_render_node = probe.primary_drm_render_node.as_deref().unwrap_or("");
    let drm_node_primary_path = probe.drm_node_primary_path.as_deref().unwrap_or("");
    let drm_node_render_path = probe.drm_node_render_path.as_deref().unwrap_or("");
    let renderer_node_path = probe.renderer_node_path.as_deref().unwrap_or("");
    let libseat_session_seat = probe.libseat_session_seat.as_deref().unwrap_or("");
    let input_runtime_failure = probe.input_runtime_failure.as_deref().unwrap_or("");
    let kms_resource_failure = probe.kms_resource_failure.as_deref().unwrap_or("");
    let kms_surface_failure = probe.kms_surface_failure.as_deref().unwrap_or("");
    let kms_framebuffer_failure = probe.kms_framebuffer_failure.as_deref().unwrap_or("");
    let kms_first_present_failure = probe.kms_first_present_failure.as_deref().unwrap_or("");
    let kms_scanout_connector_name = probe.kms_scanout_connector_name.as_deref().unwrap_or("");
    let renderer_runtime_failure = probe.renderer_runtime_failure.as_deref().unwrap_or("");
    let primary_input_event = probe.primary_input_event.as_deref().unwrap_or("");
    let components = probe.components.join(",");

    println!(
        "{}",
        event_json(
            "backend.smithay_runtime_probe",
            &[
                ("backend", FieldValue::Str(probe.backend.as_str())),
                ("feature_enabled", FieldValue::Bool(probe.feature_enabled),),
                ("compiled", FieldValue::Bool(probe.compiled)),
                ("launch_ready", FieldValue::Bool(probe.launch_ready)),
                ("passed", FieldValue::Bool(probe.passed())),
                ("target_os", FieldValue::Str(probe.target_os.as_str())),
                ("runtime_backend", FieldValue::Str(probe.runtime_backend),),
                ("display_driver", FieldValue::Str(probe.display_driver)),
                ("input_driver", FieldValue::Str(probe.input_driver)),
                ("session_driver", FieldValue::Str(probe.session_driver)),
                ("event_loop", FieldValue::Str(probe.event_loop)),
                (
                    "drm_card_selected",
                    FieldValue::Bool(probe.drm_card_selected),
                ),
                (
                    "drm_render_selected",
                    FieldValue::Bool(probe.drm_render_selected),
                ),
                (
                    "drm_node_resolved",
                    FieldValue::Bool(probe.drm_node_resolved),
                ),
                ("drm_node_type", FieldValue::Str(probe.drm_node_type)),
                (
                    "drm_node_primary_path",
                    FieldValue::Str(drm_node_primary_path),
                ),
                (
                    "drm_node_render_path",
                    FieldValue::Str(drm_node_render_path),
                ),
                ("kms_card_opened", FieldValue::Bool(probe.kms_card_opened)),
                (
                    "kms_device_created",
                    FieldValue::Bool(probe.kms_device_created),
                ),
                (
                    "kms_event_source_inserted",
                    FieldValue::Bool(probe.kms_event_source_inserted),
                ),
                (
                    "kms_event_loop_dispatched",
                    FieldValue::Bool(probe.kms_event_loop_dispatched),
                ),
                (
                    "kms_atomic_modesetting",
                    FieldValue::Bool(probe.kms_atomic_modesetting),
                ),
                ("kms_crtc_count", FieldValue::U64(probe.kms_crtc_count)),
                (
                    "kms_connector_count",
                    FieldValue::U64(probe.kms_connector_count),
                ),
                (
                    "kms_connected_connector_count",
                    FieldValue::U64(probe.kms_connected_connector_count),
                ),
                ("kms_mode_count", FieldValue::U64(probe.kms_mode_count)),
                (
                    "kms_primary_plane_count",
                    FieldValue::U64(probe.kms_primary_plane_count),
                ),
                (
                    "kms_cursor_plane_count",
                    FieldValue::U64(probe.kms_cursor_plane_count),
                ),
                (
                    "kms_overlay_plane_count",
                    FieldValue::U64(probe.kms_overlay_plane_count),
                ),
                (
                    "kms_scanout_plan_ready",
                    FieldValue::Bool(probe.kms_scanout_plan_ready),
                ),
                (
                    "kms_scanout_connector_id",
                    FieldValue::U64(probe.kms_scanout_connector_id),
                ),
                (
                    "kms_scanout_connector_name",
                    FieldValue::Str(kms_scanout_connector_name),
                ),
                (
                    "kms_scanout_crtc_id",
                    FieldValue::U64(probe.kms_scanout_crtc_id),
                ),
                (
                    "kms_scanout_primary_plane_id",
                    FieldValue::U64(probe.kms_scanout_primary_plane_id),
                ),
                (
                    "kms_scanout_mode_width",
                    FieldValue::U64(probe.kms_scanout_mode_width),
                ),
                (
                    "kms_scanout_mode_height",
                    FieldValue::U64(probe.kms_scanout_mode_height),
                ),
                (
                    "kms_scanout_mode_refresh_hz",
                    FieldValue::U64(probe.kms_scanout_mode_refresh_hz),
                ),
                (
                    "kms_scanout_mode_preferred",
                    FieldValue::Bool(probe.kms_scanout_mode_preferred),
                ),
                (
                    "kms_surface_created",
                    FieldValue::Bool(probe.kms_surface_created),
                ),
                (
                    "kms_surface_legacy",
                    FieldValue::Bool(probe.kms_surface_legacy),
                ),
                (
                    "kms_surface_crtc_matches_plan",
                    FieldValue::Bool(probe.kms_surface_crtc_matches_plan),
                ),
                (
                    "kms_surface_primary_plane_matches_plan",
                    FieldValue::Bool(probe.kms_surface_primary_plane_matches_plan),
                ),
                (
                    "kms_surface_pending_connector_count",
                    FieldValue::U64(probe.kms_surface_pending_connector_count),
                ),
                (
                    "kms_surface_current_connector_count",
                    FieldValue::U64(probe.kms_surface_current_connector_count),
                ),
                (
                    "kms_surface_pending_mode_matches_plan",
                    FieldValue::Bool(probe.kms_surface_pending_mode_matches_plan),
                ),
                (
                    "kms_surface_commit_pending",
                    FieldValue::Bool(probe.kms_surface_commit_pending),
                ),
                (
                    "kms_surface_dropped_after_pause",
                    FieldValue::Bool(probe.kms_surface_dropped_after_pause),
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
                    "kms_framebuffer_test_state_succeeded",
                    FieldValue::Bool(probe.kms_framebuffer_test_state_succeeded),
                ),
                (
                    "kms_framebuffer_test_state_permission_denied",
                    FieldValue::Bool(probe.kms_framebuffer_test_state_permission_denied),
                ),
                (
                    "kms_framebuffer_test_allow_modeset",
                    FieldValue::Bool(probe.kms_framebuffer_test_allow_modeset),
                ),
                (
                    "kms_framebuffer_primary_plane_matches_surface",
                    FieldValue::Bool(probe.kms_framebuffer_primary_plane_matches_surface),
                ),
                (
                    "kms_framebuffer_width",
                    FieldValue::U64(probe.kms_framebuffer_width),
                ),
                (
                    "kms_framebuffer_height",
                    FieldValue::U64(probe.kms_framebuffer_height),
                ),
                (
                    "kms_framebuffer_released_before_surface_drop",
                    FieldValue::Bool(probe.kms_framebuffer_released_before_surface_drop),
                ),
                (
                    "kms_framebuffer_failure",
                    FieldValue::Str(kms_framebuffer_failure),
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
                    "kms_first_present_failure",
                    FieldValue::Str(kms_first_present_failure),
                ),
                ("kms_surface_failure", FieldValue::Str(kms_surface_failure),),
                (
                    "kms_resource_failure",
                    FieldValue::Str(kms_resource_failure),
                ),
                (
                    "renderer_node_selected",
                    FieldValue::Bool(probe.renderer_node_selected),
                ),
                ("renderer_node_path", FieldValue::Str(renderer_node_path),),
                (
                    "input_event_selected",
                    FieldValue::Bool(probe.input_event_selected),
                ),
                ("uses_logind", FieldValue::Bool(probe.uses_logind)),
                ("uses_libseat", FieldValue::Bool(probe.uses_libseat)),
                ("uses_libinput", FieldValue::Bool(probe.uses_libinput)),
                (
                    "gbm_allocator_component",
                    FieldValue::Bool(probe.gbm_allocator_component),
                ),
                (
                    "egl_display_component",
                    FieldValue::Bool(probe.egl_display_component),
                ),
                (
                    "gles_renderer_component",
                    FieldValue::Bool(probe.gles_renderer_component),
                ),
                (
                    "renderer_node_opened",
                    FieldValue::Bool(probe.renderer_node_opened),
                ),
                (
                    "gbm_device_created",
                    FieldValue::Bool(probe.gbm_device_created),
                ),
                (
                    "gbm_allocator_created",
                    FieldValue::Bool(probe.gbm_allocator_created),
                ),
                (
                    "egl_display_created",
                    FieldValue::Bool(probe.egl_display_created),
                ),
                (
                    "egl_context_created",
                    FieldValue::Bool(probe.egl_context_created),
                ),
                (
                    "gles_renderer_created",
                    FieldValue::Bool(probe.gles_renderer_created),
                ),
                (
                    "offscreen_buffer_created",
                    FieldValue::Bool(probe.offscreen_buffer_created),
                ),
                (
                    "offscreen_frame_rendered",
                    FieldValue::Bool(probe.offscreen_frame_rendered),
                ),
                (
                    "offscreen_frame_copied",
                    FieldValue::Bool(probe.offscreen_frame_copied),
                ),
                (
                    "offscreen_pixel_verified",
                    FieldValue::Bool(probe.offscreen_pixel_verified),
                ),
                (
                    "offscreen_render_width",
                    FieldValue::U64(probe.offscreen_render_width),
                ),
                (
                    "offscreen_render_height",
                    FieldValue::U64(probe.offscreen_render_height),
                ),
                (
                    "offscreen_render_pixels",
                    FieldValue::U64(probe.offscreen_render_pixels),
                ),
                (
                    "offscreen_sample_red",
                    FieldValue::U64(probe.offscreen_sample_red),
                ),
                (
                    "offscreen_sample_green",
                    FieldValue::U64(probe.offscreen_sample_green),
                ),
                (
                    "offscreen_sample_blue",
                    FieldValue::U64(probe.offscreen_sample_blue),
                ),
                (
                    "offscreen_sample_alpha",
                    FieldValue::U64(probe.offscreen_sample_alpha),
                ),
                (
                    "renderer_runtime_failure",
                    FieldValue::Str(renderer_runtime_failure),
                ),
                (
                    "libseat_session_created",
                    FieldValue::Bool(probe.libseat_session_created),
                ),
                (
                    "libseat_session_active",
                    FieldValue::Bool(probe.libseat_session_active),
                ),
                (
                    "libseat_session_seat",
                    FieldValue::Str(libseat_session_seat),
                ),
                (
                    "libseat_event_source_inserted",
                    FieldValue::Bool(probe.libseat_event_source_inserted),
                ),
                (
                    "libseat_event_loop_dispatched",
                    FieldValue::Bool(probe.libseat_event_loop_dispatched),
                ),
                (
                    "libseat_session_event_count",
                    FieldValue::U64(probe.libseat_session_event_count),
                ),
                (
                    "libinput_context_created",
                    FieldValue::Bool(probe.libinput_context_created),
                ),
                (
                    "libinput_seat_assigned",
                    FieldValue::Bool(probe.libinput_seat_assigned),
                ),
                (
                    "libinput_backend_created",
                    FieldValue::Bool(probe.libinput_backend_created),
                ),
                (
                    "libinput_event_source_inserted",
                    FieldValue::Bool(probe.libinput_event_source_inserted),
                ),
                (
                    "libinput_event_loop_dispatched",
                    FieldValue::Bool(probe.libinput_event_loop_dispatched),
                ),
                (
                    "libinput_event_count",
                    FieldValue::U64(probe.libinput_event_count),
                ),
                (
                    "input_runtime_failure",
                    FieldValue::Str(input_runtime_failure),
                ),
                ("primary_drm_card", FieldValue::Str(primary_drm_card)),
                (
                    "primary_drm_render_node",
                    FieldValue::Str(primary_drm_render_node),
                ),
                ("primary_input_event", FieldValue::Str(primary_input_event)),
                (
                    "component_count",
                    FieldValue::U64(probe.components.len() as u64),
                ),
                ("components", FieldValue::Str(components.as_str())),
            ],
        )
    );
}

fn emit_launch_plan(plan: &BackendLaunchPlan) {
    let primary_drm_card = plan.primary_drm_card.as_deref().unwrap_or("");
    let primary_drm_render_node = plan.primary_drm_render_node.as_deref().unwrap_or("");
    let primary_input_event = plan.primary_input_event.as_deref().unwrap_or("");
    let session_id = plan.session_id.as_deref().unwrap_or("");
    let seat = plan.seat.as_deref().unwrap_or("");
    let session_type = plan.session_type.as_deref().unwrap_or("");

    println!(
        "{}",
        event_json(
            "backend.launch_plan",
            &[
                ("backend", FieldValue::Str(plan.backend.as_str())),
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
        )
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Config {
    backend: BackendKind,
    verify: bool,
    verify_smithay_runtime: bool,
    help: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend: BackendKind::Headless,
            verify: false,
            verify_smithay_runtime: false,
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
            } else if arg == "--verify-smithay-runtime" {
                config.verify_smithay_runtime = true;
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
  backlit-compositor-backend [--backend=headless|wayland|drm] [--verify] [--verify-smithay-runtime]

Flags:
  --backend  Backend to preflight. Defaults to headless.
  --verify   Exit non-zero when the requested backend is not ready.
  --verify-smithay-runtime
             Emit a Smithay DRM/libinput/libseat/calloop runtime probe event.

The JSON event includes runtime, DRM, input, and session hints used by
launch-readiness verification.
"
    );
}
