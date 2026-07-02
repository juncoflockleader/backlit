use std::env;
use std::fs;
use std::io::ErrorKind;
use std::io::Read;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_compositor_backend::{
    backend_launch_plan, parse_args, preflight_backend_with_environment, smithay_runtime_probe,
    BackendKind, BackendLaunchPlan, BackendPreflightEnvironment, BackendPreflightReport, ClientId,
    CompositorRuntime, HeadlessCompositor, InputEventCounters, RunConfig, RuntimeKind,
    SmithayRuntimeProbe, SurfaceId as BackendSurfaceId, SurfaceOptions,
};
#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
use backlit_compositor_backend::{
    RealShmPixel, SmithayCompositorRuntime, SmithayRealShmFrameCapture,
};
use backlit_demo_client::{render_policy_gui, verify_policy_gui};
#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
use backlit_demo_client::{Canvas, Color};
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
            ("runtime", FieldValue::Str(config.runtime.as_str())),
            ("smoke_test", FieldValue::Bool(config.smoke_test)),
            ("scripted_client", FieldValue::Bool(config.scripted_client)),
            (
                "smithay_client_smoke",
                FieldValue::Bool(config.smithay_client_smoke),
            ),
            (
                "smithay_real_shm_frame",
                FieldValue::Bool(config.smithay_real_shm_frame),
            ),
            (
                "drm_first_present_probe",
                FieldValue::Bool(config.drm_first_present_probe),
            ),
        ],
    );

    let preflight_environment = BackendPreflightEnvironment::from_host();
    let preflight_report =
        preflight_backend_with_environment(config.backend, &preflight_environment);
    emit_backend_preflight(&config, &preflight_report, &preflight_environment);
    let launch_plan =
        backend_launch_plan_for_config(&config, &preflight_report, &preflight_environment);
    emit_backend_launch_plan(&config, &launch_plan);

    if !preflight_report.ready {
        return Err(format!(
            "{} compositor backend preflight failed: {}",
            preflight_report.backend.as_str(),
            preflight_report.code,
        ));
    }

    if config.drm_first_present_probe {
        if config.backend != BackendKind::Drm || config.runtime != RuntimeKind::Smithay {
            return Err(String::from(
                "DRM first-present probe requires --backend=drm --runtime=smithay",
            ));
        }

        let probe = smithay_runtime_probe(&preflight_environment);
        emit_drm_first_present_probe(&config, &probe);

        if !probe.passed() {
            return Err(String::from(
                "DRM first-present probe did not reach a valid commit or boundary",
            ));
        }

        if !config.scripted_client
            && !config.smithay_client_smoke
            && !config.smithay_real_shm_frame
            && !config.smoke_test
            && !config.serve
            && config.idle_probe_ms.is_none()
        {
            emit(
                "compositor.exit",
                &config,
                &[(
                    "elapsed_ms",
                    FieldValue::U64(started.elapsed().as_millis() as u64),
                )],
            );
            return Ok(());
        }
    }

    if config.smithay_client_smoke {
        let smoke = run_smithay_client_smoke_for_config(&config)?;
        emit(
            "compositor.smithay_client_smoke",
            &config,
            &[
                ("passed", FieldValue::Bool(smoke.passed())),
                ("runtime_backend", FieldValue::Str(smoke.runtime_backend)),
                (
                    "smithay_protocol_globals",
                    FieldValue::U64(smoke.smithay_protocol_globals),
                ),
                (
                    "registry_global_count",
                    FieldValue::U64(smoke.registry_global_count),
                ),
                (
                    "registry_announced",
                    FieldValue::Bool(smoke.registry_announced),
                ),
                (
                    "mvp_protocol_globals",
                    FieldValue::U64(smoke.mvp_protocol_globals),
                ),
                (
                    "mvp_protocol_globals_announced",
                    FieldValue::Bool(smoke.mvp_protocol_globals_announced),
                ),
                ("wl_output_bound", FieldValue::Bool(smoke.wl_output_bound)),
                (
                    "xdg_output_manager_bound",
                    FieldValue::Bool(smoke.xdg_output_manager_bound),
                ),
                ("viewporter_bound", FieldValue::Bool(smoke.viewporter_bound)),
                (
                    "presentation_bound",
                    FieldValue::Bool(smoke.presentation_bound),
                ),
                (
                    "linux_dmabuf_bound",
                    FieldValue::Bool(smoke.linux_dmabuf_bound),
                ),
                (
                    "linux_dmabuf_version",
                    FieldValue::U64(smoke.linux_dmabuf_version),
                ),
                (
                    "linux_dmabuf_version_at_least_4",
                    FieldValue::Bool(smoke.linux_dmabuf_version_at_least_4),
                ),
                (
                    "seat_global_announced",
                    FieldValue::Bool(smoke.seat_global_announced),
                ),
                ("seat_bound", FieldValue::Bool(smoke.seat_bound)),
                (
                    "seat_name_observed",
                    FieldValue::Bool(smoke.seat_name_observed),
                ),
                (
                    "seat_keyboard_capability",
                    FieldValue::Bool(smoke.seat_keyboard_capability),
                ),
                (
                    "seat_pointer_capability",
                    FieldValue::Bool(smoke.seat_pointer_capability),
                ),
                ("keyboard_bound", FieldValue::Bool(smoke.keyboard_bound)),
                ("pointer_bound", FieldValue::Bool(smoke.pointer_bound)),
                ("compositor_bound", FieldValue::Bool(smoke.compositor_bound)),
                ("shm_bound", FieldValue::Bool(smoke.shm_bound)),
                (
                    "shm_buffer_created",
                    FieldValue::Bool(smoke.shm_buffer_created),
                ),
                (
                    "shm_buffer_attached",
                    FieldValue::Bool(smoke.shm_buffer_attached),
                ),
                (
                    "xdg_wm_base_bound",
                    FieldValue::Bool(smoke.xdg_wm_base_bound),
                ),
                ("surface_created", FieldValue::Bool(smoke.surface_created)),
                (
                    "xdg_toplevel_created",
                    FieldValue::Bool(smoke.xdg_toplevel_created),
                ),
                (
                    "configure_received",
                    FieldValue::Bool(smoke.configure_received),
                ),
                ("configure_acked", FieldValue::Bool(smoke.configure_acked)),
                (
                    "surface_committed",
                    FieldValue::Bool(smoke.surface_committed),
                ),
                (
                    "inserted_wayland_clients",
                    FieldValue::U64(smoke.inserted_wayland_clients),
                ),
                (
                    "wayland_dispatch_count",
                    FieldValue::U64(smoke.wayland_dispatch_count),
                ),
                (
                    "calloop_dispatch_count",
                    FieldValue::U64(smoke.calloop_dispatch_count),
                ),
                (
                    "input_sources_ready",
                    FieldValue::Bool(smoke.input_sources_ready),
                ),
                (
                    "input_source_count",
                    FieldValue::U64(smoke.input_source_count),
                ),
                (
                    "input_event_loop_dispatch_count",
                    FieldValue::U64(smoke.input_event_loop_dispatch_count),
                ),
                ("input_seat_ready", FieldValue::Bool(smoke.input_seat_ready)),
                (
                    "input_keyboard_handle_ready",
                    FieldValue::Bool(smoke.input_keyboard_handle_ready),
                ),
                (
                    "input_pointer_handle_ready",
                    FieldValue::Bool(smoke.input_pointer_handle_ready),
                ),
                (
                    "input_seat_dispatch_count",
                    FieldValue::U64(smoke.input_seat_dispatch_count),
                ),
                (
                    "input_keyboard_dispatch_count",
                    FieldValue::U64(smoke.input_keyboard_dispatch_count),
                ),
                (
                    "input_pointer_dispatch_count",
                    FieldValue::U64(smoke.input_pointer_dispatch_count),
                ),
                (
                    "input_event_count",
                    FieldValue::U64(smoke.input_event_counters.total),
                ),
                (
                    "input_device_event_count",
                    FieldValue::U64(smoke.input_event_counters.device),
                ),
                (
                    "input_keyboard_event_count",
                    FieldValue::U64(smoke.input_event_counters.keyboard),
                ),
                (
                    "input_pointer_event_count",
                    FieldValue::U64(smoke.input_event_counters.pointer),
                ),
                (
                    "input_touch_event_count",
                    FieldValue::U64(smoke.input_event_counters.touch),
                ),
                (
                    "input_gesture_event_count",
                    FieldValue::U64(smoke.input_event_counters.gesture),
                ),
                (
                    "input_tablet_event_count",
                    FieldValue::U64(smoke.input_event_counters.tablet),
                ),
                (
                    "input_switch_event_count",
                    FieldValue::U64(smoke.input_event_counters.switch),
                ),
                (
                    "input_special_event_count",
                    FieldValue::U64(smoke.input_event_counters.special),
                ),
                (
                    "surface_commit_count",
                    FieldValue::U64(smoke.surface_commit_count),
                ),
                (
                    "xdg_toplevel_count",
                    FieldValue::U64(smoke.xdg_toplevel_count),
                ),
                ("xdg_popup_count", FieldValue::U64(smoke.xdg_popup_count)),
                (
                    "title_changed_count",
                    FieldValue::U64(smoke.title_changed_count),
                ),
                (
                    "app_id_changed_count",
                    FieldValue::U64(smoke.app_id_changed_count),
                ),
                (
                    "observed_title",
                    FieldValue::Str(smoke.observed_title.as_str()),
                ),
                (
                    "observed_app_id",
                    FieldValue::Str(smoke.observed_app_id.as_str()),
                ),
                ("title_matched", FieldValue::Bool(smoke.title_matched)),
                ("app_id_matched", FieldValue::Bool(smoke.app_id_matched)),
                (
                    "shm_buffer_commit_count",
                    FieldValue::U64(smoke.shm_buffer_commit_count),
                ),
                ("shm_buffer_width", FieldValue::U64(smoke.shm_buffer_width)),
                (
                    "shm_buffer_height",
                    FieldValue::U64(smoke.shm_buffer_height),
                ),
                (
                    "shm_buffer_pixels",
                    FieldValue::U64(smoke.shm_buffer_pixels),
                ),
                (
                    "policy_window_mapped",
                    FieldValue::Bool(smoke.policy_window_mapped),
                ),
                (
                    "policy_app_id_preserved",
                    FieldValue::Bool(smoke.policy_app_id_preserved),
                ),
                (
                    "policy_focused_after_map",
                    FieldValue::Bool(smoke.policy_focused_after_map),
                ),
                (
                    "policy_geometry_preserved",
                    FieldValue::Bool(smoke.policy_geometry_preserved),
                ),
                ("policy_windows", FieldValue::U64(smoke.policy_windows)),
                (
                    "policy_backend_surface_presented",
                    FieldValue::Bool(smoke.policy_backend_surface_presented),
                ),
                (
                    "policy_presented_pixels",
                    FieldValue::U64(smoke.policy_presented_pixels),
                ),
            ],
        );

        if !smoke.passed() {
            return Err(String::from("Smithay Wayland client protocol smoke failed"));
        }

        if !config.scripted_client
            && !config.smithay_real_shm_frame
            && !config.smoke_test
            && !config.serve
            && config.idle_probe_ms.is_none()
        {
            emit(
                "compositor.exit",
                &config,
                &[(
                    "elapsed_ms",
                    FieldValue::U64(started.elapsed().as_millis() as u64),
                )],
            );
            return Ok(());
        }
    }

    if config.smithay_real_shm_frame {
        let frame = run_smithay_real_shm_frame_for_config(&config)?;
        emit(
            "compositor.smithay_real_shm_frame",
            &config,
            &[
                ("passed", FieldValue::Bool(frame.passed())),
                ("runtime_backend", FieldValue::Str(frame.runtime_backend)),
                (
                    "real_wayland_client",
                    FieldValue::Bool(frame.real_wayland_client),
                ),
                (
                    "real_wayland_metadata",
                    FieldValue::Bool(frame.real_wayland_metadata),
                ),
                (
                    "real_shm_pixels_captured",
                    FieldValue::Bool(frame.real_shm_pixels_captured),
                ),
                (
                    "real_shm_pixels_composited",
                    FieldValue::Bool(frame.real_shm_pixels_composited),
                ),
                (
                    "real_client_pixel_samples_verified",
                    FieldValue::Bool(frame.real_client_pixel_samples_verified),
                ),
                (
                    "policy_window_from_real_surface",
                    FieldValue::Bool(frame.policy_window_from_real_surface),
                ),
                (
                    "policy_app_id_preserved",
                    FieldValue::Bool(frame.policy_app_id_preserved),
                ),
                (
                    "policy_geometry_preserved",
                    FieldValue::Bool(frame.policy_geometry_preserved),
                ),
                (
                    "frame_ppm_written",
                    FieldValue::Bool(frame.frame_ppm_written),
                ),
                ("frame_width", FieldValue::U64(frame.frame_width)),
                ("frame_height", FieldValue::U64(frame.frame_height)),
                ("client_x", FieldValue::U64(frame.client_x)),
                ("client_y", FieldValue::U64(frame.client_y)),
                ("client_width", FieldValue::U64(frame.client_width)),
                ("client_height", FieldValue::U64(frame.client_height)),
                ("shm_stride", FieldValue::U64(frame.shm_stride)),
                (
                    "source_pixel_count",
                    FieldValue::U64(frame.source_pixel_count),
                ),
                (
                    "composited_pixels",
                    FieldValue::U64(frame.composited_pixels),
                ),
                ("frame_ppm_bytes", FieldValue::U64(frame.frame_ppm_bytes)),
                ("frame_checksum", FieldValue::U64(frame.frame_checksum)),
                (
                    "source_top_left_red",
                    FieldValue::U64(frame.source_top_left_red),
                ),
                (
                    "source_top_left_green",
                    FieldValue::U64(frame.source_top_left_green),
                ),
                (
                    "source_top_left_blue",
                    FieldValue::U64(frame.source_top_left_blue),
                ),
                (
                    "source_center_red",
                    FieldValue::U64(frame.source_center_red),
                ),
                (
                    "source_center_green",
                    FieldValue::U64(frame.source_center_green),
                ),
                (
                    "source_center_blue",
                    FieldValue::U64(frame.source_center_blue),
                ),
                (
                    "source_bottom_right_red",
                    FieldValue::U64(frame.source_bottom_right_red),
                ),
                (
                    "source_bottom_right_green",
                    FieldValue::U64(frame.source_bottom_right_green),
                ),
                (
                    "source_bottom_right_blue",
                    FieldValue::U64(frame.source_bottom_right_blue),
                ),
                (
                    "frame_ppm_path",
                    FieldValue::Str(frame.frame_ppm_path.as_str()),
                ),
            ],
        );

        if !frame.passed() {
            return Err(String::from("Smithay real SHM frame verification failed"));
        }

        if !config.scripted_client
            && !config.smoke_test
            && !config.serve
            && config.idle_probe_ms.is_none()
        {
            emit(
                "compositor.exit",
                &config,
                &[(
                    "elapsed_ms",
                    FieldValue::U64(started.elapsed().as_millis() as u64),
                )],
            );
            return Ok(());
        }
    }

    if config.scripted_client {
        let runtime = run_scripted_client_runtime_for_config(
            &config,
            config.scripted_client_preview.as_deref(),
        )?;
        emit(
            "compositor.scripted_client",
            &config,
            &[
                ("passed", FieldValue::Bool(runtime.passed())),
                ("runtime_backend", FieldValue::Str(runtime.runtime_backend)),
                ("runtime_trait", FieldValue::Bool(runtime.runtime_trait)),
                (
                    "inserted_wayland_clients",
                    FieldValue::U64(runtime.inserted_wayland_clients),
                ),
                (
                    "wayland_dispatch_count",
                    FieldValue::U64(runtime.wayland_dispatch_count),
                ),
                (
                    "calloop_dispatch_count",
                    FieldValue::U64(runtime.calloop_dispatch_count),
                ),
                (
                    "input_sources_ready",
                    FieldValue::Bool(runtime.input_sources_ready),
                ),
                (
                    "input_source_count",
                    FieldValue::U64(runtime.input_source_count),
                ),
                (
                    "input_event_loop_dispatch_count",
                    FieldValue::U64(runtime.input_event_loop_dispatch_count),
                ),
                (
                    "input_seat_ready",
                    FieldValue::Bool(runtime.input_seat_ready),
                ),
                (
                    "input_keyboard_handle_ready",
                    FieldValue::Bool(runtime.input_keyboard_handle_ready),
                ),
                (
                    "input_pointer_handle_ready",
                    FieldValue::Bool(runtime.input_pointer_handle_ready),
                ),
                (
                    "input_seat_dispatch_count",
                    FieldValue::U64(runtime.input_seat_dispatch_count),
                ),
                (
                    "input_keyboard_dispatch_count",
                    FieldValue::U64(runtime.input_keyboard_dispatch_count),
                ),
                (
                    "input_pointer_dispatch_count",
                    FieldValue::U64(runtime.input_pointer_dispatch_count),
                ),
                (
                    "input_event_count",
                    FieldValue::U64(runtime.input_event_counters.total),
                ),
                (
                    "input_device_event_count",
                    FieldValue::U64(runtime.input_event_counters.device),
                ),
                (
                    "input_keyboard_event_count",
                    FieldValue::U64(runtime.input_event_counters.keyboard),
                ),
                (
                    "input_pointer_event_count",
                    FieldValue::U64(runtime.input_event_counters.pointer),
                ),
                (
                    "input_touch_event_count",
                    FieldValue::U64(runtime.input_event_counters.touch),
                ),
                (
                    "input_gesture_event_count",
                    FieldValue::U64(runtime.input_event_counters.gesture),
                ),
                (
                    "input_tablet_event_count",
                    FieldValue::U64(runtime.input_event_counters.tablet),
                ),
                (
                    "input_switch_event_count",
                    FieldValue::U64(runtime.input_event_counters.switch),
                ),
                (
                    "input_special_event_count",
                    FieldValue::U64(runtime.input_event_counters.special),
                ),
                (
                    "smithay_protocol_globals",
                    FieldValue::U64(runtime.smithay_protocol_globals),
                ),
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
        let readiness = run_service_ready_for_config(&config)?;
        emit(
            "compositor.ready",
            &config,
            &[
                ("ready", FieldValue::Bool(readiness.passed())),
                (
                    "runtime_backend",
                    FieldValue::Str(readiness.runtime_backend),
                ),
                ("runtime_trait", FieldValue::Bool(readiness.runtime_trait)),
                (
                    "inserted_wayland_clients",
                    FieldValue::U64(readiness.inserted_wayland_clients),
                ),
                (
                    "wayland_dispatch_count",
                    FieldValue::U64(readiness.wayland_dispatch_count),
                ),
                (
                    "calloop_dispatch_count",
                    FieldValue::U64(readiness.calloop_dispatch_count),
                ),
                (
                    "input_sources_ready",
                    FieldValue::Bool(readiness.input_sources_ready),
                ),
                (
                    "input_source_count",
                    FieldValue::U64(readiness.input_source_count),
                ),
                (
                    "input_event_loop_dispatch_count",
                    FieldValue::U64(readiness.input_event_loop_dispatch_count),
                ),
                (
                    "input_seat_ready",
                    FieldValue::Bool(readiness.input_seat_ready),
                ),
                (
                    "input_keyboard_handle_ready",
                    FieldValue::Bool(readiness.input_keyboard_handle_ready),
                ),
                (
                    "input_pointer_handle_ready",
                    FieldValue::Bool(readiness.input_pointer_handle_ready),
                ),
                (
                    "input_seat_dispatch_count",
                    FieldValue::U64(readiness.input_seat_dispatch_count),
                ),
                (
                    "input_keyboard_dispatch_count",
                    FieldValue::U64(readiness.input_keyboard_dispatch_count),
                ),
                (
                    "input_pointer_dispatch_count",
                    FieldValue::U64(readiness.input_pointer_dispatch_count),
                ),
                (
                    "input_event_count",
                    FieldValue::U64(readiness.input_event_counters.total),
                ),
                (
                    "input_device_event_count",
                    FieldValue::U64(readiness.input_event_counters.device),
                ),
                (
                    "input_keyboard_event_count",
                    FieldValue::U64(readiness.input_event_counters.keyboard),
                ),
                (
                    "input_pointer_event_count",
                    FieldValue::U64(readiness.input_event_counters.pointer),
                ),
                (
                    "input_touch_event_count",
                    FieldValue::U64(readiness.input_event_counters.touch),
                ),
                (
                    "input_gesture_event_count",
                    FieldValue::U64(readiness.input_event_counters.gesture),
                ),
                (
                    "input_tablet_event_count",
                    FieldValue::U64(readiness.input_event_counters.tablet),
                ),
                (
                    "input_switch_event_count",
                    FieldValue::U64(readiness.input_event_counters.switch),
                ),
                (
                    "input_special_event_count",
                    FieldValue::U64(readiness.input_event_counters.special),
                ),
                (
                    "smithay_protocol_globals",
                    FieldValue::U64(readiness.smithay_protocol_globals),
                ),
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
        let mut session_socket = match bind_session_socket(&config)? {
            Some(socket) => {
                emit_socket_bound(&config, &socket);
                Some(socket)
            }
            None => {
                emit_socket_unavailable(&config);
                None
            }
        };
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
            run_service_loop_for_config(
                &config,
                session_socket.as_ref(),
                Duration::from_millis(duration_ms),
            )?;
            if let Some(mut socket) = session_socket.take() {
                let path = socket.path_string();
                let removed = socket.cleanup();
                emit_socket_unbound(&config, path.as_str(), removed);
            }
            emit(
                "compositor.service_exit",
                &config,
                &[
                    ("bounded", FieldValue::Bool(true)),
                    ("serve_for_ms", FieldValue::U64(duration_ms)),
                ],
            );
        } else {
            run_unbounded_service_loop_for_config(&config, session_socket.as_ref())?;
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

#[derive(Debug)]
struct BoundSessionSocket {
    socket_name: String,
    runtime_dir: String,
    path: PathBuf,
    stale_socket_removed: bool,
    cleaned: bool,
    listener: UnixListener,
}

impl BoundSessionSocket {
    fn path_string(&self) -> String {
        self.path.display().to_string()
    }

    fn cleanup(&mut self) -> bool {
        if self.cleaned {
            return false;
        }

        self.cleaned = true;
        fs::remove_file(&self.path).is_ok()
    }

    fn accept_messages(&self) -> Result<Vec<String>, String> {
        let mut messages = Vec::new();

        loop {
            match self.listener.accept() {
                Ok((mut stream, _addr)) => {
                    if let Err(error) = stream.set_read_timeout(Some(Duration::from_millis(100))) {
                        if error.kind() != ErrorKind::InvalidInput {
                            return Err(format!(
                                "failed to set socket client read timeout: {error}"
                            ));
                        }
                    }
                    let mut message = String::new();
                    stream.read_to_string(&mut message).map_err(|error| {
                        format!("failed to read compositor socket client message: {error}")
                    })?;
                    messages.push(message);
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(error) => {
                    return Err(format!(
                        "failed to accept compositor socket client: {error}"
                    ));
                }
            }
        }

        Ok(messages)
    }
}

impl Drop for BoundSessionSocket {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = fs::remove_file(&self.path);
            self.cleaned = true;
        }
    }
}

fn bind_session_socket(config: &RunConfig) -> Result<Option<BoundSessionSocket>, String> {
    bind_session_socket_in_runtime(&config.socket, env::var("XDG_RUNTIME_DIR").ok())
}

fn bind_session_socket_in_runtime(
    socket_name: &str,
    runtime_dir: Option<String>,
) -> Result<Option<BoundSessionSocket>, String> {
    let Some(runtime_dir) = runtime_dir.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    let runtime_path = Path::new(runtime_dir.as_str());
    if !runtime_path.is_dir() {
        return Ok(None);
    }

    let socket_path = if Path::new(socket_name).is_absolute() {
        PathBuf::from(socket_name)
    } else {
        runtime_path.join(socket_name)
    };
    let mut stale_socket_removed = false;

    if let Ok(metadata) = fs::symlink_metadata(&socket_path) {
        if metadata.file_type().is_socket() {
            fs::remove_file(&socket_path).map_err(|error| {
                format!(
                    "failed to remove stale compositor socket {}: {error}",
                    socket_path.display()
                )
            })?;
            stale_socket_removed = true;
        } else {
            return Err(format!(
                "refusing to replace non-socket compositor path {}",
                socket_path.display()
            ));
        }
    }

    let listener = UnixListener::bind(&socket_path).map_err(|error| {
        format!(
            "failed to bind compositor socket {}: {error}",
            socket_path.display()
        )
    })?;
    listener.set_nonblocking(true).map_err(|error| {
        format!(
            "failed to set compositor socket {} nonblocking: {error}",
            socket_path.display()
        )
    })?;

    Ok(Some(BoundSessionSocket {
        socket_name: socket_name.to_string(),
        runtime_dir,
        path: socket_path,
        stale_socket_removed,
        cleaned: false,
        listener,
    }))
}

#[derive(Debug)]
struct SocketClientRuntime<B: CompositorRuntime = HeadlessCompositor> {
    backend: B,
    manager: SurfaceManager,
    clients: Vec<SocketClientRecord>,
}

impl SocketClientRuntime<HeadlessCompositor> {
    fn new() -> Self {
        Self::with_backend(HeadlessCompositor::default())
    }
}

impl<B: CompositorRuntime> SocketClientRuntime<B> {
    fn with_backend(backend: B) -> Self {
        Self {
            backend,
            manager: SurfaceManager::new(OutputLayout::new(1400, 900, 42)),
            clients: Vec::new(),
        }
    }

    fn runtime_backend(&self) -> &'static str {
        self.backend.runtime_name()
    }

    fn handle_stream(&mut self, message: &str) -> Vec<SocketClientReport> {
        let reports: Vec<_> = message
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| self.handle_command(line))
            .collect();

        if reports.is_empty() {
            vec![SocketClientReport::invalid()]
        } else {
            reports
        }
    }

    fn handle_command(&mut self, message: &str) -> SocketClientReport {
        let Some(command) = DemoSocketCommand::parse(message) else {
            return SocketClientReport::invalid();
        };

        match command.action {
            DemoSocketAction::Surface => self.map_surface(command),
            DemoSocketAction::Damage => self.damage_surface(command),
            DemoSocketAction::Move => self.manage_surface(command),
            DemoSocketAction::Resize => self.manage_surface(command),
            DemoSocketAction::Maximize => self.manage_surface(command),
            DemoSocketAction::Fullscreen => self.manage_surface(command),
            DemoSocketAction::Close => self.close_surface(command),
            DemoSocketAction::Invalid => SocketClientReport::invalid(),
        }
    }

    fn map_surface(&mut self, command: DemoSocketCommand) -> SocketClientReport {
        let client_name = format!("socket-client-{}", command.app_id);
        let client = self.backend.connect_client(client_name.as_str());
        let backend_surface = self.backend.submit_surface(
            client,
            command.title.as_str(),
            command.width,
            command.height,
        );
        let backend_surface_presented = backend_surface.is_ok();
        let policy_window = map_scripted_app_toplevel(
            &mut self.manager,
            command.title.as_str(),
            command.app_id.as_str(),
            command.width as i32,
            command.height as i32,
        )
        .ok()
        .and_then(|surface| {
            self.manager
                .surface(surface)
                .and_then(|surface| surface.window_id)
        });
        let policy_window_mapped = policy_window.is_some();
        let policy_app_id_preserved = policy_window
            .and_then(|window| self.manager.policy().window(window))
            .map(|window| window.app_id.as_deref() == Some(command.app_id.as_str()))
            .unwrap_or(false);
        if let Some((backend_surface, policy_surface)) = backend_surface
            .ok()
            .zip(self.find_policy_surface(policy_window))
        {
            self.clients.push(SocketClientRecord {
                app_id: command.app_id.clone(),
                title: command.title.clone(),
                client,
                backend_surface,
                policy_surface,
            });
        }
        let frame = self.backend.present();
        let focused = self.focused_window_state();
        let geometry = self.window_geometry_for_surface(policy_window);

        SocketClientReport {
            message_valid: true,
            action: DemoSocketAction::Surface,
            title: command.title,
            app_id: command.app_id,
            width: command.width,
            height: command.height,
            backend_surface_presented,
            backend_surface_damaged: false,
            backend_surface_closed: false,
            policy_window_mapped,
            policy_app_id_preserved,
            policy_window_moved: false,
            policy_window_resized: false,
            policy_window_maximized: false,
            policy_window_fullscreen: false,
            policy_window_closed: false,
            client_disconnected: false,
            frame: frame.frame,
            damaged_surfaces: frame.damaged_surfaces,
            backend_clients: frame.client_count,
            backend_surfaces: frame.surface_count,
            inserted_wayland_clients: self.backend.inserted_wayland_clients(),
            wayland_dispatch_count: self.backend.wayland_dispatch_count(),
            calloop_dispatch_count: self.backend.calloop_dispatch_count(),
            input_sources_ready: self.backend.input_sources_ready(),
            input_source_count: self.backend.input_source_count(),
            input_event_loop_dispatch_count: self.backend.input_event_loop_dispatch_count(),
            input_seat_ready: self.backend.input_seat_ready(),
            input_keyboard_handle_ready: self.backend.input_keyboard_handle_ready(),
            input_pointer_handle_ready: self.backend.input_pointer_handle_ready(),
            input_seat_dispatch_count: self.backend.input_seat_dispatch_count(),
            input_keyboard_dispatch_count: self.backend.input_keyboard_dispatch_count(),
            input_pointer_dispatch_count: self.backend.input_pointer_dispatch_count(),
            input_event_counters: self.backend.input_event_counters(),
            smithay_protocol_globals: self.backend.smithay_protocol_global_count(),
            policy_windows: self.manager.policy().windows().len() as u64,
            visible_windows: self.manager.policy().visible_windows().count() as u64,
            policy_state: geometry.state,
            policy_x: geometry.x,
            policy_y: geometry.y,
            policy_width: geometry.width,
            policy_height: geometry.height,
            focused_window: focused.is_some(),
            focused_title: focused
                .as_ref()
                .map(|window| window.title.clone())
                .unwrap_or_default(),
            focused_app_id: focused
                .and_then(|window| window.app_id.clone())
                .unwrap_or_default(),
        }
    }

    fn manage_surface(&mut self, command: DemoSocketCommand) -> SocketClientReport {
        let record = self
            .find_client(command.app_id.as_str(), command.title.as_str())
            .cloned();
        let mut policy_window_moved = false;
        let mut policy_window_resized = false;
        let mut policy_window_maximized = false;
        let mut policy_window_fullscreen = false;
        let mut backend_surface_damaged = false;
        let policy_window = record
            .as_ref()
            .and_then(|record| self.window_id_for_surface(record.policy_surface));

        if let Some(record) = record.as_ref() {
            match command.action {
                DemoSocketAction::Move => {
                    policy_window_moved =
                        self.manager
                            .move_toplevel(record.policy_surface, command.x, command.y);
                    backend_surface_damaged = policy_window_moved
                        && self.backend.mark_damaged(record.backend_surface).is_ok();
                }
                DemoSocketAction::Resize => {
                    policy_window_resized = self.manager.resize_toplevel(
                        record.policy_surface,
                        command.width as i32,
                        command.height as i32,
                    );
                    backend_surface_damaged = policy_window_resized
                        && self.backend.mark_damaged(record.backend_surface).is_ok();
                }
                DemoSocketAction::Maximize => {
                    policy_window_maximized = self
                        .manager
                        .request_maximize(record.policy_surface)
                        .is_some();
                    backend_surface_damaged = policy_window_maximized
                        && self.backend.mark_damaged(record.backend_surface).is_ok();
                }
                DemoSocketAction::Fullscreen => {
                    policy_window_fullscreen = self
                        .manager
                        .request_fullscreen(record.policy_surface)
                        .is_some();
                    backend_surface_damaged = policy_window_fullscreen
                        && self.backend.mark_damaged(record.backend_surface).is_ok();
                }
                _ => {}
            }
        }

        let frame = self.backend.present();
        let focused = self.focused_window_state();
        let geometry = self.window_geometry_for_surface(policy_window);

        SocketClientReport {
            message_valid: true,
            action: command.action,
            title: command.title,
            app_id: command.app_id,
            width: command.width,
            height: command.height,
            backend_surface_presented: false,
            backend_surface_damaged: backend_surface_damaged && frame.damaged_surfaces == 1,
            backend_surface_closed: false,
            policy_window_mapped: false,
            policy_app_id_preserved: false,
            policy_window_moved,
            policy_window_resized,
            policy_window_maximized,
            policy_window_fullscreen,
            policy_window_closed: false,
            client_disconnected: false,
            frame: frame.frame,
            damaged_surfaces: frame.damaged_surfaces,
            backend_clients: frame.client_count,
            backend_surfaces: frame.surface_count,
            inserted_wayland_clients: self.backend.inserted_wayland_clients(),
            wayland_dispatch_count: self.backend.wayland_dispatch_count(),
            calloop_dispatch_count: self.backend.calloop_dispatch_count(),
            input_sources_ready: self.backend.input_sources_ready(),
            input_source_count: self.backend.input_source_count(),
            input_event_loop_dispatch_count: self.backend.input_event_loop_dispatch_count(),
            input_seat_ready: self.backend.input_seat_ready(),
            input_keyboard_handle_ready: self.backend.input_keyboard_handle_ready(),
            input_pointer_handle_ready: self.backend.input_pointer_handle_ready(),
            input_seat_dispatch_count: self.backend.input_seat_dispatch_count(),
            input_keyboard_dispatch_count: self.backend.input_keyboard_dispatch_count(),
            input_pointer_dispatch_count: self.backend.input_pointer_dispatch_count(),
            input_event_counters: self.backend.input_event_counters(),
            smithay_protocol_globals: self.backend.smithay_protocol_global_count(),
            policy_windows: self.manager.policy().windows().len() as u64,
            visible_windows: self.manager.policy().visible_windows().count() as u64,
            policy_state: geometry.state,
            policy_x: geometry.x,
            policy_y: geometry.y,
            policy_width: geometry.width,
            policy_height: geometry.height,
            focused_window: focused.is_some(),
            focused_title: focused
                .as_ref()
                .map(|window| window.title.clone())
                .unwrap_or_default(),
            focused_app_id: focused
                .and_then(|window| window.app_id.clone())
                .unwrap_or_default(),
        }
    }

    fn damage_surface(&mut self, command: DemoSocketCommand) -> SocketClientReport {
        let backend_surface = self
            .find_client(command.app_id.as_str(), command.title.as_str())
            .map(|record| record.backend_surface);
        let policy_window = self
            .find_client(command.app_id.as_str(), command.title.as_str())
            .and_then(|record| self.window_id_for_surface(record.policy_surface));
        let backend_surface_damaged = backend_surface
            .map(|surface| self.backend.mark_damaged(surface).is_ok())
            .unwrap_or(false);
        let frame = self.backend.present();
        let focused = self.focused_window_state();
        let geometry = self.window_geometry_for_surface(policy_window);

        SocketClientReport {
            message_valid: true,
            action: DemoSocketAction::Damage,
            title: command.title,
            app_id: command.app_id,
            width: command.width,
            height: command.height,
            backend_surface_presented: false,
            backend_surface_damaged: backend_surface_damaged && frame.damaged_surfaces == 1,
            backend_surface_closed: false,
            policy_window_mapped: false,
            policy_app_id_preserved: false,
            policy_window_moved: false,
            policy_window_resized: false,
            policy_window_maximized: false,
            policy_window_fullscreen: false,
            policy_window_closed: false,
            client_disconnected: false,
            frame: frame.frame,
            damaged_surfaces: frame.damaged_surfaces,
            backend_clients: frame.client_count,
            backend_surfaces: frame.surface_count,
            inserted_wayland_clients: self.backend.inserted_wayland_clients(),
            wayland_dispatch_count: self.backend.wayland_dispatch_count(),
            calloop_dispatch_count: self.backend.calloop_dispatch_count(),
            input_sources_ready: self.backend.input_sources_ready(),
            input_source_count: self.backend.input_source_count(),
            input_event_loop_dispatch_count: self.backend.input_event_loop_dispatch_count(),
            input_seat_ready: self.backend.input_seat_ready(),
            input_keyboard_handle_ready: self.backend.input_keyboard_handle_ready(),
            input_pointer_handle_ready: self.backend.input_pointer_handle_ready(),
            input_seat_dispatch_count: self.backend.input_seat_dispatch_count(),
            input_keyboard_dispatch_count: self.backend.input_keyboard_dispatch_count(),
            input_pointer_dispatch_count: self.backend.input_pointer_dispatch_count(),
            input_event_counters: self.backend.input_event_counters(),
            smithay_protocol_globals: self.backend.smithay_protocol_global_count(),
            policy_windows: self.manager.policy().windows().len() as u64,
            visible_windows: self.manager.policy().visible_windows().count() as u64,
            policy_state: geometry.state,
            policy_x: geometry.x,
            policy_y: geometry.y,
            policy_width: geometry.width,
            policy_height: geometry.height,
            focused_window: focused.is_some(),
            focused_title: focused
                .as_ref()
                .map(|window| window.title.clone())
                .unwrap_or_default(),
            focused_app_id: focused
                .and_then(|window| window.app_id.clone())
                .unwrap_or_default(),
        }
    }

    fn close_surface(&mut self, command: DemoSocketCommand) -> SocketClientReport {
        let record_index = self.find_client_index(command.app_id.as_str(), command.title.as_str());
        let record = record_index.map(|index| self.clients.remove(index));
        let policy_window = record
            .as_ref()
            .and_then(|record| self.window_id_for_surface(record.policy_surface));
        let backend_surface_closed = record
            .as_ref()
            .map(|record| self.backend.close_surface(record.backend_surface).is_ok())
            .unwrap_or(false);
        let policy_window_closed = record
            .as_ref()
            .map(|record| {
                self.manager.close(record.policy_surface)
                    && self
                        .manager
                        .surface(record.policy_surface)
                        .map(|surface| surface.phase == SurfacePhase::Closed)
                        .unwrap_or(false)
            })
            .unwrap_or(false);
        let client_disconnected = record
            .as_ref()
            .map(|record| self.backend.disconnect_client(record.client).is_ok())
            .unwrap_or(false);
        let frame = self.backend.present();
        let focused = self.focused_window_state();
        let geometry = self.window_geometry_for_surface(policy_window);

        SocketClientReport {
            message_valid: true,
            action: DemoSocketAction::Close,
            title: command.title,
            app_id: command.app_id,
            width: command.width,
            height: command.height,
            backend_surface_presented: false,
            backend_surface_damaged: false,
            backend_surface_closed,
            policy_window_mapped: false,
            policy_app_id_preserved: false,
            policy_window_moved: false,
            policy_window_resized: false,
            policy_window_maximized: false,
            policy_window_fullscreen: false,
            policy_window_closed,
            client_disconnected,
            frame: frame.frame,
            damaged_surfaces: frame.damaged_surfaces,
            backend_clients: frame.client_count,
            backend_surfaces: frame.surface_count,
            inserted_wayland_clients: self.backend.inserted_wayland_clients(),
            wayland_dispatch_count: self.backend.wayland_dispatch_count(),
            calloop_dispatch_count: self.backend.calloop_dispatch_count(),
            input_sources_ready: self.backend.input_sources_ready(),
            input_source_count: self.backend.input_source_count(),
            input_event_loop_dispatch_count: self.backend.input_event_loop_dispatch_count(),
            input_seat_ready: self.backend.input_seat_ready(),
            input_keyboard_handle_ready: self.backend.input_keyboard_handle_ready(),
            input_pointer_handle_ready: self.backend.input_pointer_handle_ready(),
            input_seat_dispatch_count: self.backend.input_seat_dispatch_count(),
            input_keyboard_dispatch_count: self.backend.input_keyboard_dispatch_count(),
            input_pointer_dispatch_count: self.backend.input_pointer_dispatch_count(),
            input_event_counters: self.backend.input_event_counters(),
            smithay_protocol_globals: self.backend.smithay_protocol_global_count(),
            policy_windows: self.manager.policy().windows().len() as u64,
            visible_windows: self.manager.policy().visible_windows().count() as u64,
            policy_state: geometry.state,
            policy_x: geometry.x,
            policy_y: geometry.y,
            policy_width: geometry.width,
            policy_height: geometry.height,
            focused_window: focused.is_some(),
            focused_title: focused
                .as_ref()
                .map(|window| window.title.clone())
                .unwrap_or_default(),
            focused_app_id: focused
                .and_then(|window| window.app_id.clone())
                .unwrap_or_default(),
        }
    }

    fn focused_window_state(&self) -> Option<&backlit_window_policy::Window> {
        self.manager
            .policy()
            .focused()
            .and_then(|window| self.manager.policy().window(window))
    }

    fn window_id_for_surface(
        &self,
        policy_surface: backlit_surface::SurfaceId,
    ) -> Option<backlit_window_policy::WindowId> {
        self.manager
            .surface(policy_surface)
            .and_then(|surface| surface.window_id)
    }

    fn window_geometry_for_surface(
        &self,
        policy_window: Option<backlit_window_policy::WindowId>,
    ) -> SocketWindowGeometry {
        policy_window
            .and_then(|window| self.manager.policy().window(window))
            .map(SocketWindowGeometry::from_window)
            .unwrap_or_default()
    }

    fn find_client(&self, app_id: &str, title: &str) -> Option<&SocketClientRecord> {
        self.clients
            .iter()
            .find(|record| record.app_id == app_id || record.title == title)
    }

    fn find_client_index(&self, app_id: &str, title: &str) -> Option<usize> {
        self.clients
            .iter()
            .position(|record| record.app_id == app_id || record.title == title)
    }

    fn find_policy_surface(
        &self,
        policy_window: Option<backlit_window_policy::WindowId>,
    ) -> Option<backlit_surface::SurfaceId> {
        let policy_window = policy_window?;
        self.manager.policy().window(policy_window)?;
        self.manager.surface_ids().find(|surface| {
            self.manager
                .surface(*surface)
                .map(|known| known.window_id == Some(policy_window))
                .unwrap_or(false)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SocketClientReport {
    message_valid: bool,
    action: DemoSocketAction,
    title: String,
    app_id: String,
    width: u32,
    height: u32,
    backend_surface_presented: bool,
    backend_surface_damaged: bool,
    backend_surface_closed: bool,
    policy_window_mapped: bool,
    policy_app_id_preserved: bool,
    policy_window_moved: bool,
    policy_window_resized: bool,
    policy_window_maximized: bool,
    policy_window_fullscreen: bool,
    policy_window_closed: bool,
    client_disconnected: bool,
    frame: u64,
    damaged_surfaces: u64,
    backend_clients: u64,
    backend_surfaces: u64,
    inserted_wayland_clients: u64,
    wayland_dispatch_count: u64,
    calloop_dispatch_count: u64,
    input_sources_ready: bool,
    input_source_count: u64,
    input_event_loop_dispatch_count: u64,
    input_seat_ready: bool,
    input_keyboard_handle_ready: bool,
    input_pointer_handle_ready: bool,
    input_seat_dispatch_count: u64,
    input_keyboard_dispatch_count: u64,
    input_pointer_dispatch_count: u64,
    input_event_counters: InputEventCounters,
    smithay_protocol_globals: u64,
    policy_windows: u64,
    visible_windows: u64,
    policy_state: &'static str,
    policy_x: i32,
    policy_y: i32,
    policy_width: i32,
    policy_height: i32,
    focused_window: bool,
    focused_title: String,
    focused_app_id: String,
}

impl SocketClientReport {
    fn invalid() -> Self {
        Self {
            message_valid: false,
            action: DemoSocketAction::Invalid,
            title: String::new(),
            app_id: String::new(),
            width: 0,
            height: 0,
            backend_surface_presented: false,
            backend_surface_damaged: false,
            backend_surface_closed: false,
            policy_window_mapped: false,
            policy_app_id_preserved: false,
            policy_window_moved: false,
            policy_window_resized: false,
            policy_window_maximized: false,
            policy_window_fullscreen: false,
            policy_window_closed: false,
            client_disconnected: false,
            frame: 0,
            damaged_surfaces: 0,
            backend_clients: 0,
            backend_surfaces: 0,
            inserted_wayland_clients: 0,
            wayland_dispatch_count: 0,
            calloop_dispatch_count: 0,
            input_sources_ready: false,
            input_source_count: 0,
            input_event_loop_dispatch_count: 0,
            input_seat_ready: false,
            input_keyboard_handle_ready: false,
            input_pointer_handle_ready: false,
            input_seat_dispatch_count: 0,
            input_keyboard_dispatch_count: 0,
            input_pointer_dispatch_count: 0,
            input_event_counters: InputEventCounters::default(),
            smithay_protocol_globals: 0,
            policy_windows: 0,
            visible_windows: 0,
            policy_state: "none",
            policy_x: 0,
            policy_y: 0,
            policy_width: 0,
            policy_height: 0,
            focused_window: false,
            focused_title: String::new(),
            focused_app_id: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SocketWindowGeometry {
    state: &'static str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl SocketWindowGeometry {
    fn from_window(window: &backlit_window_policy::Window) -> Self {
        Self {
            state: window_state_name(window.state),
            x: window.geometry.x,
            y: window.geometry.y,
            width: window.geometry.width,
            height: window.geometry.height,
        }
    }
}

impl Default for SocketWindowGeometry {
    fn default() -> Self {
        Self {
            state: "none",
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

fn window_state_name(state: WindowState) -> &'static str {
    match state {
        WindowState::Normal => "normal",
        WindowState::Maximized => "maximized",
        WindowState::Fullscreen => "fullscreen",
        WindowState::Minimized => "minimized",
        WindowState::Snapped => "snapped",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DemoSocketAction {
    Surface,
    Damage,
    Move,
    Resize,
    Maximize,
    Fullscreen,
    Close,
    Invalid,
}

impl DemoSocketAction {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Surface => "surface",
            Self::Damage => "damage",
            Self::Move => "move",
            Self::Resize => "resize",
            Self::Maximize => "maximize",
            Self::Fullscreen => "fullscreen",
            Self::Close => "close",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SocketClientRecord {
    app_id: String,
    title: String,
    client: ClientId,
    backend_surface: BackendSurfaceId,
    policy_surface: backlit_surface::SurfaceId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemoSocketCommand {
    action: DemoSocketAction,
    title: String,
    app_id: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl DemoSocketCommand {
    fn parse(message: &str) -> Option<Self> {
        let mut tokens = message.split_whitespace();
        if tokens.next()? != "BACKLIT_DEMO_CLIENT" {
            return None;
        }
        let action = match tokens.next()? {
            "surface" => DemoSocketAction::Surface,
            "damage" => DemoSocketAction::Damage,
            "move" => DemoSocketAction::Move,
            "resize" => DemoSocketAction::Resize,
            "maximize" => DemoSocketAction::Maximize,
            "fullscreen" => DemoSocketAction::Fullscreen,
            "close" => DemoSocketAction::Close,
            _ => return None,
        };

        let mut title = None;
        let mut app_id = None;
        let mut x = None;
        let mut y = None;
        let mut width = None;
        let mut height = None;

        for token in tokens {
            if let Some(value) = token.strip_prefix("title=") {
                title = Some(value.to_string());
            } else if let Some(value) = token.strip_prefix("app_id=") {
                app_id = Some(value.to_string());
            } else if let Some(value) = token.strip_prefix("x=") {
                x = value.parse::<i32>().ok();
            } else if let Some(value) = token.strip_prefix("y=") {
                y = value.parse::<i32>().ok();
            } else if let Some(value) = token.strip_prefix("width=") {
                width = value.parse::<u32>().ok();
            } else if let Some(value) = token.strip_prefix("height=") {
                height = value.parse::<u32>().ok();
            }
        }

        let title = title
            .filter(|title| !title.is_empty())
            .or_else(|| app_id.clone())?;
        let app_id = app_id
            .filter(|app_id| !app_id.is_empty())
            .unwrap_or_else(|| title.clone());

        Some(Self {
            action,
            title,
            app_id,
            x: x.unwrap_or(0),
            y: y.unwrap_or(0),
            width: width.unwrap_or(1).max(1),
            height: height.unwrap_or(1).max(1),
        })
    }
}

fn run_service_loop_for_config(
    config: &RunConfig,
    socket: Option<&BoundSessionSocket>,
    duration: Duration,
) -> Result<(), String> {
    match config.runtime {
        RuntimeKind::Headless => {
            let mut runtime = SocketClientRuntime::new();
            run_service_loop_for(config, socket, &mut runtime, duration)
        }
        RuntimeKind::Smithay => run_service_loop_with_smithay(config, socket, duration),
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn run_service_loop_with_smithay(
    config: &RunConfig,
    socket: Option<&BoundSessionSocket>,
    duration: Duration,
) -> Result<(), String> {
    let backend = SmithayCompositorRuntime::try_new().map_err(|error| error.to_string())?;
    let mut runtime = SocketClientRuntime::with_backend(backend);
    run_service_loop_for(config, socket, &mut runtime, duration)
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn run_service_loop_with_smithay(
    _config: &RunConfig,
    _socket: Option<&BoundSessionSocket>,
    _duration: Duration,
) -> Result<(), String> {
    Err(String::from(
        "smithay runtime requires Linux and the smithay-backend feature",
    ))
}

fn run_unbounded_service_loop_for_config(
    config: &RunConfig,
    socket: Option<&BoundSessionSocket>,
) -> Result<(), String> {
    match config.runtime {
        RuntimeKind::Headless => {
            let mut runtime = SocketClientRuntime::new();
            run_unbounded_service_loop_for(config, socket, &mut runtime)
        }
        RuntimeKind::Smithay => run_unbounded_service_loop_with_smithay(config, socket),
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn run_unbounded_service_loop_with_smithay(
    config: &RunConfig,
    socket: Option<&BoundSessionSocket>,
) -> Result<(), String> {
    let backend = SmithayCompositorRuntime::try_new().map_err(|error| error.to_string())?;
    let mut runtime = SocketClientRuntime::with_backend(backend);
    run_unbounded_service_loop_for(config, socket, &mut runtime)
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn run_unbounded_service_loop_with_smithay(
    _config: &RunConfig,
    _socket: Option<&BoundSessionSocket>,
) -> Result<(), String> {
    Err(String::from(
        "smithay runtime requires Linux and the smithay-backend feature",
    ))
}

fn run_unbounded_service_loop_for<B: CompositorRuntime>(
    config: &RunConfig,
    socket: Option<&BoundSessionSocket>,
    runtime: &mut SocketClientRuntime<B>,
) -> Result<(), String> {
    loop {
        poll_socket_clients(config, socket, runtime)?;
        thread::sleep(Duration::from_millis(10));
    }
}

fn run_service_loop_for<B: CompositorRuntime>(
    config: &RunConfig,
    socket: Option<&BoundSessionSocket>,
    runtime: &mut SocketClientRuntime<B>,
    duration: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + duration;

    while Instant::now() < deadline {
        poll_socket_clients(config, socket, runtime)?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        thread::sleep(remaining.min(Duration::from_millis(10)));
    }

    Ok(())
}

fn poll_socket_clients<B: CompositorRuntime>(
    config: &RunConfig,
    socket: Option<&BoundSessionSocket>,
    runtime: &mut SocketClientRuntime<B>,
) -> Result<(), String> {
    let Some(socket) = socket else {
        return Ok(());
    };

    let runtime_backend = runtime.runtime_backend();
    for message in socket.accept_messages()? {
        for report in runtime.handle_stream(message.as_str()) {
            emit_socket_client(config, runtime_backend, &report);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SmithayClientSmoke {
    runtime_backend: &'static str,
    smithay_protocol_globals: u64,
    registry_global_count: u64,
    registry_announced: bool,
    mvp_protocol_globals: u64,
    mvp_protocol_globals_announced: bool,
    wl_output_bound: bool,
    xdg_output_manager_bound: bool,
    viewporter_bound: bool,
    presentation_bound: bool,
    linux_dmabuf_bound: bool,
    linux_dmabuf_version: u64,
    linux_dmabuf_version_at_least_4: bool,
    seat_global_announced: bool,
    seat_bound: bool,
    seat_name_observed: bool,
    seat_keyboard_capability: bool,
    seat_pointer_capability: bool,
    keyboard_bound: bool,
    pointer_bound: bool,
    compositor_bound: bool,
    shm_bound: bool,
    shm_buffer_created: bool,
    shm_buffer_attached: bool,
    xdg_wm_base_bound: bool,
    surface_created: bool,
    xdg_toplevel_created: bool,
    configure_received: bool,
    configure_acked: bool,
    surface_committed: bool,
    inserted_wayland_clients: u64,
    wayland_dispatch_count: u64,
    calloop_dispatch_count: u64,
    input_sources_ready: bool,
    input_source_count: u64,
    input_event_loop_dispatch_count: u64,
    input_seat_ready: bool,
    input_keyboard_handle_ready: bool,
    input_pointer_handle_ready: bool,
    input_seat_dispatch_count: u64,
    input_keyboard_dispatch_count: u64,
    input_pointer_dispatch_count: u64,
    input_event_counters: InputEventCounters,
    surface_commit_count: u64,
    xdg_toplevel_count: u64,
    xdg_popup_count: u64,
    title_changed_count: u64,
    app_id_changed_count: u64,
    observed_title: String,
    observed_app_id: String,
    title_matched: bool,
    app_id_matched: bool,
    shm_buffer_commit_count: u64,
    shm_buffer_width: u64,
    shm_buffer_height: u64,
    shm_buffer_pixels: u64,
    policy_window_mapped: bool,
    policy_app_id_preserved: bool,
    policy_focused_after_map: bool,
    policy_geometry_preserved: bool,
    policy_windows: u64,
    policy_backend_surface_presented: bool,
    policy_presented_pixels: u64,
}

impl SmithayClientSmoke {
    fn passed(&self) -> bool {
        self.runtime_backend == "smithay-compositor-runtime"
            && self.smithay_protocol_globals >= 10
            && self.registry_global_count >= 10
            && self.registry_announced
            && self.mvp_protocol_globals >= 7
            && self.mvp_protocol_globals_announced
            && self.wl_output_bound
            && self.xdg_output_manager_bound
            && self.viewporter_bound
            && self.presentation_bound
            && self.linux_dmabuf_bound
            && self.linux_dmabuf_version_at_least_4
            && self.seat_global_announced
            && self.seat_bound
            && self.seat_name_observed
            && self.seat_keyboard_capability
            && self.seat_pointer_capability
            && self.keyboard_bound
            && self.pointer_bound
            && self.compositor_bound
            && self.shm_bound
            && self.shm_buffer_created
            && self.shm_buffer_attached
            && self.xdg_wm_base_bound
            && self.surface_created
            && self.xdg_toplevel_created
            && self.configure_received
            && self.configure_acked
            && self.surface_committed
            && self.inserted_wayland_clients >= 1
            && self.wayland_dispatch_count >= 3
            && self.calloop_dispatch_count >= 3
            && self.input_sources_ready
            && self.input_source_count >= 2
            && self.input_event_loop_dispatch_count >= 3
            && self.input_seat_ready
            && self.input_keyboard_handle_ready
            && self.input_pointer_handle_ready
            && self.input_seat_dispatch_count >= 5
            && self.input_keyboard_dispatch_count >= 2
            && self.input_pointer_dispatch_count >= 3
            && self.surface_commit_count >= 1
            && self.xdg_toplevel_count >= 1
            && self.title_changed_count >= 1
            && self.app_id_changed_count >= 1
            && self.title_matched
            && self.app_id_matched
            && !self.observed_title.is_empty()
            && !self.observed_app_id.is_empty()
            && self.shm_buffer_commit_count >= 1
            && self.shm_buffer_width == 320
            && self.shm_buffer_height == 240
            && self.shm_buffer_pixels == 320 * 240
            && self.policy_window_mapped
            && self.policy_app_id_preserved
            && self.policy_focused_after_map
            && self.policy_geometry_preserved
            && self.policy_windows == 1
            && self.policy_backend_surface_presented
            && self.policy_presented_pixels == self.shm_buffer_pixels
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RealShmFrameSmoke {
    runtime_backend: &'static str,
    real_wayland_client: bool,
    real_wayland_metadata: bool,
    real_shm_pixels_captured: bool,
    real_shm_pixels_composited: bool,
    real_client_pixel_samples_verified: bool,
    policy_window_from_real_surface: bool,
    policy_app_id_preserved: bool,
    policy_geometry_preserved: bool,
    frame_ppm_written: bool,
    frame_width: u64,
    frame_height: u64,
    client_x: u64,
    client_y: u64,
    client_width: u64,
    client_height: u64,
    shm_stride: u64,
    source_pixel_count: u64,
    composited_pixels: u64,
    frame_ppm_bytes: u64,
    frame_checksum: u64,
    source_top_left_red: u64,
    source_top_left_green: u64,
    source_top_left_blue: u64,
    source_center_red: u64,
    source_center_green: u64,
    source_center_blue: u64,
    source_bottom_right_red: u64,
    source_bottom_right_green: u64,
    source_bottom_right_blue: u64,
    frame_ppm_path: String,
}

impl RealShmFrameSmoke {
    fn passed(&self) -> bool {
        self.runtime_backend == "smithay-compositor-runtime"
            && self.real_wayland_client
            && self.real_wayland_metadata
            && self.real_shm_pixels_captured
            && self.real_shm_pixels_composited
            && self.real_client_pixel_samples_verified
            && self.policy_window_from_real_surface
            && self.policy_app_id_preserved
            && self.policy_geometry_preserved
            && self.frame_ppm_written
            && self.frame_width >= 320
            && self.frame_height >= 240
            && self.client_width == 320
            && self.client_height == 240
            && self.source_pixel_count == 320 * 240
            && self.composited_pixels == self.source_pixel_count
            && self.frame_ppm_bytes > self.source_pixel_count
            && self.frame_checksum > 0
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SmithayPolicySurfaceSmoke {
    window_mapped: bool,
    app_id_preserved: bool,
    focused_after_map: bool,
    geometry_preserved: bool,
    windows: u64,
    backend_surface_presented: bool,
    presented_pixels: u64,
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn run_smithay_client_smoke_for_config(config: &RunConfig) -> Result<SmithayClientSmoke, String> {
    if config.runtime != RuntimeKind::Smithay {
        return Err(String::from(
            "Smithay Wayland client smoke requires --runtime=smithay",
        ));
    }

    let mut runtime = SmithayCompositorRuntime::try_new().map_err(|error| error.to_string())?;
    let runtime_backend = runtime.runtime_name();
    let report = runtime
        .run_wayland_client_smoke()
        .map_err(|error| error.to_string())?;
    let policy = map_smithay_smoke_into_policy(&mut runtime, &report);

    Ok(SmithayClientSmoke {
        runtime_backend,
        smithay_protocol_globals: report.protocol_globals,
        registry_global_count: report.registry_global_count,
        registry_announced: report.registry_announced,
        mvp_protocol_globals: report.mvp_protocol_globals,
        mvp_protocol_globals_announced: report.mvp_protocol_globals_announced,
        wl_output_bound: report.wl_output_bound,
        xdg_output_manager_bound: report.xdg_output_manager_bound,
        viewporter_bound: report.viewporter_bound,
        presentation_bound: report.presentation_bound,
        linux_dmabuf_bound: report.linux_dmabuf_bound,
        linux_dmabuf_version: report.linux_dmabuf_version,
        linux_dmabuf_version_at_least_4: report.linux_dmabuf_version_at_least_4,
        seat_global_announced: report.seat_global_announced,
        seat_bound: report.seat_bound,
        seat_name_observed: report.seat_name_observed,
        seat_keyboard_capability: report.seat_keyboard_capability,
        seat_pointer_capability: report.seat_pointer_capability,
        keyboard_bound: report.keyboard_bound,
        pointer_bound: report.pointer_bound,
        compositor_bound: report.compositor_bound,
        shm_bound: report.shm_bound,
        shm_buffer_created: report.shm_buffer_created,
        shm_buffer_attached: report.shm_buffer_attached,
        xdg_wm_base_bound: report.xdg_wm_base_bound,
        surface_created: report.surface_created,
        xdg_toplevel_created: report.xdg_toplevel_created,
        configure_received: report.configure_received,
        configure_acked: report.configure_acked,
        surface_committed: report.surface_committed,
        inserted_wayland_clients: report.inserted_wayland_clients,
        wayland_dispatch_count: report.wayland_dispatch_count,
        calloop_dispatch_count: report.calloop_dispatch_count,
        input_sources_ready: report.input_sources_ready,
        input_source_count: report.input_source_count,
        input_event_loop_dispatch_count: report.input_event_loop_dispatch_count,
        input_seat_ready: report.input_seat_ready,
        input_keyboard_handle_ready: report.input_keyboard_handle_ready,
        input_pointer_handle_ready: report.input_pointer_handle_ready,
        input_seat_dispatch_count: report.input_seat_dispatch_count,
        input_keyboard_dispatch_count: report.input_keyboard_dispatch_count,
        input_pointer_dispatch_count: report.input_pointer_dispatch_count,
        input_event_counters: report.input_event_counters,
        surface_commit_count: report.surface_commit_count,
        xdg_toplevel_count: report.xdg_toplevel_count,
        xdg_popup_count: report.xdg_popup_count,
        title_changed_count: report.title_changed_count,
        app_id_changed_count: report.app_id_changed_count,
        observed_title: report.observed_title,
        observed_app_id: report.observed_app_id,
        title_matched: report.title_matched,
        app_id_matched: report.app_id_matched,
        shm_buffer_commit_count: report.shm_buffer_commit_count,
        shm_buffer_width: report.shm_buffer_width,
        shm_buffer_height: report.shm_buffer_height,
        shm_buffer_pixels: report.shm_buffer_pixels,
        policy_window_mapped: policy.window_mapped,
        policy_app_id_preserved: policy.app_id_preserved,
        policy_focused_after_map: policy.focused_after_map,
        policy_geometry_preserved: policy.geometry_preserved,
        policy_windows: policy.windows,
        policy_backend_surface_presented: policy.backend_surface_presented,
        policy_presented_pixels: policy.presented_pixels,
    })
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn map_smithay_smoke_into_policy(
    runtime: &mut SmithayCompositorRuntime,
    report: &backlit_compositor_backend::SmithayWaylandClientSmokeReport,
) -> SmithayPolicySurfaceSmoke {
    let width = report.shm_buffer_width as i32;
    let height = report.shm_buffer_height as i32;
    let mut manager = SurfaceManager::new(OutputLayout::new(800, 520, 42));
    let surface = if report.passed() && width > 0 && height > 0 {
        map_scripted_app_toplevel(
            &mut manager,
            report.observed_title.as_str(),
            report.observed_app_id.as_str(),
            width,
            height,
        )
        .ok()
    } else {
        None
    };
    let window_id = surface.and_then(|surface| manager.surface(surface)?.window_id);
    let policy_window = window_id.and_then(|window_id| manager.policy().window(window_id));
    let backend_client = runtime.connect_client("real-wayland-policy-mirror");
    let backend_surface_presented = if report.passed() && width > 0 && height > 0 {
        runtime
            .submit_surface(
                backend_client,
                report.observed_title.as_str(),
                report.shm_buffer_width as u32,
                report.shm_buffer_height as u32,
            )
            .is_ok()
    } else {
        false
    };
    let frame = runtime.present();

    SmithayPolicySurfaceSmoke {
        window_mapped: window_id.is_some(),
        app_id_preserved: policy_window.and_then(|window| window.app_id.as_deref())
            == Some(report.observed_app_id.as_str()),
        focused_after_map: window_id
            .map(|window_id| manager.policy().focused() == Some(window_id))
            .unwrap_or(false),
        geometry_preserved: policy_window
            .map(|window| window.geometry.width == width && window.geometry.height == height)
            .unwrap_or(false),
        windows: manager.policy().windows().len() as u64,
        backend_surface_presented,
        presented_pixels: frame.total_pixels,
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn run_smithay_real_shm_frame_for_config(config: &RunConfig) -> Result<RealShmFrameSmoke, String> {
    if config.runtime != RuntimeKind::Smithay {
        return Err(String::from(
            "Smithay real SHM frame requires --runtime=smithay",
        ));
    }

    let mut runtime = SmithayCompositorRuntime::try_new().map_err(|error| error.to_string())?;
    let runtime_backend = runtime.runtime_name();
    let capture = runtime
        .run_real_shm_frame_capture()
        .map_err(|error| error.to_string())?;
    compose_real_shm_capture(runtime_backend, &capture, config)
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn compose_real_shm_capture(
    runtime_backend: &'static str,
    capture: &SmithayRealShmFrameCapture,
    config: &RunConfig,
) -> Result<RealShmFrameSmoke, String> {
    let frame_width = 800u32;
    let frame_height = 520u32;
    let layout = OutputLayout::new(frame_width as i32, frame_height as i32, 42);
    let mut manager = SurfaceManager::new(layout);
    let surface = map_scripted_app_toplevel(
        &mut manager,
        capture.surface.title.as_str(),
        capture.surface.app_id.as_str(),
        capture.surface.width as i32,
        capture.surface.height as i32,
    )?;
    let window_id = manager
        .surface(surface)
        .and_then(|surface| surface.window_id)
        .ok_or_else(|| String::from("real-shm-frame:missing-policy-window"))?;
    let window = manager
        .policy()
        .window(window_id)
        .cloned()
        .ok_or_else(|| String::from("real-shm-frame:missing-window-geometry"))?;
    let mut frame = render_policy_gui(frame_width, frame_height, manager.policy(), layout);
    let mut composited_pixels = 0u64;

    for y in 0..capture.surface.height {
        for x in 0..capture.surface.width {
            let Some(pixel) = capture.surface.pixel(x, y) else {
                continue;
            };
            let frame_x = window.geometry.x + x as i32;
            let frame_y = window.geometry.y + y as i32;
            if frame_x < 0 || frame_y < 0 {
                continue;
            }
            if frame.set_pixel(frame_x as u32, frame_y as u32, real_shm_color(pixel)) {
                composited_pixels += 1;
            }
        }
    }

    let source_sample_points = capture.surface.sample_coordinates;
    let frame_top_left =
        frame_real_shm_sample(&frame, window.geometry, source_sample_points.top_left);
    let frame_center = frame_real_shm_sample(&frame, window.geometry, source_sample_points.center);
    let frame_bottom_right =
        frame_real_shm_sample(&frame, window.geometry, source_sample_points.bottom_right);
    let real_client_pixel_samples_verified = capture.surface.samples_verified()
        && frame_color_matches_real_pixel(frame_top_left, capture.surface.samples.top_left)
        && frame_color_matches_real_pixel(frame_center, capture.surface.samples.center)
        && frame_color_matches_real_pixel(frame_bottom_right, capture.surface.samples.bottom_right);

    let output_path = config
        .smithay_real_shm_frame_output
        .as_deref()
        .unwrap_or("target/smithay-real-shm-frame/backlit-real-shm-frame.ppm");
    let frame_ppm_written = frame.write_ppm(output_path).is_ok();
    let frame_ppm_bytes = if frame_ppm_written {
        fs::metadata(output_path)
            .map(|metadata| metadata.len())
            .unwrap_or_default()
    } else {
        0
    };

    Ok(RealShmFrameSmoke {
        runtime_backend,
        real_wayland_client: capture.smoke.passed(),
        real_wayland_metadata: capture.smoke.title_matched
            && capture.smoke.app_id_matched
            && capture.surface.title == capture.smoke.observed_title
            && capture.surface.app_id == capture.smoke.observed_app_id,
        real_shm_pixels_captured: capture.surface.width > 0
            && capture.surface.height > 0
            && capture.surface.pixels.len()
                == capture.surface.width.saturating_mul(capture.surface.height) as usize
            && capture.surface.stride >= capture.surface.width.saturating_mul(4)
            && capture.surface.format == "ARGB8888",
        real_shm_pixels_composited: composited_pixels
            == capture.surface.width.saturating_mul(capture.surface.height) as u64,
        real_client_pixel_samples_verified,
        policy_window_from_real_surface: window.title == capture.surface.title
            && manager.policy().focused() == Some(window.id),
        policy_app_id_preserved: window.app_id.as_deref() == Some(capture.surface.app_id.as_str()),
        policy_geometry_preserved: window.geometry.width == capture.surface.width as i32
            && window.geometry.height == capture.surface.height as i32,
        frame_ppm_written: frame_ppm_written && frame_ppm_bytes > 0,
        frame_width: frame.width() as u64,
        frame_height: frame.height() as u64,
        client_x: window.geometry.x.max(0) as u64,
        client_y: window.geometry.y.max(0) as u64,
        client_width: capture.surface.width as u64,
        client_height: capture.surface.height as u64,
        shm_stride: capture.surface.stride as u64,
        source_pixel_count: capture.surface.pixels.len() as u64,
        composited_pixels,
        frame_ppm_bytes,
        frame_checksum: frame.checksum(),
        source_top_left_red: capture.surface.samples.top_left.red as u64,
        source_top_left_green: capture.surface.samples.top_left.green as u64,
        source_top_left_blue: capture.surface.samples.top_left.blue as u64,
        source_center_red: capture.surface.samples.center.red as u64,
        source_center_green: capture.surface.samples.center.green as u64,
        source_center_blue: capture.surface.samples.center.blue as u64,
        source_bottom_right_red: capture.surface.samples.bottom_right.red as u64,
        source_bottom_right_green: capture.surface.samples.bottom_right.green as u64,
        source_bottom_right_blue: capture.surface.samples.bottom_right.blue as u64,
        frame_ppm_path: output_path.to_string(),
    })
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn real_shm_color(pixel: RealShmPixel) -> Color {
    Color::rgb(pixel.red, pixel.green, pixel.blue)
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn frame_real_shm_sample(
    frame: &Canvas,
    geometry: backlit_window_policy::Rect,
    sample: (u32, u32),
) -> Option<Color> {
    let x = geometry.x + sample.0 as i32;
    let y = geometry.y + sample.1 as i32;
    if x < 0 || y < 0 {
        return None;
    }

    frame.pixel(x as u32, y as u32)
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn frame_color_matches_real_pixel(frame_color: Option<Color>, pixel: RealShmPixel) -> bool {
    frame_color == Some(real_shm_color(pixel))
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn run_smithay_client_smoke_for_config(_config: &RunConfig) -> Result<SmithayClientSmoke, String> {
    Err(String::from(
        "Smithay Wayland client smoke requires Linux and the smithay-backend feature",
    ))
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn run_smithay_real_shm_frame_for_config(_config: &RunConfig) -> Result<RealShmFrameSmoke, String> {
    Err(String::from(
        "Smithay real SHM frame requires Linux and the smithay-backend feature",
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScriptedClientRuntime {
    runtime_backend: &'static str,
    runtime_trait: bool,
    inserted_wayland_clients: u64,
    wayland_dispatch_count: u64,
    calloop_dispatch_count: u64,
    input_sources_ready: bool,
    input_source_count: u64,
    input_event_loop_dispatch_count: u64,
    input_seat_ready: bool,
    input_keyboard_handle_ready: bool,
    input_pointer_handle_ready: bool,
    input_seat_dispatch_count: u64,
    input_keyboard_dispatch_count: u64,
    input_pointer_dispatch_count: u64,
    input_event_counters: InputEventCounters,
    smithay_protocol_globals: u64,
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
        self.runtime_trait
            && !self.runtime_backend.is_empty()
            && self.smithay_event_loop_runtime_ok()
            && self.smithay_protocol_globals_ok()
            && self.client_connected
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

    fn smithay_event_loop_runtime_ok(self) -> bool {
        self.runtime_backend != "smithay-compositor-runtime"
            || (self.inserted_wayland_clients >= 1
                && self.wayland_dispatch_count >= self.frames
                && self.calloop_dispatch_count >= self.frames
                && self.input_sources_ready
                && self.input_source_count >= 2
                && self.input_event_loop_dispatch_count >= self.frames
                && self.input_seat_ready
                && self.input_keyboard_handle_ready
                && self.input_pointer_handle_ready
                && self.input_seat_dispatch_count >= 5
                && self.input_keyboard_dispatch_count >= 2
                && self.input_pointer_dispatch_count >= 3)
    }

    fn smithay_protocol_globals_ok(self) -> bool {
        self.runtime_backend != "smithay-compositor-runtime" || self.smithay_protocol_globals >= 10
    }
}

fn run_scripted_client_runtime(
    policy_preview_path: Option<&str>,
) -> Result<ScriptedClientRuntime, String> {
    run_scripted_client_runtime_with_backend(HeadlessCompositor::default(), policy_preview_path)
}

fn run_scripted_client_runtime_for_config(
    config: &RunConfig,
    policy_preview_path: Option<&str>,
) -> Result<ScriptedClientRuntime, String> {
    match config.runtime {
        RuntimeKind::Headless => run_scripted_client_runtime(policy_preview_path),
        RuntimeKind::Smithay => run_scripted_client_runtime_with_smithay(policy_preview_path),
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn run_scripted_client_runtime_with_smithay(
    policy_preview_path: Option<&str>,
) -> Result<ScriptedClientRuntime, String> {
    let runtime = SmithayCompositorRuntime::try_new().map_err(|error| error.to_string())?;
    run_scripted_client_runtime_with_backend(runtime, policy_preview_path)
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn run_scripted_client_runtime_with_smithay(
    _policy_preview_path: Option<&str>,
) -> Result<ScriptedClientRuntime, String> {
    Err(String::from(
        "smithay runtime requires Linux and the smithay-backend feature",
    ))
}

fn run_scripted_client_runtime_with_backend<B: CompositorRuntime>(
    mut backend: B,
    policy_preview_path: Option<&str>,
) -> Result<ScriptedClientRuntime, String> {
    let runtime_backend = backend.runtime_name();
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
        runtime_backend,
        runtime_trait: true,
        inserted_wayland_clients: backend.inserted_wayland_clients(),
        wayland_dispatch_count: backend.wayland_dispatch_count(),
        calloop_dispatch_count: backend.calloop_dispatch_count(),
        input_sources_ready: backend.input_sources_ready(),
        input_source_count: backend.input_source_count(),
        input_event_loop_dispatch_count: backend.input_event_loop_dispatch_count(),
        input_seat_ready: backend.input_seat_ready(),
        input_keyboard_handle_ready: backend.input_keyboard_handle_ready(),
        input_pointer_handle_ready: backend.input_pointer_handle_ready(),
        input_seat_dispatch_count: backend.input_seat_dispatch_count(),
        input_keyboard_dispatch_count: backend.input_keyboard_dispatch_count(),
        input_pointer_dispatch_count: backend.input_pointer_dispatch_count(),
        input_event_counters: backend.input_event_counters(),
        smithay_protocol_globals: backend.smithay_protocol_global_count(),
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
    configure_and_commit_scripted_surface(manager, surface, title, width, height)
}

fn map_scripted_app_toplevel(
    manager: &mut SurfaceManager,
    title: &str,
    app_id: &str,
    width: i32,
    height: i32,
) -> Result<backlit_surface::SurfaceId, String> {
    let surface = manager.create_app_toplevel(title, Some(app_id), (width, height));
    configure_and_commit_scripted_surface(manager, surface, title, width, height)
}

fn configure_and_commit_scripted_surface(
    manager: &mut SurfaceManager,
    surface: backlit_surface::SurfaceId,
    title: &str,
    width: i32,
    height: i32,
) -> Result<backlit_surface::SurfaceId, String> {
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
    runtime_backend: &'static str,
    runtime_trait: bool,
    inserted_wayland_clients: u64,
    wayland_dispatch_count: u64,
    calloop_dispatch_count: u64,
    input_sources_ready: bool,
    input_source_count: u64,
    input_event_loop_dispatch_count: u64,
    input_seat_ready: bool,
    input_keyboard_handle_ready: bool,
    input_pointer_handle_ready: bool,
    input_seat_dispatch_count: u64,
    input_keyboard_dispatch_count: u64,
    input_pointer_dispatch_count: u64,
    input_event_counters: InputEventCounters,
    smithay_protocol_globals: u64,
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
        self.runtime_trait
            && !self.runtime_backend.is_empty()
            && self.smithay_event_loop_runtime_ok()
            && self.smithay_protocol_globals_ok()
            && self.accepting_clients
            && self.bootstrap_client_connected
            && self.bootstrap_surface_presented
            && self.clients == 1
            && self.surfaces == 1
            && self.frames == 1
            && self.damaged_surfaces == 1
            && self.presented_pixels == 1
    }

    fn smithay_event_loop_runtime_ok(self) -> bool {
        self.runtime_backend != "smithay-compositor-runtime"
            || (self.inserted_wayland_clients >= 1
                && self.wayland_dispatch_count >= self.frames
                && self.calloop_dispatch_count >= self.frames
                && self.input_sources_ready
                && self.input_source_count >= 2
                && self.input_event_loop_dispatch_count >= self.frames
                && self.input_seat_ready
                && self.input_keyboard_handle_ready
                && self.input_pointer_handle_ready
                && self.input_seat_dispatch_count >= 5
                && self.input_keyboard_dispatch_count >= 2
                && self.input_pointer_dispatch_count >= 3)
    }

    fn smithay_protocol_globals_ok(self) -> bool {
        self.runtime_backend != "smithay-compositor-runtime" || self.smithay_protocol_globals >= 10
    }
}

fn run_service_ready() -> CompositorReadyReport {
    run_service_ready_with_backend(HeadlessCompositor::default())
}

fn run_service_ready_for_config(config: &RunConfig) -> Result<CompositorReadyReport, String> {
    match config.runtime {
        RuntimeKind::Headless => Ok(run_service_ready()),
        RuntimeKind::Smithay => run_service_ready_with_smithay(),
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn run_service_ready_with_smithay() -> Result<CompositorReadyReport, String> {
    let runtime = SmithayCompositorRuntime::try_new().map_err(|error| error.to_string())?;
    Ok(run_service_ready_with_backend(runtime))
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn run_service_ready_with_smithay() -> Result<CompositorReadyReport, String> {
    Err(String::from(
        "smithay runtime requires Linux and the smithay-backend feature",
    ))
}

fn run_service_ready_with_backend<B: CompositorRuntime>(mut backend: B) -> CompositorReadyReport {
    let runtime_backend = backend.runtime_name();
    let client = backend.connect_client("backlit-session-service");
    let bootstrap_surface_presented = backend
        .submit_surface(client, "backlit-bootstrap", 1, 1)
        .is_ok();
    let frame = backend.present();

    CompositorReadyReport {
        runtime_backend,
        runtime_trait: true,
        inserted_wayland_clients: backend.inserted_wayland_clients(),
        wayland_dispatch_count: backend.wayland_dispatch_count(),
        calloop_dispatch_count: backend.calloop_dispatch_count(),
        input_sources_ready: backend.input_sources_ready(),
        input_source_count: backend.input_source_count(),
        input_event_loop_dispatch_count: backend.input_event_loop_dispatch_count(),
        input_seat_ready: backend.input_seat_ready(),
        input_keyboard_handle_ready: backend.input_keyboard_handle_ready(),
        input_pointer_handle_ready: backend.input_pointer_handle_ready(),
        input_seat_dispatch_count: backend.input_seat_dispatch_count(),
        input_keyboard_dispatch_count: backend.input_keyboard_dispatch_count(),
        input_pointer_dispatch_count: backend.input_pointer_dispatch_count(),
        input_event_counters: backend.input_event_counters(),
        smithay_protocol_globals: backend.smithay_protocol_global_count(),
        accepting_clients: backend.client_count() > 0,
        bootstrap_client_connected: backend.client_count() == 1,
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
    run_compositor_surface_smoke_with_backend(HeadlessCompositor::default())
}

fn run_compositor_surface_smoke_with_backend<B: CompositorRuntime>(
    mut backend: B,
) -> CompositorSurfaceSmoke {
    let mut manager = SurfaceManager::new(OutputLayout::new(800, 520, 42));
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
    run_direct_scanout_smoke_with_backends(
        HeadlessCompositor::default(),
        HeadlessCompositor::default(),
    )
}

fn run_direct_scanout_smoke_with_backends<B: CompositorRuntime, S: CompositorRuntime>(
    mut compositor: B,
    mut shm_compositor: S,
) -> DirectScanoutSmoke {
    let failed = DirectScanoutSmoke {
        eligible: false,
        dmabuf: false,
        fullscreen: false,
        overlay_blocked: false,
        shm_blocked: false,
    };
    let client = compositor.connect_client("scanout-video-client");
    let Ok(video) = compositor.submit_surface_with_options(
        client,
        "fullscreen-video",
        1920,
        1080,
        SurfaceOptions::dmabuf_fullscreen(),
    ) else {
        return failed;
    };
    let Ok(eligible) = compositor.direct_scanout_candidate(video, 1920, 1080) else {
        return failed;
    };

    let panel_presented = compositor
        .submit_surface(client, "panel-overlay", 1920, 42)
        .is_ok();
    let overlay_blocked = panel_presented
        && compositor
            .direct_scanout_candidate(video, 1920, 1080)
            .map(|report| !report.eligible && report.reason == "occluded-by-other-surface")
            .unwrap_or(false);

    let client = shm_compositor.connect_client("scanout-shm-client");
    let Ok(shm_video) = shm_compositor.submit_surface_with_options(
        client,
        "fullscreen-shm-video",
        1920,
        1080,
        SurfaceOptions {
            fullscreen: true,
            ..SurfaceOptions::default()
        },
    ) else {
        return DirectScanoutSmoke {
            eligible: eligible.eligible,
            dmabuf: eligible.buffer_kind.as_str() == "dmabuf",
            fullscreen: eligible.reason == "eligible",
            overlay_blocked,
            shm_blocked: false,
        };
    };
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

fn emit_backend_launch_plan(config: &RunConfig, plan: &BackendLaunchPlan) {
    let primary_drm_card = plan.primary_drm_card.as_deref().unwrap_or("");
    let primary_drm_render_node = plan.primary_drm_render_node.as_deref().unwrap_or("");
    let primary_input_event = plan.primary_input_event.as_deref().unwrap_or("");
    let session_id = plan.session_id.as_deref().unwrap_or("");
    let seat = plan.seat.as_deref().unwrap_or("");
    let session_type = plan.session_type.as_deref().unwrap_or("");

    emit(
        "compositor.backend_launch_plan",
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

fn backend_launch_plan_for_config(
    config: &RunConfig,
    report: &BackendPreflightReport,
    environment: &BackendPreflightEnvironment,
) -> BackendLaunchPlan {
    let mut plan = backend_launch_plan(config.backend, report, environment);
    if config.backend == BackendKind::Drm && config.runtime == RuntimeKind::Smithay {
        plan.implementation = "smithay-compositor-runtime";
    }
    plan
}

fn emit_socket_bound(config: &RunConfig, socket: &BoundSessionSocket) {
    let path = socket.path_string();
    emit(
        "compositor.socket_bound",
        config,
        &[
            ("socket_name", FieldValue::Str(socket.socket_name.as_str())),
            ("runtime_dir", FieldValue::Str(socket.runtime_dir.as_str())),
            ("socket_path", FieldValue::Str(path.as_str())),
            (
                "stale_socket_removed",
                FieldValue::Bool(socket.stale_socket_removed),
            ),
        ],
    );
}

fn emit_socket_unavailable(config: &RunConfig) {
    let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_default();
    emit(
        "compositor.socket_unavailable",
        config,
        &[
            ("runtime_dir", FieldValue::Str(runtime_dir.as_str())),
            (
                "runtime_dir_present",
                FieldValue::Bool(!runtime_dir.trim().is_empty()),
            ),
        ],
    );
}

fn emit_socket_unbound(config: &RunConfig, path: &str, removed: bool) {
    emit(
        "compositor.socket_unbound",
        config,
        &[
            ("socket_path", FieldValue::Str(path)),
            ("removed", FieldValue::Bool(removed)),
        ],
    );
}

fn emit_socket_client(config: &RunConfig, runtime_backend: &str, report: &SocketClientReport) {
    emit(
        "compositor.socket_client",
        config,
        &[
            ("runtime_backend", FieldValue::Str(runtime_backend)),
            ("message_valid", FieldValue::Bool(report.message_valid)),
            ("action", FieldValue::Str(report.action.as_str())),
            ("title", FieldValue::Str(report.title.as_str())),
            ("app_id", FieldValue::Str(report.app_id.as_str())),
            ("width", FieldValue::U64(report.width as u64)),
            ("height", FieldValue::U64(report.height as u64)),
            (
                "backend_surface_presented",
                FieldValue::Bool(report.backend_surface_presented),
            ),
            (
                "backend_surface_damaged",
                FieldValue::Bool(report.backend_surface_damaged),
            ),
            (
                "backend_surface_closed",
                FieldValue::Bool(report.backend_surface_closed),
            ),
            (
                "policy_window_mapped",
                FieldValue::Bool(report.policy_window_mapped),
            ),
            (
                "policy_app_id_preserved",
                FieldValue::Bool(report.policy_app_id_preserved),
            ),
            (
                "policy_window_moved",
                FieldValue::Bool(report.policy_window_moved),
            ),
            (
                "policy_window_resized",
                FieldValue::Bool(report.policy_window_resized),
            ),
            (
                "policy_window_maximized",
                FieldValue::Bool(report.policy_window_maximized),
            ),
            (
                "policy_window_fullscreen",
                FieldValue::Bool(report.policy_window_fullscreen),
            ),
            (
                "policy_window_closed",
                FieldValue::Bool(report.policy_window_closed),
            ),
            (
                "client_disconnected",
                FieldValue::Bool(report.client_disconnected),
            ),
            ("frame", FieldValue::U64(report.frame)),
            ("damaged_surfaces", FieldValue::U64(report.damaged_surfaces)),
            ("backend_clients", FieldValue::U64(report.backend_clients)),
            ("backend_surfaces", FieldValue::U64(report.backend_surfaces)),
            (
                "inserted_wayland_clients",
                FieldValue::U64(report.inserted_wayland_clients),
            ),
            (
                "wayland_dispatch_count",
                FieldValue::U64(report.wayland_dispatch_count),
            ),
            (
                "calloop_dispatch_count",
                FieldValue::U64(report.calloop_dispatch_count),
            ),
            (
                "input_sources_ready",
                FieldValue::Bool(report.input_sources_ready),
            ),
            (
                "input_source_count",
                FieldValue::U64(report.input_source_count),
            ),
            (
                "input_event_loop_dispatch_count",
                FieldValue::U64(report.input_event_loop_dispatch_count),
            ),
            (
                "input_seat_ready",
                FieldValue::Bool(report.input_seat_ready),
            ),
            (
                "input_keyboard_handle_ready",
                FieldValue::Bool(report.input_keyboard_handle_ready),
            ),
            (
                "input_pointer_handle_ready",
                FieldValue::Bool(report.input_pointer_handle_ready),
            ),
            (
                "input_seat_dispatch_count",
                FieldValue::U64(report.input_seat_dispatch_count),
            ),
            (
                "input_keyboard_dispatch_count",
                FieldValue::U64(report.input_keyboard_dispatch_count),
            ),
            (
                "input_pointer_dispatch_count",
                FieldValue::U64(report.input_pointer_dispatch_count),
            ),
            (
                "input_event_count",
                FieldValue::U64(report.input_event_counters.total),
            ),
            (
                "input_device_event_count",
                FieldValue::U64(report.input_event_counters.device),
            ),
            (
                "input_keyboard_event_count",
                FieldValue::U64(report.input_event_counters.keyboard),
            ),
            (
                "input_pointer_event_count",
                FieldValue::U64(report.input_event_counters.pointer),
            ),
            (
                "input_touch_event_count",
                FieldValue::U64(report.input_event_counters.touch),
            ),
            (
                "input_gesture_event_count",
                FieldValue::U64(report.input_event_counters.gesture),
            ),
            (
                "input_tablet_event_count",
                FieldValue::U64(report.input_event_counters.tablet),
            ),
            (
                "input_switch_event_count",
                FieldValue::U64(report.input_event_counters.switch),
            ),
            (
                "input_special_event_count",
                FieldValue::U64(report.input_event_counters.special),
            ),
            (
                "smithay_protocol_globals",
                FieldValue::U64(report.smithay_protocol_globals),
            ),
            ("policy_windows", FieldValue::U64(report.policy_windows)),
            ("visible_windows", FieldValue::U64(report.visible_windows)),
            ("policy_state", FieldValue::Str(report.policy_state)),
            ("policy_x", FieldValue::U64(report.policy_x.max(0) as u64)),
            ("policy_y", FieldValue::U64(report.policy_y.max(0) as u64)),
            (
                "policy_width",
                FieldValue::U64(report.policy_width.max(0) as u64),
            ),
            (
                "policy_height",
                FieldValue::U64(report.policy_height.max(0) as u64),
            ),
            ("focused_window", FieldValue::Bool(report.focused_window)),
            (
                "focused_title",
                FieldValue::Str(report.focused_title.as_str()),
            ),
            (
                "focused_app_id",
                FieldValue::Str(report.focused_app_id.as_str()),
            ),
        ],
    );
}

fn emit_drm_first_present_probe(config: &RunConfig, probe: &SmithayRuntimeProbe) {
    let first_present_failure = probe.kms_first_present_failure.as_deref().unwrap_or("");

    emit(
        "compositor.drm_first_present_probe",
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
                "libinput_event_count",
                FieldValue::U64(probe.libinput_event_count),
            ),
            (
                "libinput_device_event_count",
                FieldValue::U64(probe.libinput_event_counters.device),
            ),
            (
                "libinput_keyboard_event_count",
                FieldValue::U64(probe.libinput_event_counters.keyboard),
            ),
            (
                "libinput_pointer_event_count",
                FieldValue::U64(probe.libinput_event_counters.pointer),
            ),
            (
                "libinput_touch_event_count",
                FieldValue::U64(probe.libinput_event_counters.touch),
            ),
            (
                "libinput_gesture_event_count",
                FieldValue::U64(probe.libinput_event_counters.gesture),
            ),
            (
                "libinput_tablet_event_count",
                FieldValue::U64(probe.libinput_event_counters.tablet),
            ),
            (
                "libinput_switch_event_count",
                FieldValue::U64(probe.libinput_event_counters.switch),
            ),
            (
                "libinput_special_event_count",
                FieldValue::U64(probe.libinput_event_counters.special),
            ),
            (
                "kms_first_present_failure",
                FieldValue::Str(first_present_failure),
            ),
        ],
    );
}

fn print_help() {
    println!(
        "\
backlit-compositor

Usage:
  backlit-compositor [--backend=headless|wayland|drm] [--runtime=headless|smithay] [--socket=backlit-0] [--smoke-test] [--scripted-client] [--smithay-client-smoke] [--smithay-real-shm-frame] [--drm-first-present-probe] [--scripted-client-preview=path] [--smithay-real-shm-frame-output=path] [--serve] [--serve-for-ms=1000] [--idle-probe-ms=1000]

Flags:
  --backend      Select compositor backend. Defaults to headless.
  --runtime      Select runtime implementation. Defaults to headless.
  --socket       Wayland socket name to create or target. Defaults to backlit-0.
  --smoke-test   Run the current MVP 0 policy/metrics smoke test and exit.
  --scripted-client
                 Run a deterministic app-client lifecycle through the compositor runtime.
  --smithay-client-smoke
                 Run a real Wayland registry/surface/xdg-toplevel protocol smoke through Smithay.
  --smithay-real-shm-frame
                 Render generated wl_shm client pixels into a Backlit policy frame and verify samples.
  --drm-first-present-probe
                 Probe Smithay DRM/KMS first-present framebuffer, plane state, and commit boundary.
  --scripted-client-preview
                 Write the scripted client policy preview frame to a PPM file.
  --smithay-real-shm-frame-output
                 Write the real SHM Backlit frame to a PPM file.
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
    use super::{
        backend_launch_plan_for_config, run_compositor_surface_smoke,
        run_compositor_surface_smoke_with_backend, run_direct_scanout_smoke_with_backends,
        run_scripted_client_runtime, run_service_ready_with_backend, DemoSocketAction,
        DemoSocketCommand, SocketClientRuntime,
    };
    use backlit_compositor_backend::{
        BackendKind, BackendPreflightEnvironment, HeadlessCompositor, RunConfig, RuntimeKind,
    };
    use std::fs;
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn compositor_surface_smoke_maps_xdg_toplevel_into_backend_frame() {
        let report = run_compositor_surface_smoke();

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.presented_surfaces, 2);
        assert_eq!(report.presented_pixels, 640 * 480 + 240 * 160);
        assert_eq!(report.windows_after_close, 0);
    }

    #[test]
    fn compositor_surface_smoke_accepts_runtime_trait_backend() {
        let report = run_compositor_surface_smoke_with_backend(HeadlessCompositor::default());

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.presented_surfaces, 2);
    }

    #[test]
    fn direct_scanout_smoke_accepts_runtime_trait_backends() {
        let report = run_direct_scanout_smoke_with_backends(
            HeadlessCompositor::default(),
            HeadlessCompositor::default(),
        );

        assert!(report.eligible, "{report:?}");
        assert!(report.overlay_blocked, "{report:?}");
        assert!(report.shm_blocked, "{report:?}");
    }

    #[test]
    fn drm_smithay_runtime_launch_plan_names_real_runtime() {
        let config = RunConfig {
            backend: BackendKind::Drm,
            runtime: RuntimeKind::Smithay,
            ..RunConfig::default()
        };
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_drm_render_access(1, 1)
            .with_input_event_nodes(1)
            .with_input_event_access(1)
            .with_primary_drm_card("/dev/dri/card0")
            .with_primary_drm_render_node("/dev/dri/renderD128")
            .with_primary_input_event("/dev/input/event0")
            .with_active_local_session("7", "seat0", "tty");
        let report = backlit_compositor_backend::preflight_backend_with_environment(
            config.backend,
            &environment,
        );

        let plan = backend_launch_plan_for_config(&config, &report, &environment);

        assert!(plan.ready);
        assert_eq!(plan.implementation, "smithay-compositor-runtime");
        assert_eq!(plan.display_driver, "drm-kms");
        assert!(plan.uses_drm);
        assert!(plan.uses_libinput);
    }

    #[test]
    fn compositor_service_ready_accepts_client_and_presents_bootstrap_surface() {
        let report = super::run_service_ready();

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.runtime_backend, "headless-compositor");
        assert!(report.runtime_trait);
        assert!(report.accepting_clients);
        assert_eq!(report.clients, 1);
        assert_eq!(report.surfaces, 1);
        assert_eq!(report.presented_pixels, 1);
    }

    #[test]
    fn compositor_service_ready_accepts_runtime_trait_backend() {
        let report = run_service_ready_with_backend(HeadlessCompositor::default());

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.runtime_backend, "headless-compositor");
        assert!(report.runtime_trait);
    }

    #[test]
    fn scripted_client_runtime_maps_damages_and_disconnects() {
        let report = run_scripted_client_runtime(None).unwrap();

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.runtime_backend, "headless-compositor");
        assert!(report.runtime_trait);
        assert_eq!(report.surfaces_after_map, 2);
        assert_eq!(report.surfaces_after_disconnect, 0);
        assert_eq!(report.clients_after_disconnect, 0);
        assert_eq!(report.policy_windows_after_map, 2);
        assert!(report.policy_preview_verified);
    }

    #[test]
    fn service_socket_binds_accepts_connections_and_cleans_up() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let runtime_base = if cfg!(target_os = "macos") {
            PathBuf::from("/private/tmp")
        } else {
            std::env::temp_dir()
        };
        let runtime_dir = runtime_base.join(format!("blsock-{}-{unique}", std::process::id()));
        fs::create_dir_all(&runtime_dir).unwrap();

        let runtime_dir_string = runtime_dir.display().to_string();
        let mut socket = match super::bind_session_socket_in_runtime(
            "backlit-test-socket",
            Some(runtime_dir_string),
        ) {
            Ok(Some(socket)) => socket,
            Err(error) if error.contains("Operation not permitted") => {
                let _ = fs::remove_dir(&runtime_dir);
                return;
            }
            other => panic!("unexpected socket bind result: {other:?}"),
        };
        let socket_path = socket.path.clone();

        assert!(fs::metadata(&socket_path).unwrap().file_type().is_socket());
        UnixStream::connect(&socket_path).unwrap();
        assert!(socket.cleanup());
        assert!(!socket_path.exists());

        fs::remove_dir(&runtime_dir).unwrap();
    }

    #[test]
    fn demo_surface_request_parses_socket_message() {
        let request = DemoSocketCommand::parse(
            "BACKLIT_DEMO_CLIENT surface title=socket-demo app_id=org.backlit.SocketDemo width=640 height=480\n",
        )
        .unwrap();

        assert_eq!(request.action, DemoSocketAction::Surface);
        assert_eq!(request.title, "socket-demo");
        assert_eq!(request.app_id, "org.backlit.SocketDemo");
        assert_eq!(request.width, 640);
        assert_eq!(request.height, 480);

        let damage =
            DemoSocketCommand::parse("BACKLIT_DEMO_CLIENT damage app_id=org.backlit.SocketDemo\n")
                .unwrap();
        assert_eq!(damage.action, DemoSocketAction::Damage);
        assert_eq!(damage.title, "org.backlit.SocketDemo");
        assert_eq!(damage.app_id, "org.backlit.SocketDemo");
        assert_eq!(damage.width, 1);
        assert_eq!(damage.height, 1);

        let moved = DemoSocketCommand::parse(
            "BACKLIT_DEMO_CLIENT move title=socket-demo app_id=org.backlit.SocketDemo x=120 y=140\n",
        )
        .unwrap();
        assert_eq!(moved.action, DemoSocketAction::Move);
        assert_eq!(moved.x, 120);
        assert_eq!(moved.y, 140);

        let resized = DemoSocketCommand::parse(
            "BACKLIT_DEMO_CLIENT resize title=socket-demo app_id=org.backlit.SocketDemo width=960 height=620\n",
        )
        .unwrap();
        assert_eq!(resized.action, DemoSocketAction::Resize);
        assert_eq!(resized.width, 960);
        assert_eq!(resized.height, 620);

        let close =
            DemoSocketCommand::parse("BACKLIT_DEMO_CLIENT close title=socket-demo\n").unwrap();
        assert_eq!(close.action, DemoSocketAction::Close);
        assert_eq!(close.title, "socket-demo");
        assert_eq!(close.app_id, "socket-demo");

        let legacy = DemoSocketCommand::parse(
            "BACKLIT_DEMO_CLIENT surface title=legacy-demo width=320 height=240\n",
        )
        .unwrap();
        assert_eq!(legacy.app_id, "legacy-demo");

        assert!(DemoSocketCommand::parse("garbage").is_none());
    }

    #[test]
    fn socket_client_runtime_preserves_app_id_in_policy_window() {
        let mut runtime = SocketClientRuntime::new();
        let reports = runtime.handle_stream(
            "BACKLIT_DEMO_CLIENT surface title=socket-demo app_id=org.backlit.SocketDemo width=640 height=480\n",
        );
        let report = &reports[0];

        assert!(report.message_valid);
        assert_eq!(report.action, DemoSocketAction::Surface);
        assert!(report.backend_surface_presented);
        assert!(report.policy_window_mapped);
        assert!(report.policy_app_id_preserved);
        assert_eq!(report.app_id, "org.backlit.SocketDemo");
        assert_eq!(report.backend_clients, 1);
        assert_eq!(report.policy_windows, 1);
        assert_eq!(report.visible_windows, 1);
        assert!(report.focused_window);
        assert_eq!(report.focused_title, "socket-demo");
        assert_eq!(report.focused_app_id, "org.backlit.SocketDemo");
    }

    #[test]
    fn socket_client_runtime_accepts_runtime_trait_backend() {
        let mut runtime = SocketClientRuntime::with_backend(HeadlessCompositor::default());
        let reports = runtime.handle_stream(
            "BACKLIT_DEMO_CLIENT surface title=socket-demo app_id=org.backlit.SocketDemo width=640 height=480\n",
        );
        let report = &reports[0];

        assert!(report.backend_surface_presented);
        assert_eq!(report.backend_clients, 1);
        assert_eq!(report.backend_surfaces, 1);
        assert!(report.policy_app_id_preserved);
    }

    #[test]
    fn socket_client_runtime_damages_and_closes_policy_window() {
        let mut runtime = SocketClientRuntime::new();
        let reports = runtime.handle_stream(
            "\
BACKLIT_DEMO_CLIENT surface title=socket-demo app_id=org.backlit.SocketDemo width=640 height=480
BACKLIT_DEMO_CLIENT damage app_id=org.backlit.SocketDemo
BACKLIT_DEMO_CLIENT close app_id=org.backlit.SocketDemo
",
        );

        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0].action, DemoSocketAction::Surface);
        assert!(reports[0].policy_window_mapped);
        assert_eq!(reports[0].policy_windows, 1);
        assert_eq!(reports[1].action, DemoSocketAction::Damage);
        assert!(reports[1].backend_surface_damaged);
        assert_eq!(reports[1].damaged_surfaces, 1);
        assert_eq!(reports[1].policy_windows, 1);
        assert_eq!(reports[2].action, DemoSocketAction::Close);
        assert!(reports[2].backend_surface_closed);
        assert!(reports[2].policy_window_closed);
        assert!(reports[2].client_disconnected);
        assert_eq!(reports[2].backend_clients, 0);
        assert_eq!(reports[2].backend_surfaces, 0);
        assert_eq!(reports[2].policy_windows, 0);
        assert_eq!(reports[2].visible_windows, 0);
    }

    #[test]
    fn socket_client_runtime_focuses_new_window_and_falls_back_after_close() {
        let mut runtime = SocketClientRuntime::new();
        let terminal = runtime.handle_stream(
            "BACKLIT_DEMO_CLIENT surface title=socket-terminal app_id=org.backlit.SocketTerminal width=640 height=480\n",
        );
        let browser = runtime.handle_stream(
            "\
BACKLIT_DEMO_CLIENT surface title=socket-browser app_id=org.backlit.SocketBrowser width=900 height=600
BACKLIT_DEMO_CLIENT move title=socket-browser app_id=org.backlit.SocketBrowser x=120 y=140
BACKLIT_DEMO_CLIENT resize title=socket-browser app_id=org.backlit.SocketBrowser width=960 height=620
BACKLIT_DEMO_CLIENT maximize title=socket-browser app_id=org.backlit.SocketBrowser
BACKLIT_DEMO_CLIENT fullscreen title=socket-browser app_id=org.backlit.SocketBrowser
BACKLIT_DEMO_CLIENT damage app_id=org.backlit.SocketBrowser
BACKLIT_DEMO_CLIENT close app_id=org.backlit.SocketBrowser
",
        );

        assert_eq!(terminal.len(), 1);
        assert_eq!(terminal[0].action, DemoSocketAction::Surface);
        assert_eq!(terminal[0].backend_clients, 1);
        assert_eq!(terminal[0].backend_surfaces, 1);
        assert_eq!(terminal[0].policy_windows, 1);
        assert_eq!(terminal[0].focused_app_id, "org.backlit.SocketTerminal");

        assert_eq!(browser.len(), 7);
        assert_eq!(browser[0].action, DemoSocketAction::Surface);
        assert_eq!(browser[0].backend_clients, 2);
        assert_eq!(browser[0].backend_surfaces, 2);
        assert_eq!(browser[0].policy_windows, 2);
        assert_eq!(browser[0].focused_app_id, "org.backlit.SocketBrowser");
        assert_eq!(browser[1].action, DemoSocketAction::Move);
        assert!(browser[1].policy_window_moved);
        assert_eq!(browser[1].policy_state, "normal");
        assert_eq!(browser[1].policy_x, 120);
        assert_eq!(browser[1].policy_y, 140);
        assert_eq!(browser[2].action, DemoSocketAction::Resize);
        assert!(browser[2].policy_window_resized);
        assert_eq!(browser[2].policy_width, 960);
        assert_eq!(browser[2].policy_height, 620);
        assert_eq!(browser[3].action, DemoSocketAction::Maximize);
        assert!(browser[3].policy_window_maximized);
        assert_eq!(browser[3].policy_state, "maximized");
        assert_eq!(browser[3].policy_y, 42);
        assert_eq!(browser[4].action, DemoSocketAction::Fullscreen);
        assert!(browser[4].policy_window_fullscreen);
        assert_eq!(browser[4].policy_state, "fullscreen");
        assert_eq!(browser[4].policy_x, 0);
        assert_eq!(browser[4].policy_y, 0);
        assert_eq!(browser[5].action, DemoSocketAction::Damage);
        assert!(browser[5].backend_surface_damaged);
        assert_eq!(browser[5].policy_state, "fullscreen");
        assert_eq!(browser[6].action, DemoSocketAction::Close);
        assert!(browser[6].policy_window_closed);
        assert_eq!(browser[6].backend_clients, 1);
        assert_eq!(browser[6].backend_surfaces, 1);
        assert_eq!(browser[6].policy_windows, 1);
        assert_eq!(browser[6].visible_windows, 1);
        assert_eq!(browser[6].focused_title, "socket-terminal");
        assert_eq!(browser[6].focused_app_id, "org.backlit.SocketTerminal");
    }
}
