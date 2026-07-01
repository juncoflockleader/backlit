use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_compositor_backend::{
    backend_launch_plan, preflight_backend_with_environment, smithay_runtime_probe, BackendKind,
    BackendLaunchPlan, BackendPreflightEnvironment, SmithayRuntimeProbe,
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
        emit_smithay_runtime_probe(&probe);
        if config.verify && report.ready && !probe.passed() {
            return Err(String::from(
                "DRM backend preflight is ready but Smithay runtime probe did not pass",
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

fn emit_smithay_runtime_probe(probe: &SmithayRuntimeProbe) {
    let primary_drm_card = probe.primary_drm_card.as_deref().unwrap_or("");
    let primary_drm_render_node = probe.primary_drm_render_node.as_deref().unwrap_or("");
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
                    "input_event_selected",
                    FieldValue::Bool(probe.input_event_selected),
                ),
                ("uses_logind", FieldValue::Bool(probe.uses_logind)),
                ("uses_libseat", FieldValue::Bool(probe.uses_libseat)),
                ("uses_libinput", FieldValue::Bool(probe.uses_libinput)),
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
