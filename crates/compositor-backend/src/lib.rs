use std::fmt;
use std::fs;
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::process::Command;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Headless,
    Wayland,
    Drm,
}

impl BackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Headless => "headless",
            Self::Wayland => "wayland",
            Self::Drm => "drm",
        }
    }

    pub fn needs_linux_graphics_stack(self) -> bool {
        matches!(self, Self::Wayland | Self::Drm)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendPreflightReport {
    pub backend: BackendKind,
    pub ready: bool,
    pub code: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendPreflightEnvironment {
    pub wayland_display: Option<String>,
    pub xdg_runtime_dir: Option<String>,
    pub xdg_runtime_dir_present: bool,
    pub xdg_runtime_dir_owned_by_user: bool,
    pub target_os: String,
    pub drm_card_nodes: u64,
    pub drm_render_nodes: u64,
    pub input_event_nodes: u64,
    pub drm_card_readable: u64,
    pub drm_card_writable: u64,
    pub drm_render_readable: u64,
    pub drm_render_writable: u64,
    pub input_event_readable: u64,
    pub session_id: Option<String>,
    pub seat: Option<String>,
    pub session_type: Option<String>,
    pub session_state: Option<String>,
    pub logind_session_verified: bool,
    pub session_active: bool,
    pub session_remote: bool,
    pub logind_available: bool,
    pub libseat_available: bool,
    pub libinput_available: bool,
    pub primary_drm_card: Option<String>,
    pub primary_drm_render_node: Option<String>,
    pub primary_input_event: Option<String>,
}

impl BackendPreflightEnvironment {
    pub fn for_target(target_os: impl Into<String>) -> Self {
        Self {
            wayland_display: None,
            xdg_runtime_dir: None,
            xdg_runtime_dir_present: false,
            xdg_runtime_dir_owned_by_user: false,
            target_os: target_os.into(),
            drm_card_nodes: 0,
            drm_render_nodes: 0,
            input_event_nodes: 0,
            drm_card_readable: 0,
            drm_card_writable: 0,
            drm_render_readable: 0,
            drm_render_writable: 0,
            input_event_readable: 0,
            session_id: None,
            seat: None,
            session_type: None,
            session_state: None,
            logind_session_verified: false,
            session_active: false,
            session_remote: false,
            logind_available: false,
            libseat_available: false,
            libinput_available: false,
            primary_drm_card: None,
            primary_drm_render_node: None,
            primary_input_event: None,
        }
    }

    pub fn from_host() -> Self {
        let mut environment = Self::for_target(std::env::consts::OS);
        environment.wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        environment.xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();
        let runtime_status = runtime_dir_status(
            environment.xdg_runtime_dir.as_deref(),
            &environment.target_os,
        );
        environment.xdg_runtime_dir_present = runtime_status.present;
        environment.xdg_runtime_dir_owned_by_user = runtime_status.owned_by_user;
        environment.session_id = std::env::var("XDG_SESSION_ID").ok();
        environment.seat = std::env::var("XDG_SEAT").ok();
        environment.session_type = std::env::var("XDG_SESSION_TYPE").ok();
        environment.logind_available = command_available("loginctl", &environment.target_os);
        environment.libseat_available =
            pkg_config_package_available("libseat", &environment.target_os);
        environment.libinput_available =
            pkg_config_package_available("libinput", &environment.target_os);
        environment.refresh_logind_session_status();
        environment.drm_card_nodes = count_entries_with_prefix("/dev/dri", "card");
        environment.drm_render_nodes = count_entries_with_prefix("/dev/dri", "renderD");
        environment.input_event_nodes = count_entries_with_prefix("/dev/input", "event");
        environment.drm_card_readable =
            count_openable_entries_with_prefix("/dev/dri", "card", AccessMode::Read);
        environment.drm_card_writable =
            count_openable_entries_with_prefix("/dev/dri", "card", AccessMode::Write);
        environment.drm_render_readable =
            count_openable_entries_with_prefix("/dev/dri", "renderD", AccessMode::Read);
        environment.drm_render_writable =
            count_openable_entries_with_prefix("/dev/dri", "renderD", AccessMode::Write);
        environment.input_event_readable =
            count_openable_entries_with_prefix("/dev/input", "event", AccessMode::Read);
        environment.primary_drm_card = first_openable_entry_with_prefix(
            "/dev/dri",
            "card",
            &[AccessMode::Read, AccessMode::Write],
        );
        environment.primary_drm_render_node = first_openable_entry_with_prefix(
            "/dev/dri",
            "renderD",
            &[AccessMode::Read, AccessMode::Write],
        )
        .or_else(|| first_openable_entry_with_prefix("/dev/dri", "renderD", &[AccessMode::Read]))
        .or_else(|| first_entry_with_prefix("/dev/dri", "renderD"));
        environment.primary_input_event =
            first_openable_entry_with_prefix("/dev/input", "event", &[AccessMode::Read])
                .or_else(|| first_entry_with_prefix("/dev/input", "event"));
        environment
    }

    pub fn with_wayland_display(mut self, value: impl Into<String>) -> Self {
        self.wayland_display = Some(value.into());
        self
    }

    pub fn with_xdg_runtime_dir(mut self, value: impl Into<String>) -> Self {
        self.xdg_runtime_dir = Some(value.into());
        self.xdg_runtime_dir_present = true;
        self.xdg_runtime_dir_owned_by_user = true;
        self
    }

    pub fn with_drm_nodes(mut self, card_nodes: u64, render_nodes: u64) -> Self {
        self.drm_card_nodes = card_nodes;
        self.drm_render_nodes = render_nodes;
        self
    }

    pub fn with_input_event_nodes(mut self, event_nodes: u64) -> Self {
        self.input_event_nodes = event_nodes;
        self
    }

    pub fn with_drm_card_access(mut self, readable: u64, writable: u64) -> Self {
        self.drm_card_readable = readable;
        self.drm_card_writable = writable;
        self
    }

    pub fn with_drm_render_access(mut self, readable: u64, writable: u64) -> Self {
        self.drm_render_readable = readable;
        self.drm_render_writable = writable;
        self
    }

    pub fn with_input_event_access(mut self, readable: u64) -> Self {
        self.input_event_readable = readable;
        self
    }

    pub fn with_primary_drm_card(mut self, value: impl Into<String>) -> Self {
        self.primary_drm_card = Some(value.into());
        self
    }

    pub fn with_primary_drm_render_node(mut self, value: impl Into<String>) -> Self {
        self.primary_drm_render_node = Some(value.into());
        self
    }

    pub fn with_primary_input_event(mut self, value: impl Into<String>) -> Self {
        self.primary_input_event = Some(value.into());
        self
    }

    pub fn with_seat_broker_tools(
        mut self,
        logind_available: bool,
        libseat_available: bool,
        libinput_available: bool,
    ) -> Self {
        self.logind_available = logind_available;
        self.libseat_available = libseat_available;
        self.libinput_available = libinput_available;
        self
    }

    pub fn with_session_id(mut self, value: impl Into<String>) -> Self {
        self.session_id = Some(value.into());
        self
    }

    pub fn with_active_local_session(
        mut self,
        value: impl Into<String>,
        seat: impl Into<String>,
        session_type: impl Into<String>,
    ) -> Self {
        self.session_id = Some(value.into());
        self.seat = Some(seat.into());
        self.session_type = Some(session_type.into());
        self.session_state = Some(String::from("active"));
        self.logind_session_verified = true;
        self.session_active = true;
        self.session_remote = false;
        self
    }

    pub fn drm_node_count(&self) -> u64 {
        self.drm_card_nodes + self.drm_render_nodes
    }

    pub fn drm_card_access_ready(&self) -> bool {
        self.drm_card_nodes > 0 && self.drm_card_readable > 0 && self.drm_card_writable > 0
    }

    pub fn input_requires_logind_broker(&self) -> bool {
        self.input_event_nodes > 0 && self.input_event_readable == 0
    }

    pub fn input_broker_ready(&self) -> bool {
        if self.input_event_nodes == 0 {
            return false;
        }

        if self.input_event_readable > 0 {
            return true;
        }

        self.input_requires_logind_broker()
            && self.logind_available
            && self.logind_session_verified
            && self.session_active
            && !self.session_remote
            && !missing(self.seat.as_deref())
            && !missing(self.session_type.as_deref())
            && self.session_type.as_deref() != Some("unspecified")
            && self.libseat_available
            && self.libinput_available
    }

    pub fn input_broker_mode(&self) -> &'static str {
        if self.input_event_nodes > 0 && self.input_event_readable > 0 {
            "direct"
        } else if self.input_broker_ready() {
            "logind-libseat"
        } else {
            "missing"
        }
    }

    fn refresh_logind_session_status(&mut self) {
        let Some(session_id) = self.session_id.as_deref() else {
            return;
        };

        if let Some(status) = logind_session_status(session_id, self.target_os.as_str()) {
            self.logind_session_verified = true;
            self.session_active = status.active;
            self.session_remote = status.remote;
            self.session_state = string_option(status.state);
            if !status.seat.is_empty() {
                self.seat = Some(status.seat);
            }
            if !status.session_type.is_empty() {
                self.session_type = Some(status.session_type);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendLaunchPlan {
    pub backend: BackendKind,
    pub ready: bool,
    pub implementation: &'static str,
    pub display_driver: &'static str,
    pub input_driver: &'static str,
    pub device_access: &'static str,
    pub uses_parent_wayland: bool,
    pub uses_drm: bool,
    pub uses_logind: bool,
    pub uses_libseat: bool,
    pub uses_libinput: bool,
    pub drm_card_selected: bool,
    pub drm_render_selected: bool,
    pub input_event_selected: bool,
    pub primary_drm_card: Option<String>,
    pub primary_drm_render_node: Option<String>,
    pub primary_input_event: Option<String>,
    pub session_id: Option<String>,
    pub seat: Option<String>,
    pub session_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayRuntimeProbe {
    pub feature_enabled: bool,
    pub compiled: bool,
    pub launch_ready: bool,
    pub target_os: String,
    pub backend: BackendKind,
    pub runtime_backend: &'static str,
    pub display_driver: &'static str,
    pub input_driver: &'static str,
    pub session_driver: &'static str,
    pub event_loop: &'static str,
    pub drm_card_selected: bool,
    pub drm_render_selected: bool,
    pub drm_node_resolved: bool,
    pub drm_node_type: &'static str,
    pub drm_node_primary_path: Option<String>,
    pub drm_node_render_path: Option<String>,
    pub kms_card_opened: bool,
    pub kms_device_created: bool,
    pub kms_event_source_inserted: bool,
    pub kms_event_loop_dispatched: bool,
    pub kms_atomic_modesetting: bool,
    pub kms_crtc_count: u64,
    pub kms_connector_count: u64,
    pub kms_connected_connector_count: u64,
    pub kms_mode_count: u64,
    pub kms_primary_plane_count: u64,
    pub kms_cursor_plane_count: u64,
    pub kms_overlay_plane_count: u64,
    pub kms_scanout_plan_ready: bool,
    pub kms_scanout_connector_id: u64,
    pub kms_scanout_connector_name: Option<String>,
    pub kms_scanout_crtc_id: u64,
    pub kms_scanout_primary_plane_id: u64,
    pub kms_scanout_mode_width: u64,
    pub kms_scanout_mode_height: u64,
    pub kms_scanout_mode_refresh_hz: u64,
    pub kms_scanout_mode_preferred: bool,
    pub kms_surface_created: bool,
    pub kms_surface_legacy: bool,
    pub kms_surface_crtc_matches_plan: bool,
    pub kms_surface_primary_plane_matches_plan: bool,
    pub kms_surface_pending_connector_count: u64,
    pub kms_surface_current_connector_count: u64,
    pub kms_surface_pending_mode_matches_plan: bool,
    pub kms_surface_commit_pending: bool,
    pub kms_surface_dropped_after_pause: bool,
    pub kms_framebuffer_created: bool,
    pub kms_framebuffer_added: bool,
    pub kms_framebuffer_test_state_succeeded: bool,
    pub kms_framebuffer_test_state_permission_denied: bool,
    pub kms_framebuffer_test_allow_modeset: bool,
    pub kms_framebuffer_primary_plane_matches_surface: bool,
    pub kms_framebuffer_width: u64,
    pub kms_framebuffer_height: u64,
    pub kms_framebuffer_released_before_surface_drop: bool,
    pub kms_framebuffer_failure: Option<String>,
    pub kms_first_present_framebuffer_filled: bool,
    pub kms_first_present_plane_state_ready: bool,
    pub kms_first_present_commit_attempted: bool,
    pub kms_first_present_commit_succeeded: bool,
    pub kms_first_present_vblank_event_received: bool,
    pub kms_first_present_blocked_by_drm_master: bool,
    pub kms_first_present_failure: Option<String>,
    pub kms_surface_failure: Option<String>,
    pub kms_resource_failure: Option<String>,
    pub renderer_node_selected: bool,
    pub renderer_node_path: Option<String>,
    pub input_event_selected: bool,
    pub uses_logind: bool,
    pub uses_libseat: bool,
    pub uses_libinput: bool,
    pub gbm_allocator_component: bool,
    pub egl_display_component: bool,
    pub gles_renderer_component: bool,
    pub renderer_node_opened: bool,
    pub gbm_device_created: bool,
    pub gbm_allocator_created: bool,
    pub egl_display_created: bool,
    pub egl_context_created: bool,
    pub gles_renderer_created: bool,
    pub offscreen_buffer_created: bool,
    pub offscreen_frame_rendered: bool,
    pub offscreen_frame_copied: bool,
    pub offscreen_pixel_verified: bool,
    pub offscreen_render_width: u64,
    pub offscreen_render_height: u64,
    pub offscreen_render_pixels: u64,
    pub offscreen_sample_red: u64,
    pub offscreen_sample_green: u64,
    pub offscreen_sample_blue: u64,
    pub offscreen_sample_alpha: u64,
    pub renderer_runtime_failure: Option<String>,
    pub libseat_session_created: bool,
    pub libseat_session_active: bool,
    pub libseat_session_seat: Option<String>,
    pub libseat_event_source_inserted: bool,
    pub libseat_event_loop_dispatched: bool,
    pub libseat_session_event_count: u64,
    pub libinput_context_created: bool,
    pub libinput_seat_assigned: bool,
    pub libinput_backend_created: bool,
    pub libinput_event_source_inserted: bool,
    pub libinput_event_loop_dispatched: bool,
    pub libinput_event_count: u64,
    pub input_runtime_failure: Option<String>,
    pub primary_drm_card: Option<String>,
    pub primary_drm_render_node: Option<String>,
    pub primary_input_event: Option<String>,
    pub components: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayRuntimeBootstrap {
    pub feature_enabled: bool,
    pub compiled: bool,
    pub runtime_backend: &'static str,
    pub display_created: bool,
    pub display_handle_created: bool,
    pub listening_socket_bound: bool,
    pub socket_name: String,
    pub socket_connect_succeeded: bool,
    pub socket_accept_succeeded: bool,
    pub client_inserted: bool,
    pub display_clients_dispatched: bool,
    pub display_dispatch_count: u64,
    pub display_clients_flushed: bool,
    pub event_loop_created: bool,
    pub event_loop_dispatched: bool,
    pub failure: String,
}

impl SmithayRuntimeBootstrap {
    pub fn passed(&self) -> bool {
        self.feature_enabled
            && self.compiled
            && self.runtime_backend == "smithay-drm-bootstrap"
            && self.display_created
            && self.display_handle_created
            && self.listening_socket_bound
            && !self.socket_name.is_empty()
            && self.socket_connect_succeeded
            && self.socket_accept_succeeded
            && self.client_inserted
            && self.display_clients_dispatched
            && self.display_clients_flushed
            && self.event_loop_created
            && self.event_loop_dispatched
            && self.failure.is_empty()
    }
}

impl SmithayRuntimeProbe {
    pub fn passed(&self) -> bool {
        self.feature_enabled
            && self.compiled
            && self.launch_ready
            && self.backend == BackendKind::Drm
            && self.runtime_backend == "smithay-drm-probe"
            && self.display_driver == "smithay-drm-kms"
            && self.input_driver == "smithay-libinput"
            && self.session_driver == "smithay-libseat-logind"
            && self.event_loop == "calloop"
            && self.drm_card_selected
            && self.drm_node_resolved
            && self.kms_card_opened
            && self.kms_device_created
            && self.kms_event_source_inserted
            && self.kms_event_loop_dispatched
            && self.kms_crtc_count > 0
            && self.kms_connector_count > 0
            && self.kms_connected_connector_count > 0
            && self.kms_mode_count > 0
            && self.kms_primary_plane_count > 0
            && self.kms_scanout_plan_ready
            && self.kms_scanout_connector_id > 0
            && self.kms_scanout_crtc_id > 0
            && self.kms_scanout_primary_plane_id > 0
            && self.kms_scanout_mode_width > 0
            && self.kms_scanout_mode_height > 0
            && self.kms_scanout_mode_refresh_hz > 0
            && self.kms_surface_created
            && self.kms_surface_crtc_matches_plan
            && self.kms_surface_primary_plane_matches_plan
            && self.kms_surface_pending_connector_count > 0
            && self.kms_surface_pending_mode_matches_plan
            && self.kms_surface_dropped_after_pause
            && self.kms_framebuffer_created
            && self.kms_framebuffer_added
            && (self.kms_framebuffer_test_state_succeeded
                || self.kms_framebuffer_test_state_permission_denied)
            && self.kms_framebuffer_primary_plane_matches_surface
            && self.kms_framebuffer_width == self.kms_scanout_mode_width
            && self.kms_framebuffer_height == self.kms_scanout_mode_height
            && self.kms_framebuffer_released_before_surface_drop
            && self.kms_framebuffer_failure.is_none()
            && self.kms_first_present_framebuffer_filled
            && self.kms_first_present_plane_state_ready
            && (self.kms_first_present_blocked_by_drm_master
                || self.kms_first_present_commit_succeeded)
            && (!self.kms_first_present_commit_succeeded
                || self.kms_first_present_vblank_event_received)
            && self.kms_first_present_failure.is_none()
            && self.kms_surface_failure.is_none()
            && self.kms_resource_failure.is_none()
            && self.renderer_node_selected
            && self.input_event_selected
            && self.uses_logind
            && self.uses_libseat
            && self.uses_libinput
            && self.gbm_allocator_component
            && self.egl_display_component
            && self.gles_renderer_component
            && self.renderer_node_opened
            && self.gbm_device_created
            && self.gbm_allocator_created
            && self.egl_display_created
            && self.egl_context_created
            && self.gles_renderer_created
            && self.offscreen_buffer_created
            && self.offscreen_frame_rendered
            && self.offscreen_frame_copied
            && self.offscreen_pixel_verified
            && self.offscreen_render_width > 0
            && self.offscreen_render_height > 0
            && self.offscreen_render_pixels
                == self.offscreen_render_width * self.offscreen_render_height
            && self.renderer_runtime_failure.is_none()
            && self.libseat_session_created
            && self.libseat_event_source_inserted
            && self.libseat_event_loop_dispatched
            && self.libinput_context_created
            && self.libinput_seat_assigned
            && self.libinput_backend_created
            && self.libinput_event_source_inserted
            && self.libinput_event_loop_dispatched
            && self.input_runtime_failure.is_none()
    }
}

pub fn smithay_runtime_bootstrap() -> SmithayRuntimeBootstrap {
    smithay_runtime_bootstrap_impl()
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn smithay_runtime_bootstrap_impl() -> SmithayRuntimeBootstrap {
    use std::{env, os::unix::net::UnixStream, path::PathBuf, sync::Arc, time::Duration};

    use smithay::reexports::calloop::EventLoop;
    use smithay::reexports::wayland_server::backend::ClientData;
    use smithay::reexports::wayland_server::{Display, ListeningSocket};

    #[derive(Default)]
    struct BootstrapState;

    let mut display = match Display::<BootstrapState>::new() {
        Ok(display) => display,
        Err(error) => {
            return SmithayRuntimeBootstrap {
                feature_enabled: true,
                compiled: true,
                runtime_backend: "smithay-drm-bootstrap",
                display_created: false,
                display_handle_created: false,
                listening_socket_bound: false,
                socket_name: String::new(),
                socket_connect_succeeded: false,
                socket_accept_succeeded: false,
                client_inserted: false,
                display_clients_dispatched: false,
                display_dispatch_count: 0,
                display_clients_flushed: false,
                event_loop_created: false,
                event_loop_dispatched: false,
                failure: format!("display-new:{error}"),
            };
        }
    };
    let mut display_handle = display.handle();
    let listening_socket = match ListeningSocket::bind_auto("backlit-smithay-bootstrap", 0..64) {
        Ok(listening_socket) => listening_socket,
        Err(error) => {
            return SmithayRuntimeBootstrap {
                feature_enabled: true,
                compiled: true,
                runtime_backend: "smithay-drm-bootstrap",
                display_created: true,
                display_handle_created: true,
                listening_socket_bound: false,
                socket_name: String::new(),
                socket_connect_succeeded: false,
                socket_accept_succeeded: false,
                client_inserted: false,
                display_clients_dispatched: false,
                display_dispatch_count: 0,
                display_clients_flushed: false,
                event_loop_created: false,
                event_loop_dispatched: false,
                failure: format!("socket-bind:{error}"),
            };
        }
    };
    let socket_name = match listening_socket
        .socket_name()
        .and_then(|name| name.to_str())
        .map(String::from)
    {
        Some(socket_name) => socket_name,
        None => {
            return SmithayRuntimeBootstrap {
                feature_enabled: true,
                compiled: true,
                runtime_backend: "smithay-drm-bootstrap",
                display_created: true,
                display_handle_created: true,
                listening_socket_bound: true,
                socket_name: String::new(),
                socket_connect_succeeded: false,
                socket_accept_succeeded: false,
                client_inserted: false,
                display_clients_dispatched: false,
                display_dispatch_count: 0,
                display_clients_flushed: false,
                event_loop_created: false,
                event_loop_dispatched: false,
                failure: String::from("socket-name:unavailable"),
            };
        }
    };
    let runtime_dir = match env::var_os("XDG_RUNTIME_DIR") {
        Some(runtime_dir) => runtime_dir,
        None => {
            return SmithayRuntimeBootstrap {
                feature_enabled: true,
                compiled: true,
                runtime_backend: "smithay-drm-bootstrap",
                display_created: true,
                display_handle_created: true,
                listening_socket_bound: true,
                socket_name,
                socket_connect_succeeded: false,
                socket_accept_succeeded: false,
                client_inserted: false,
                display_clients_dispatched: false,
                display_dispatch_count: 0,
                display_clients_flushed: false,
                event_loop_created: false,
                event_loop_dispatched: false,
                failure: String::from("socket-connect:missing-runtime-dir"),
            };
        }
    };
    let socket_path = PathBuf::from(runtime_dir).join(&socket_name);
    let _client_stream = match UnixStream::connect(&socket_path) {
        Ok(client_stream) => client_stream,
        Err(error) => {
            return SmithayRuntimeBootstrap {
                feature_enabled: true,
                compiled: true,
                runtime_backend: "smithay-drm-bootstrap",
                display_created: true,
                display_handle_created: true,
                listening_socket_bound: true,
                socket_name,
                socket_connect_succeeded: false,
                socket_accept_succeeded: false,
                client_inserted: false,
                display_clients_dispatched: false,
                display_dispatch_count: 0,
                display_clients_flushed: false,
                event_loop_created: false,
                event_loop_dispatched: false,
                failure: format!("socket-connect:{error}"),
            };
        }
    };
    let accepted_stream = match accept_bootstrap_client(&listening_socket) {
        Ok(accepted_stream) => accepted_stream,
        Err(error) => {
            return SmithayRuntimeBootstrap {
                feature_enabled: true,
                compiled: true,
                runtime_backend: "smithay-drm-bootstrap",
                display_created: true,
                display_handle_created: true,
                listening_socket_bound: true,
                socket_name,
                socket_connect_succeeded: true,
                socket_accept_succeeded: false,
                client_inserted: false,
                display_clients_dispatched: false,
                display_dispatch_count: 0,
                display_clients_flushed: false,
                event_loop_created: false,
                event_loop_dispatched: false,
                failure: error,
            };
        }
    };
    let client_data: Arc<dyn ClientData> = Arc::new(());
    let _client = match display_handle.insert_client(accepted_stream, client_data) {
        Ok(client) => client,
        Err(error) => {
            return SmithayRuntimeBootstrap {
                feature_enabled: true,
                compiled: true,
                runtime_backend: "smithay-drm-bootstrap",
                display_created: true,
                display_handle_created: true,
                listening_socket_bound: true,
                socket_name,
                socket_connect_succeeded: true,
                socket_accept_succeeded: true,
                client_inserted: false,
                display_clients_dispatched: false,
                display_dispatch_count: 0,
                display_clients_flushed: false,
                event_loop_created: false,
                event_loop_dispatched: false,
                failure: format!("client-insert:{error}"),
            };
        }
    };
    let mut state = BootstrapState;
    let display_dispatch_count = match display.dispatch_clients(&mut state) {
        Ok(count) => count as u64,
        Err(error) => {
            return SmithayRuntimeBootstrap {
                feature_enabled: true,
                compiled: true,
                runtime_backend: "smithay-drm-bootstrap",
                display_created: true,
                display_handle_created: true,
                listening_socket_bound: true,
                socket_name,
                socket_connect_succeeded: true,
                socket_accept_succeeded: true,
                client_inserted: true,
                display_clients_dispatched: false,
                display_dispatch_count: 0,
                display_clients_flushed: false,
                event_loop_created: false,
                event_loop_dispatched: false,
                failure: format!("display-dispatch:{error}"),
            };
        }
    };
    let display_clients_flushed = match display.flush_clients() {
        Ok(()) => true,
        Err(error) => {
            return SmithayRuntimeBootstrap {
                feature_enabled: true,
                compiled: true,
                runtime_backend: "smithay-drm-bootstrap",
                display_created: true,
                display_handle_created: true,
                listening_socket_bound: true,
                socket_name,
                socket_connect_succeeded: true,
                socket_accept_succeeded: true,
                client_inserted: true,
                display_clients_dispatched: true,
                display_dispatch_count,
                display_clients_flushed: false,
                event_loop_created: false,
                event_loop_dispatched: false,
                failure: format!("display-flush:{error}"),
            };
        }
    };
    let mut event_loop = match EventLoop::<BootstrapState>::try_new() {
        Ok(event_loop) => event_loop,
        Err(error) => {
            return SmithayRuntimeBootstrap {
                feature_enabled: true,
                compiled: true,
                runtime_backend: "smithay-drm-bootstrap",
                display_created: true,
                display_handle_created: true,
                listening_socket_bound: true,
                socket_name,
                socket_connect_succeeded: true,
                socket_accept_succeeded: true,
                client_inserted: true,
                display_clients_dispatched: true,
                display_dispatch_count,
                display_clients_flushed,
                event_loop_created: false,
                event_loop_dispatched: false,
                failure: format!("event-loop-new:{error}"),
            };
        }
    };
    let event_loop_dispatched =
        match event_loop.dispatch(Some(Duration::from_millis(0)), &mut state) {
            Ok(()) => true,
            Err(error) => {
                return SmithayRuntimeBootstrap {
                    feature_enabled: true,
                    compiled: true,
                    runtime_backend: "smithay-drm-bootstrap",
                    display_created: true,
                    display_handle_created: true,
                    listening_socket_bound: true,
                    socket_name,
                    socket_connect_succeeded: true,
                    socket_accept_succeeded: true,
                    client_inserted: true,
                    display_clients_dispatched: true,
                    display_dispatch_count,
                    display_clients_flushed,
                    event_loop_created: true,
                    event_loop_dispatched: false,
                    failure: format!("event-loop-dispatch:{error}"),
                };
            }
        };

    SmithayRuntimeBootstrap {
        feature_enabled: true,
        compiled: true,
        runtime_backend: "smithay-drm-bootstrap",
        display_created: true,
        display_handle_created: true,
        listening_socket_bound: true,
        socket_name,
        socket_connect_succeeded: true,
        socket_accept_succeeded: true,
        client_inserted: true,
        display_clients_dispatched: true,
        display_dispatch_count,
        display_clients_flushed,
        event_loop_created: true,
        event_loop_dispatched,
        failure: String::new(),
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn accept_bootstrap_client(
    listening_socket: &smithay::reexports::wayland_server::ListeningSocket,
) -> Result<std::os::unix::net::UnixStream, String> {
    for _ in 0..16 {
        match listening_socket.accept() {
            Ok(Some(stream)) => return Ok(stream),
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(1)),
            Err(error) => return Err(format!("socket-accept:{error}")),
        }
    }

    Err(String::from("socket-accept:would-block"))
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn smithay_runtime_bootstrap_impl() -> SmithayRuntimeBootstrap {
    SmithayRuntimeBootstrap {
        feature_enabled: cfg!(feature = "smithay-backend"),
        compiled: false,
        runtime_backend: "smithay-uncompiled",
        display_created: false,
        display_handle_created: false,
        listening_socket_bound: false,
        socket_name: String::new(),
        socket_connect_succeeded: false,
        socket_accept_succeeded: false,
        client_inserted: false,
        display_clients_dispatched: false,
        display_dispatch_count: 0,
        display_clients_flushed: false,
        event_loop_created: false,
        event_loop_dispatched: false,
        failure: String::from("unavailable"),
    }
}

pub fn smithay_runtime_probe(environment: &BackendPreflightEnvironment) -> SmithayRuntimeProbe {
    let feature_enabled = cfg!(feature = "smithay-backend");
    let compiled = cfg!(all(feature = "smithay-backend", target_os = "linux"));
    let drm_node_probe = smithay_drm_node_probe(environment.primary_drm_card.as_deref());
    let renderer_node_path = if compiled {
        environment
            .primary_drm_render_node
            .clone()
            .or_else(|| drm_node_probe.renderer_path.clone())
    } else {
        None
    };
    let renderer_node_selected = renderer_node_path.is_some();
    let input_runtime_probe = smithay_input_runtime_probe(
        environment.target_os.as_str(),
        environment.input_broker_ready(),
        environment.seat.as_deref(),
    );
    let kms_runtime_probe = smithay_kms_runtime_probe(
        environment.target_os.as_str(),
        environment.primary_drm_card.as_deref(),
    );
    let renderer_runtime_probe = smithay_renderer_runtime_probe(
        environment.target_os.as_str(),
        renderer_node_path.as_deref(),
    );
    let launch_ready = compiled
        && environment.target_os == "linux"
        && environment.drm_card_access_ready()
        && environment.input_broker_ready()
        && environment.primary_drm_card.is_some()
        && environment.primary_input_event.is_some()
        && drm_node_probe.resolved
        && kms_runtime_probe.passed()
        && renderer_node_selected
        && renderer_runtime_probe.passed()
        && input_runtime_probe.passed();

    SmithayRuntimeProbe {
        feature_enabled,
        compiled,
        launch_ready,
        target_os: environment.target_os.clone(),
        backend: BackendKind::Drm,
        runtime_backend: if compiled {
            "smithay-drm-probe"
        } else {
            "smithay-uncompiled"
        },
        display_driver: if compiled {
            "smithay-drm-kms"
        } else {
            "unavailable"
        },
        input_driver: if compiled {
            "smithay-libinput"
        } else {
            "unavailable"
        },
        session_driver: if compiled {
            "smithay-libseat-logind"
        } else {
            "unavailable"
        },
        event_loop: if compiled { "calloop" } else { "unavailable" },
        drm_card_selected: environment.primary_drm_card.is_some(),
        drm_render_selected: environment.primary_drm_render_node.is_some(),
        drm_node_resolved: drm_node_probe.resolved,
        drm_node_type: drm_node_probe.node_type,
        drm_node_primary_path: drm_node_probe.primary_path,
        drm_node_render_path: drm_node_probe.render_path.clone(),
        kms_card_opened: kms_runtime_probe.kms_card_opened,
        kms_device_created: kms_runtime_probe.kms_device_created,
        kms_event_source_inserted: kms_runtime_probe.kms_event_source_inserted,
        kms_event_loop_dispatched: kms_runtime_probe.kms_event_loop_dispatched,
        kms_atomic_modesetting: kms_runtime_probe.kms_atomic_modesetting,
        kms_crtc_count: kms_runtime_probe.kms_crtc_count,
        kms_connector_count: kms_runtime_probe.kms_connector_count,
        kms_connected_connector_count: kms_runtime_probe.kms_connected_connector_count,
        kms_mode_count: kms_runtime_probe.kms_mode_count,
        kms_primary_plane_count: kms_runtime_probe.kms_primary_plane_count,
        kms_cursor_plane_count: kms_runtime_probe.kms_cursor_plane_count,
        kms_overlay_plane_count: kms_runtime_probe.kms_overlay_plane_count,
        kms_scanout_plan_ready: kms_runtime_probe.kms_scanout_plan_ready,
        kms_scanout_connector_id: kms_runtime_probe.kms_scanout_connector_id,
        kms_scanout_connector_name: kms_runtime_probe.kms_scanout_connector_name,
        kms_scanout_crtc_id: kms_runtime_probe.kms_scanout_crtc_id,
        kms_scanout_primary_plane_id: kms_runtime_probe.kms_scanout_primary_plane_id,
        kms_scanout_mode_width: kms_runtime_probe.kms_scanout_mode_width,
        kms_scanout_mode_height: kms_runtime_probe.kms_scanout_mode_height,
        kms_scanout_mode_refresh_hz: kms_runtime_probe.kms_scanout_mode_refresh_hz,
        kms_scanout_mode_preferred: kms_runtime_probe.kms_scanout_mode_preferred,
        kms_surface_created: kms_runtime_probe.kms_surface_created,
        kms_surface_legacy: kms_runtime_probe.kms_surface_legacy,
        kms_surface_crtc_matches_plan: kms_runtime_probe.kms_surface_crtc_matches_plan,
        kms_surface_primary_plane_matches_plan: kms_runtime_probe
            .kms_surface_primary_plane_matches_plan,
        kms_surface_pending_connector_count: kms_runtime_probe.kms_surface_pending_connector_count,
        kms_surface_current_connector_count: kms_runtime_probe.kms_surface_current_connector_count,
        kms_surface_pending_mode_matches_plan: kms_runtime_probe
            .kms_surface_pending_mode_matches_plan,
        kms_surface_commit_pending: kms_runtime_probe.kms_surface_commit_pending,
        kms_surface_dropped_after_pause: kms_runtime_probe.kms_surface_dropped_after_pause,
        kms_framebuffer_created: kms_runtime_probe.kms_framebuffer_created,
        kms_framebuffer_added: kms_runtime_probe.kms_framebuffer_added,
        kms_framebuffer_test_state_succeeded: kms_runtime_probe
            .kms_framebuffer_test_state_succeeded,
        kms_framebuffer_test_state_permission_denied: kms_runtime_probe
            .kms_framebuffer_test_state_permission_denied,
        kms_framebuffer_test_allow_modeset: kms_runtime_probe.kms_framebuffer_test_allow_modeset,
        kms_framebuffer_primary_plane_matches_surface: kms_runtime_probe
            .kms_framebuffer_primary_plane_matches_surface,
        kms_framebuffer_width: kms_runtime_probe.kms_framebuffer_width,
        kms_framebuffer_height: kms_runtime_probe.kms_framebuffer_height,
        kms_framebuffer_released_before_surface_drop: kms_runtime_probe
            .kms_framebuffer_released_before_surface_drop,
        kms_framebuffer_failure: kms_runtime_probe.framebuffer_failure,
        kms_first_present_framebuffer_filled: kms_runtime_probe
            .kms_first_present_framebuffer_filled,
        kms_first_present_plane_state_ready: kms_runtime_probe.kms_first_present_plane_state_ready,
        kms_first_present_commit_attempted: kms_runtime_probe.kms_first_present_commit_attempted,
        kms_first_present_commit_succeeded: kms_runtime_probe.kms_first_present_commit_succeeded,
        kms_first_present_vblank_event_received: kms_runtime_probe
            .kms_first_present_vblank_event_received,
        kms_first_present_blocked_by_drm_master: kms_runtime_probe
            .kms_first_present_blocked_by_drm_master,
        kms_first_present_failure: kms_runtime_probe.first_present_failure,
        kms_surface_failure: kms_runtime_probe.surface_failure,
        kms_resource_failure: kms_runtime_probe.failure,
        renderer_node_selected,
        renderer_node_path,
        input_event_selected: environment.primary_input_event.is_some(),
        uses_logind: environment.logind_session_verified || environment.logind_available,
        uses_libseat: environment.input_broker_mode() == "logind-libseat",
        uses_libinput: environment.input_broker_ready(),
        gbm_allocator_component: compiled,
        egl_display_component: compiled,
        gles_renderer_component: compiled,
        renderer_node_opened: renderer_runtime_probe.renderer_node_opened,
        gbm_device_created: renderer_runtime_probe.gbm_device_created,
        gbm_allocator_created: renderer_runtime_probe.gbm_allocator_created,
        egl_display_created: renderer_runtime_probe.egl_display_created,
        egl_context_created: renderer_runtime_probe.egl_context_created,
        gles_renderer_created: renderer_runtime_probe.gles_renderer_created,
        offscreen_buffer_created: renderer_runtime_probe.offscreen_buffer_created,
        offscreen_frame_rendered: renderer_runtime_probe.offscreen_frame_rendered,
        offscreen_frame_copied: renderer_runtime_probe.offscreen_frame_copied,
        offscreen_pixel_verified: renderer_runtime_probe.offscreen_pixel_verified,
        offscreen_render_width: renderer_runtime_probe.offscreen_render_width,
        offscreen_render_height: renderer_runtime_probe.offscreen_render_height,
        offscreen_render_pixels: renderer_runtime_probe.offscreen_render_pixels,
        offscreen_sample_red: renderer_runtime_probe.offscreen_sample_red,
        offscreen_sample_green: renderer_runtime_probe.offscreen_sample_green,
        offscreen_sample_blue: renderer_runtime_probe.offscreen_sample_blue,
        offscreen_sample_alpha: renderer_runtime_probe.offscreen_sample_alpha,
        renderer_runtime_failure: renderer_runtime_probe.failure,
        libseat_session_created: input_runtime_probe.libseat_session_created,
        libseat_session_active: input_runtime_probe.libseat_session_active,
        libseat_session_seat: input_runtime_probe.libseat_session_seat,
        libseat_event_source_inserted: input_runtime_probe.libseat_event_source_inserted,
        libseat_event_loop_dispatched: input_runtime_probe.libseat_event_loop_dispatched,
        libseat_session_event_count: input_runtime_probe.libseat_session_event_count,
        libinput_context_created: input_runtime_probe.libinput_context_created,
        libinput_seat_assigned: input_runtime_probe.libinput_seat_assigned,
        libinput_backend_created: input_runtime_probe.libinput_backend_created,
        libinput_event_source_inserted: input_runtime_probe.libinput_event_source_inserted,
        libinput_event_loop_dispatched: input_runtime_probe.libinput_event_loop_dispatched,
        libinput_event_count: input_runtime_probe.libinput_event_count,
        input_runtime_failure: input_runtime_probe.failure,
        primary_drm_card: environment.primary_drm_card.clone(),
        primary_drm_render_node: environment.primary_drm_render_node.clone(),
        primary_input_event: environment.primary_input_event.clone(),
        components: smithay_runtime_components(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SmithayDrmNodeProbe {
    resolved: bool,
    node_type: &'static str,
    primary_path: Option<String>,
    render_path: Option<String>,
    renderer_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SmithayKmsRuntimeProbe {
    kms_card_opened: bool,
    kms_device_created: bool,
    kms_event_source_inserted: bool,
    kms_event_loop_dispatched: bool,
    kms_atomic_modesetting: bool,
    kms_crtc_count: u64,
    kms_connector_count: u64,
    kms_connected_connector_count: u64,
    kms_mode_count: u64,
    kms_primary_plane_count: u64,
    kms_cursor_plane_count: u64,
    kms_overlay_plane_count: u64,
    kms_scanout_plan_ready: bool,
    kms_scanout_connector_id: u64,
    kms_scanout_connector_name: Option<String>,
    kms_scanout_crtc_id: u64,
    kms_scanout_primary_plane_id: u64,
    kms_scanout_mode_width: u64,
    kms_scanout_mode_height: u64,
    kms_scanout_mode_refresh_hz: u64,
    kms_scanout_mode_preferred: bool,
    kms_surface_created: bool,
    kms_surface_legacy: bool,
    kms_surface_crtc_matches_plan: bool,
    kms_surface_primary_plane_matches_plan: bool,
    kms_surface_pending_connector_count: u64,
    kms_surface_current_connector_count: u64,
    kms_surface_pending_mode_matches_plan: bool,
    kms_surface_commit_pending: bool,
    kms_surface_dropped_after_pause: bool,
    kms_framebuffer_created: bool,
    kms_framebuffer_added: bool,
    kms_framebuffer_test_state_succeeded: bool,
    kms_framebuffer_test_state_permission_denied: bool,
    kms_framebuffer_test_allow_modeset: bool,
    kms_framebuffer_primary_plane_matches_surface: bool,
    kms_framebuffer_width: u64,
    kms_framebuffer_height: u64,
    kms_framebuffer_released_before_surface_drop: bool,
    kms_first_present_framebuffer_filled: bool,
    kms_first_present_plane_state_ready: bool,
    kms_first_present_commit_attempted: bool,
    kms_first_present_commit_succeeded: bool,
    kms_first_present_vblank_event_received: bool,
    kms_first_present_blocked_by_drm_master: bool,
    first_present_failure: Option<String>,
    framebuffer_failure: Option<String>,
    surface_failure: Option<String>,
    failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SmithayInputRuntimeProbe {
    libseat_session_created: bool,
    libseat_session_active: bool,
    libseat_session_seat: Option<String>,
    libseat_event_source_inserted: bool,
    libseat_event_loop_dispatched: bool,
    libseat_session_event_count: u64,
    libinput_context_created: bool,
    libinput_seat_assigned: bool,
    libinput_backend_created: bool,
    libinput_event_source_inserted: bool,
    libinput_event_loop_dispatched: bool,
    libinput_event_count: u64,
    failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SmithayRendererRuntimeProbe {
    renderer_node_opened: bool,
    gbm_device_created: bool,
    gbm_allocator_created: bool,
    egl_display_created: bool,
    egl_context_created: bool,
    gles_renderer_created: bool,
    offscreen_buffer_created: bool,
    offscreen_frame_rendered: bool,
    offscreen_frame_copied: bool,
    offscreen_pixel_verified: bool,
    offscreen_render_width: u64,
    offscreen_render_height: u64,
    offscreen_render_pixels: u64,
    offscreen_sample_red: u64,
    offscreen_sample_green: u64,
    offscreen_sample_blue: u64,
    offscreen_sample_alpha: u64,
    failure: Option<String>,
}

impl SmithayKmsRuntimeProbe {
    fn passed(&self) -> bool {
        self.kms_card_opened
            && self.kms_device_created
            && self.kms_event_source_inserted
            && self.kms_event_loop_dispatched
            && self.kms_crtc_count > 0
            && self.kms_connector_count > 0
            && self.kms_connected_connector_count > 0
            && self.kms_mode_count > 0
            && self.kms_primary_plane_count > 0
            && self.kms_scanout_plan_ready
            && self.kms_scanout_connector_id > 0
            && self.kms_scanout_crtc_id > 0
            && self.kms_scanout_primary_plane_id > 0
            && self.kms_scanout_mode_width > 0
            && self.kms_scanout_mode_height > 0
            && self.kms_scanout_mode_refresh_hz > 0
            && self.kms_surface_created
            && self.kms_surface_crtc_matches_plan
            && self.kms_surface_primary_plane_matches_plan
            && self.kms_surface_pending_connector_count > 0
            && self.kms_surface_pending_mode_matches_plan
            && self.kms_surface_dropped_after_pause
            && self.kms_framebuffer_created
            && self.kms_framebuffer_added
            && (self.kms_framebuffer_test_state_succeeded
                || self.kms_framebuffer_test_state_permission_denied)
            && self.kms_framebuffer_primary_plane_matches_surface
            && self.kms_framebuffer_width == self.kms_scanout_mode_width
            && self.kms_framebuffer_height == self.kms_scanout_mode_height
            && self.kms_framebuffer_released_before_surface_drop
            && self.kms_first_present_framebuffer_filled
            && self.kms_first_present_plane_state_ready
            && (self.kms_first_present_blocked_by_drm_master
                || self.kms_first_present_commit_succeeded)
            && (!self.kms_first_present_commit_succeeded
                || self.kms_first_present_vblank_event_received)
            && self.first_present_failure.is_none()
            && self.framebuffer_failure.is_none()
            && self.surface_failure.is_none()
            && self.failure.is_none()
    }
}

impl SmithayInputRuntimeProbe {
    fn passed(&self) -> bool {
        self.libseat_session_created
            && self.libseat_event_source_inserted
            && self.libseat_event_loop_dispatched
            && self.libinput_context_created
            && self.libinput_seat_assigned
            && self.libinput_backend_created
            && self.libinput_event_source_inserted
            && self.libinput_event_loop_dispatched
            && self.failure.is_none()
    }
}

impl SmithayRendererRuntimeProbe {
    fn passed(&self) -> bool {
        self.renderer_node_opened
            && self.gbm_device_created
            && self.gbm_allocator_created
            && self.egl_display_created
            && self.egl_context_created
            && self.gles_renderer_created
            && self.offscreen_buffer_created
            && self.offscreen_frame_rendered
            && self.offscreen_frame_copied
            && self.offscreen_pixel_verified
            && self.offscreen_render_width > 0
            && self.offscreen_render_height > 0
            && self.offscreen_render_pixels
                == self.offscreen_render_width * self.offscreen_render_height
            && self.failure.is_none()
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn smithay_drm_node_probe(path: Option<&str>) -> SmithayDrmNodeProbe {
    use smithay::backend::drm::{DrmNode, NodeType};

    let Some(path) = path else {
        return unavailable_smithay_drm_node_probe();
    };

    let Ok(node) = DrmNode::from_path(path) else {
        return unavailable_smithay_drm_node_probe();
    };

    let primary_path = node
        .dev_path_with_type(NodeType::Primary)
        .map(|path| path.to_string_lossy().into_owned());
    let render_path = node
        .dev_path_with_type(NodeType::Render)
        .map(|path| path.to_string_lossy().into_owned());
    let renderer_path = render_path.clone().or_else(|| {
        node.dev_path()
            .map(|path| path.to_string_lossy().into_owned())
    });

    SmithayDrmNodeProbe {
        resolved: true,
        node_type: smithay_drm_node_type_name(node.ty()),
        primary_path,
        render_path,
        renderer_path,
    }
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn smithay_drm_node_probe(_path: Option<&str>) -> SmithayDrmNodeProbe {
    unavailable_smithay_drm_node_probe()
}

fn unavailable_smithay_drm_node_probe() -> SmithayDrmNodeProbe {
    SmithayDrmNodeProbe {
        resolved: false,
        node_type: "unavailable",
        primary_path: None,
        render_path: None,
        renderer_path: None,
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn smithay_kms_runtime_probe(
    target_os: &str,
    primary_drm_card: Option<&str>,
) -> SmithayKmsRuntimeProbe {
    use std::os::unix::io::OwnedFd;
    use std::time::Duration;

    use smithay::backend::allocator::{dumb::DumbAllocator, Allocator, Fourcc, Modifier};
    use smithay::backend::drm::dumb::framebuffer_from_dumb_buffer;
    use smithay::backend::drm::{
        DrmDevice, DrmDeviceFd, DrmError, DrmEvent, PlaneConfig, PlaneState,
    };
    use smithay::reexports::calloop::EventLoop;
    use smithay::reexports::drm::buffer::Buffer as DrmBuffer;
    use smithay::reexports::drm::control::{
        connector, crtc, plane, Device as ControlDevice, Mode, ModeTypeFlags,
    };
    use smithay::utils::{DeviceFd, Rectangle, Transform};

    #[derive(Default)]
    struct KmsEventState {
        vblank_count: u64,
        error_count: u64,
    }

    let mut probe = SmithayKmsRuntimeProbe {
        failure: None,
        surface_failure: None,
        framebuffer_failure: None,
        first_present_failure: None,
        ..unavailable_smithay_kms_runtime_probe("unavailable")
    };

    if target_os != "linux" {
        return unavailable_smithay_kms_runtime_probe("non-linux-target");
    }

    let Some(path) = primary_drm_card.filter(|value| !value.trim().is_empty()) else {
        return unavailable_smithay_kms_runtime_probe("missing-drm-card");
    };

    let file = match std::fs::File::options().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(error) => {
            probe.failure = Some(format!("kms-card-open:{error}"));
            return probe;
        }
    };
    probe.kms_card_opened = true;

    let drm_fd = DrmDeviceFd::new(DeviceFd::from(Into::<OwnedFd>::into(file)));
    let (mut device, notifier) = match DrmDevice::new(drm_fd, false) {
        Ok(device) => device,
        Err(error) => {
            probe.failure = Some(format!("kms-device:{error:?}"));
            return probe;
        }
    };
    probe.kms_device_created = true;
    probe.kms_atomic_modesetting = device.is_atomic();

    let resources = match device.resource_handles() {
        Ok(resources) => resources,
        Err(error) => {
            probe.failure = Some(format!("kms-resources:{error:?}"));
            device.pause();
            return probe;
        }
    };

    probe.kms_crtc_count = device.crtcs().len() as u64;
    probe.kms_connector_count = resources.connectors().len() as u64;

    let mut scanout_surface_target: Option<(connector::Handle, crtc::Handle, plane::Handle, Mode)> =
        None;
    let mut primary_planes_by_crtc = Vec::new();
    for crtc in device.crtcs() {
        let planes = match device.planes(crtc) {
            Ok(planes) => planes,
            Err(error) => {
                probe.failure = Some(format!("kms-planes:{error:?}"));
                device.pause();
                return probe;
            }
        };
        probe.kms_primary_plane_count += planes.primary.len() as u64;
        probe.kms_cursor_plane_count += planes.cursor.len() as u64;
        probe.kms_overlay_plane_count += planes.overlay.len() as u64;
        if let Some(primary_plane) = planes.primary.first() {
            primary_planes_by_crtc.push((*crtc, primary_plane.handle));
        }
    }

    for connector_handle in resources.connectors() {
        let connector_info = match device.get_connector(*connector_handle, false) {
            Ok(info) => info,
            Err(error) => {
                probe.failure = Some(format!("kms-connector:{error:?}"));
                device.pause();
                return probe;
            }
        };
        if connector_info.state() == connector::State::Connected {
            probe.kms_connected_connector_count += 1;
            probe.kms_mode_count += connector_info.modes().len() as u64;

            if !probe.kms_scanout_plan_ready {
                let selected_mode = connector_info
                    .modes()
                    .iter()
                    .copied()
                    .find(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
                    .or_else(|| connector_info.modes().first().copied());
                if let Some(selected_mode) = selected_mode {
                    let mut candidate_crtcs = Vec::new();
                    for encoder_handle in connector_info
                        .current_encoder()
                        .into_iter()
                        .chain(connector_info.encoders().iter().copied())
                    {
                        let encoder_info = match device.get_encoder(encoder_handle) {
                            Ok(info) => info,
                            Err(error) => {
                                probe.failure = Some(format!("kms-encoder:{error:?}"));
                                device.pause();
                                return probe;
                            }
                        };
                        if let Some(current_crtc) = encoder_info.crtc() {
                            if !candidate_crtcs.contains(&current_crtc) {
                                candidate_crtcs.push(current_crtc);
                            }
                        }
                        for possible_crtc in resources.filter_crtcs(encoder_info.possible_crtcs()) {
                            if !candidate_crtcs.contains(&possible_crtc) {
                                candidate_crtcs.push(possible_crtc);
                            }
                        }
                    }
                    if candidate_crtcs.is_empty() {
                        candidate_crtcs.extend(device.crtcs().iter().copied());
                    }

                    let selected_crtc_plane = candidate_crtcs
                        .iter()
                        .find_map(|candidate_crtc| {
                            primary_planes_by_crtc
                                .iter()
                                .find(|(crtc, _plane)| crtc == candidate_crtc)
                                .copied()
                        })
                        .or_else(|| primary_planes_by_crtc.first().copied());

                    if let Some((selected_crtc, selected_primary_plane)) = selected_crtc_plane {
                        let (mode_width, mode_height) = selected_mode.size();
                        probe.kms_scanout_plan_ready = true;
                        probe.kms_scanout_connector_id =
                            Into::<u32>::into(connector_info.handle()) as u64;
                        probe.kms_scanout_connector_name = Some(connector_info.to_string());
                        probe.kms_scanout_crtc_id = Into::<u32>::into(selected_crtc) as u64;
                        probe.kms_scanout_primary_plane_id =
                            Into::<u32>::into(selected_primary_plane) as u64;
                        probe.kms_scanout_mode_width = mode_width as u64;
                        probe.kms_scanout_mode_height = mode_height as u64;
                        probe.kms_scanout_mode_refresh_hz = selected_mode.vrefresh() as u64;
                        probe.kms_scanout_mode_preferred =
                            selected_mode.mode_type().contains(ModeTypeFlags::PREFERRED);
                        scanout_surface_target = Some((
                            connector_info.handle(),
                            selected_crtc,
                            selected_primary_plane,
                            selected_mode,
                        ));
                    }
                }
            }
        }
    }

    let mut event_loop = match EventLoop::<KmsEventState>::try_new() {
        Ok(event_loop) => event_loop,
        Err(error) => {
            probe.failure = Some(format!("kms-event-loop-new:{error}"));
            device.pause();
            return probe;
        }
    };

    probe.kms_event_source_inserted = event_loop
        .handle()
        .insert_source(notifier, |event, _metadata, data| match event {
            DrmEvent::VBlank(_) => data.vblank_count += 1,
            DrmEvent::Error(_) => data.error_count += 1,
        })
        .is_ok();
    if !probe.kms_event_source_inserted {
        probe.failure = Some(String::from("kms-event-source-insert"));
        device.pause();
        return probe;
    }

    let mut data = KmsEventState::default();
    probe.kms_event_loop_dispatched = event_loop
        .dispatch(Some(Duration::from_millis(0)), &mut data)
        .is_ok();

    if !probe.kms_event_loop_dispatched {
        probe.failure = Some(String::from("kms-event-loop-dispatch"));
    } else if probe.kms_crtc_count == 0 {
        probe.failure = Some(String::from("kms-no-crtcs"));
    } else if probe.kms_connector_count == 0 {
        probe.failure = Some(String::from("kms-no-connectors"));
    } else if probe.kms_connected_connector_count == 0 {
        probe.failure = Some(String::from("kms-no-connected-connectors"));
    } else if probe.kms_mode_count == 0 {
        probe.failure = Some(String::from("kms-no-modes"));
    } else if probe.kms_primary_plane_count == 0 {
        probe.failure = Some(String::from("kms-no-primary-planes"));
    } else if !probe.kms_scanout_plan_ready {
        probe.failure = Some(String::from("kms-no-scanout-plan"));
    } else if probe.kms_scanout_mode_refresh_hz == 0 {
        probe.failure = Some(String::from("kms-scanout-mode-refresh-zero"));
    }

    if probe.failure.is_none() {
        let Some((surface_connector, surface_crtc, surface_primary_plane, surface_mode)) =
            scanout_surface_target
        else {
            probe.surface_failure = Some(String::from("kms-surface-missing-target"));
            device.pause();
            return probe;
        };

        let surface = match device.create_surface(surface_crtc, surface_mode, &[surface_connector])
        {
            Ok(surface) => surface,
            Err(error) => {
                probe.surface_failure = Some(format!("kms-surface-create:{error:?}"));
                device.pause();
                return probe;
            }
        };

        probe.kms_surface_created = true;
        probe.kms_surface_legacy = surface.is_legacy();
        probe.kms_surface_crtc_matches_plan = surface.crtc() == surface_crtc
            && Into::<u32>::into(surface.crtc()) as u64 == probe.kms_scanout_crtc_id;
        probe.kms_surface_primary_plane_matches_plan = surface.plane() == surface_primary_plane
            && Into::<u32>::into(surface.plane()) as u64 == probe.kms_scanout_primary_plane_id;
        probe.kms_surface_pending_connector_count =
            surface.pending_connectors().into_iter().count() as u64;
        probe.kms_surface_current_connector_count =
            surface.current_connectors().into_iter().count() as u64;
        probe.kms_surface_pending_mode_matches_plan = surface.pending_mode() == surface_mode
            && surface.pending_mode().size().0 as u64 == probe.kms_scanout_mode_width
            && surface.pending_mode().size().1 as u64 == probe.kms_scanout_mode_height
            && surface.pending_mode().vrefresh() as u64 == probe.kms_scanout_mode_refresh_hz;
        probe.kms_surface_commit_pending = surface.commit_pending();

        if !probe.kms_surface_crtc_matches_plan {
            probe.surface_failure = Some(String::from("kms-surface-crtc-mismatch"));
        } else if !probe.kms_surface_primary_plane_matches_plan {
            probe.surface_failure = Some(String::from("kms-surface-plane-mismatch"));
        } else if probe.kms_surface_pending_connector_count == 0 {
            probe.surface_failure = Some(String::from("kms-surface-no-pending-connectors"));
        } else if !probe.kms_surface_pending_mode_matches_plan {
            probe.surface_failure = Some(String::from("kms-surface-pending-mode-mismatch"));
        }

        if probe.surface_failure.is_none() {
            let framebuffer_width = surface_mode.size().0 as u32;
            let framebuffer_height = surface_mode.size().1 as u32;
            probe.kms_framebuffer_width = framebuffer_width as u64;
            probe.kms_framebuffer_height = framebuffer_height as u64;
            probe.kms_framebuffer_primary_plane_matches_surface =
                surface.plane() == surface_primary_plane;
            probe.kms_framebuffer_test_allow_modeset =
                !surface.is_legacy() && surface.commit_pending();

            let mut allocator = DumbAllocator::new(surface.device_fd().clone());
            match allocator.create_buffer(
                framebuffer_width,
                framebuffer_height,
                Fourcc::Xrgb8888,
                &[Modifier::Linear],
            ) {
                Ok(dumb_buffer) => {
                    probe.kms_framebuffer_created = true;
                    match framebuffer_from_dumb_buffer(surface.device_fd(), &dumb_buffer, false) {
                        Ok(framebuffer) => {
                            probe.kms_framebuffer_added = true;
                            let framebuffer_handle = *framebuffer.as_ref();
                            let mut raw_buffer = *dumb_buffer.handle();
                            let stride = raw_buffer.pitch() as usize;
                            let visible_bytes_per_row = framebuffer_width as usize * 4;
                            let row_count = framebuffer_height as usize;
                            match surface.device_fd().map_dumb_buffer(&mut raw_buffer) {
                                Ok(mut mapping) => {
                                    let visible_len = stride
                                        .checked_mul(row_count.saturating_sub(1))
                                        .and_then(|base| base.checked_add(visible_bytes_per_row));
                                    let bytes: &mut [u8] = mapping.as_mut();
                                    if visible_len.is_some_and(|len| len <= bytes.len()) {
                                        for y in 0..row_count {
                                            let row_start = y * stride;
                                            let row_end = row_start + visible_bytes_per_row;
                                            for pixel in
                                                bytes[row_start..row_end].chunks_exact_mut(4)
                                            {
                                                pixel.copy_from_slice(&[0x9a, 0x64, 0x25, 0xff]);
                                            }
                                        }
                                        probe.kms_first_present_framebuffer_filled = true;
                                    } else {
                                        probe.first_present_failure =
                                            Some(String::from("kms-first-present-fill-bounds"));
                                    }
                                }
                                Err(error) => {
                                    probe.first_present_failure =
                                        Some(format!("kms-first-present-map:{error}"));
                                }
                            }

                            let build_plane_state = || PlaneState {
                                handle: surface.plane(),
                                config: Some(PlaneConfig {
                                    src: Rectangle::from_size(
                                        (framebuffer_width as i32, framebuffer_height as i32)
                                            .into(),
                                    )
                                    .to_f64(),
                                    dst: Rectangle::from_size(
                                        (framebuffer_width as i32, framebuffer_height as i32)
                                            .into(),
                                    ),
                                    transform: Transform::Normal,
                                    alpha: 1.0,
                                    damage_clips: None,
                                    fb: framebuffer_handle,
                                    fence: None,
                                }),
                            };
                            probe.kms_first_present_plane_state_ready = true;

                            match surface.test_state(
                                [build_plane_state()],
                                probe.kms_framebuffer_test_allow_modeset,
                            ) {
                                Ok(()) => {
                                    probe.kms_framebuffer_test_state_succeeded = true;
                                    if probe.kms_first_present_framebuffer_filled {
                                        let previous_vblank_count = data.vblank_count;
                                        let previous_error_count = data.error_count;
                                        probe.kms_first_present_commit_attempted = true;
                                        match surface.commit([build_plane_state()], true) {
                                            Ok(()) => {
                                                probe.kms_first_present_commit_succeeded = true;
                                                match event_loop.dispatch(
                                                    Some(Duration::from_millis(100)),
                                                    &mut data,
                                                ) {
                                                    Ok(())
                                                        if data.vblank_count
                                                            > previous_vblank_count =>
                                                    {
                                                        probe.kms_first_present_vblank_event_received =
                                                            true;
                                                    }
                                                    Ok(())
                                                        if data.error_count
                                                            > previous_error_count =>
                                                    {
                                                        probe.first_present_failure =
                                                            Some(String::from(
                                                                "kms-first-present-event-error",
                                                            ));
                                                    }
                                                    Ok(()) => {
                                                        probe.first_present_failure =
                                                            Some(String::from(
                                                                "kms-first-present-vblank-timeout",
                                                            ));
                                                    }
                                                    Err(error) => {
                                                        probe.first_present_failure = Some(format!(
                                                            "kms-first-present-event-dispatch:{error}"
                                                        ));
                                                    }
                                                }
                                            }
                                            Err(error) => {
                                                if matches!(
                                                    &error,
                                                    DrmError::Access(access)
                                                        if access.source.kind()
                                                            == std::io::ErrorKind::PermissionDenied
                                                ) {
                                                    probe.kms_first_present_blocked_by_drm_master =
                                                        true;
                                                } else {
                                                    probe.first_present_failure = Some(format!(
                                                        "kms-first-present-commit:{error:?}"
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(error) => {
                                    if matches!(
                                        &error,
                                        DrmError::Access(access)
                                            if access.source.kind()
                                                == std::io::ErrorKind::PermissionDenied
                                    ) {
                                        probe.kms_framebuffer_test_state_permission_denied = true;
                                        probe.kms_first_present_blocked_by_drm_master = true;
                                    } else {
                                        probe.framebuffer_failure =
                                            Some(format!("kms-framebuffer-test-state:{error:?}"));
                                    }
                                }
                            }

                            drop(framebuffer);
                            drop(dumb_buffer);
                            probe.kms_framebuffer_released_before_surface_drop = true;
                        }
                        Err(error) => {
                            probe.framebuffer_failure =
                                Some(format!("kms-framebuffer-add:{error:?}"));
                            drop(dumb_buffer);
                        }
                    }
                }
                Err(error) => {
                    probe.framebuffer_failure = Some(format!("kms-framebuffer-create:{error}"));
                }
            }

            if !probe.kms_framebuffer_primary_plane_matches_surface {
                probe.framebuffer_failure = Some(String::from("kms-framebuffer-plane-mismatch"));
            } else if probe.kms_framebuffer_width != probe.kms_scanout_mode_width
                || probe.kms_framebuffer_height != probe.kms_scanout_mode_height
            {
                probe.framebuffer_failure = Some(String::from("kms-framebuffer-size-mismatch"));
            } else if !probe.kms_framebuffer_created {
                probe.framebuffer_failure = Some(String::from("kms-framebuffer-not-created"));
            } else if !probe.kms_framebuffer_added {
                probe.framebuffer_failure = Some(String::from("kms-framebuffer-not-added"));
            } else if !probe.kms_framebuffer_test_state_succeeded
                && !probe.kms_framebuffer_test_state_permission_denied
            {
                probe
                    .framebuffer_failure
                    .get_or_insert_with(|| String::from("kms-framebuffer-test-state-failed"));
            } else if !probe.kms_framebuffer_released_before_surface_drop {
                probe.framebuffer_failure = Some(String::from("kms-framebuffer-not-released"));
            }
        }

        device.pause();
        drop(surface);
        probe.kms_surface_dropped_after_pause = true;
        return probe;
    }

    device.pause();
    probe
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn smithay_kms_runtime_probe(
    _target_os: &str,
    _primary_drm_card: Option<&str>,
) -> SmithayKmsRuntimeProbe {
    unavailable_smithay_kms_runtime_probe("unavailable")
}

fn unavailable_smithay_kms_runtime_probe(reason: impl Into<String>) -> SmithayKmsRuntimeProbe {
    let reason = reason.into();
    SmithayKmsRuntimeProbe {
        kms_card_opened: false,
        kms_device_created: false,
        kms_event_source_inserted: false,
        kms_event_loop_dispatched: false,
        kms_atomic_modesetting: false,
        kms_crtc_count: 0,
        kms_connector_count: 0,
        kms_connected_connector_count: 0,
        kms_mode_count: 0,
        kms_primary_plane_count: 0,
        kms_cursor_plane_count: 0,
        kms_overlay_plane_count: 0,
        kms_scanout_plan_ready: false,
        kms_scanout_connector_id: 0,
        kms_scanout_connector_name: None,
        kms_scanout_crtc_id: 0,
        kms_scanout_primary_plane_id: 0,
        kms_scanout_mode_width: 0,
        kms_scanout_mode_height: 0,
        kms_scanout_mode_refresh_hz: 0,
        kms_scanout_mode_preferred: false,
        kms_surface_created: false,
        kms_surface_legacy: false,
        kms_surface_crtc_matches_plan: false,
        kms_surface_primary_plane_matches_plan: false,
        kms_surface_pending_connector_count: 0,
        kms_surface_current_connector_count: 0,
        kms_surface_pending_mode_matches_plan: false,
        kms_surface_commit_pending: false,
        kms_surface_dropped_after_pause: false,
        kms_framebuffer_created: false,
        kms_framebuffer_added: false,
        kms_framebuffer_test_state_succeeded: false,
        kms_framebuffer_test_state_permission_denied: false,
        kms_framebuffer_test_allow_modeset: false,
        kms_framebuffer_primary_plane_matches_surface: false,
        kms_framebuffer_width: 0,
        kms_framebuffer_height: 0,
        kms_framebuffer_released_before_surface_drop: false,
        kms_first_present_framebuffer_filled: false,
        kms_first_present_plane_state_ready: false,
        kms_first_present_commit_attempted: false,
        kms_first_present_commit_succeeded: false,
        kms_first_present_vblank_event_received: false,
        kms_first_present_blocked_by_drm_master: false,
        first_present_failure: Some(reason.clone()),
        framebuffer_failure: Some(reason.clone()),
        surface_failure: Some(reason.clone()),
        failure: Some(reason),
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn smithay_renderer_runtime_probe(
    target_os: &str,
    renderer_node_path: Option<&str>,
) -> SmithayRendererRuntimeProbe {
    use std::os::unix::io::OwnedFd;

    use smithay::backend::allocator::gbm::{GbmAllocator, GbmBufferFlags, GbmDevice};
    use smithay::backend::allocator::Fourcc;
    use smithay::backend::drm::DrmDeviceFd;
    use smithay::backend::egl::{EGLContext, EGLDisplay};
    use smithay::backend::renderer::gles::{GlesRenderbuffer, GlesRenderer};
    use smithay::backend::renderer::{Bind, Color32F, ExportMem, Frame, Offscreen, Renderer};
    use smithay::utils::{DeviceFd, Rectangle, Transform};

    const OFFSCREEN_WIDTH: i32 = 16;
    const OFFSCREEN_HEIGHT: i32 = 16;
    const EXPECTED_RED: u8 = 255;
    const EXPECTED_GREEN: u8 = 0;
    const EXPECTED_BLUE: u8 = 0;
    const EXPECTED_ALPHA: u8 = 255;

    if target_os != "linux" {
        return unavailable_smithay_renderer_runtime_probe("non-linux-target");
    }

    let Some(path) = renderer_node_path.filter(|value| !value.trim().is_empty()) else {
        return unavailable_smithay_renderer_runtime_probe("missing-render-node");
    };

    let file = match std::fs::File::options().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(error) => {
            return unavailable_smithay_renderer_runtime_probe(format!("render-node-open:{error}"));
        }
    };

    let drm_fd = DrmDeviceFd::new(DeviceFd::from(Into::<OwnedFd>::into(file)));
    let gbm = match GbmDevice::new(drm_fd) {
        Ok(gbm) => gbm,
        Err(error) => {
            return SmithayRendererRuntimeProbe {
                renderer_node_opened: true,
                failure: Some(format!("gbm-device:{error:?}")),
                ..unavailable_smithay_renderer_runtime_probe("gbm-device")
            };
        }
    };

    let _allocator = GbmAllocator::new(gbm.clone(), GbmBufferFlags::RENDERING);

    let egl_display = match unsafe { EGLDisplay::new(gbm.clone()) } {
        Ok(display) => display,
        Err(error) => {
            return SmithayRendererRuntimeProbe {
                renderer_node_opened: true,
                gbm_device_created: true,
                gbm_allocator_created: true,
                failure: Some(format!("egl-display:{error:?}")),
                ..unavailable_smithay_renderer_runtime_probe("egl-display")
            };
        }
    };

    let egl_context = match EGLContext::new(&egl_display) {
        Ok(context) => context,
        Err(error) => {
            return SmithayRendererRuntimeProbe {
                renderer_node_opened: true,
                gbm_device_created: true,
                gbm_allocator_created: true,
                egl_display_created: true,
                failure: Some(format!("egl-context:{error:?}")),
                ..unavailable_smithay_renderer_runtime_probe("egl-context")
            };
        }
    };

    let mut renderer = match unsafe { GlesRenderer::new(egl_context) } {
        Ok(renderer) => renderer,
        Err(error) => {
            return SmithayRendererRuntimeProbe {
                renderer_node_opened: true,
                gbm_device_created: true,
                gbm_allocator_created: true,
                egl_display_created: true,
                egl_context_created: true,
                failure: Some(format!("gles-renderer:{error:?}")),
                ..unavailable_smithay_renderer_runtime_probe("gles-renderer")
            };
        }
    };

    let mut offscreen = match Offscreen::<GlesRenderbuffer>::create_buffer(
        &mut renderer,
        Fourcc::Abgr8888,
        (OFFSCREEN_WIDTH, OFFSCREEN_HEIGHT).into(),
    ) {
        Ok(offscreen) => offscreen,
        Err(error) => {
            return SmithayRendererRuntimeProbe {
                renderer_node_opened: true,
                gbm_device_created: true,
                gbm_allocator_created: true,
                egl_display_created: true,
                egl_context_created: true,
                gles_renderer_created: true,
                failure: Some(format!("offscreen-buffer:{error:?}")),
                ..unavailable_smithay_renderer_runtime_probe("offscreen-buffer")
            };
        }
    };

    let mut framebuffer = match renderer.bind(&mut offscreen) {
        Ok(framebuffer) => framebuffer,
        Err(error) => {
            return SmithayRendererRuntimeProbe {
                renderer_node_opened: true,
                gbm_device_created: true,
                gbm_allocator_created: true,
                egl_display_created: true,
                egl_context_created: true,
                gles_renderer_created: true,
                offscreen_buffer_created: true,
                failure: Some(format!("offscreen-bind:{error:?}")),
                ..unavailable_smithay_renderer_runtime_probe("offscreen-bind")
            };
        }
    };

    let mut frame = match renderer.render(
        &mut framebuffer,
        (OFFSCREEN_WIDTH, OFFSCREEN_HEIGHT).into(),
        Transform::Normal,
    ) {
        Ok(frame) => frame,
        Err(error) => {
            return SmithayRendererRuntimeProbe {
                renderer_node_opened: true,
                gbm_device_created: true,
                gbm_allocator_created: true,
                egl_display_created: true,
                egl_context_created: true,
                gles_renderer_created: true,
                offscreen_buffer_created: true,
                failure: Some(format!("offscreen-render:{error:?}")),
                ..unavailable_smithay_renderer_runtime_probe("offscreen-render")
            };
        }
    };

    let full_damage = [Rectangle::from_size(
        (OFFSCREEN_WIDTH, OFFSCREEN_HEIGHT).into(),
    )];
    if let Err(error) = frame.clear(Color32F::new(1.0, 0.0, 0.0, 1.0), &full_damage) {
        return SmithayRendererRuntimeProbe {
            renderer_node_opened: true,
            gbm_device_created: true,
            gbm_allocator_created: true,
            egl_display_created: true,
            egl_context_created: true,
            gles_renderer_created: true,
            offscreen_buffer_created: true,
            failure: Some(format!("offscreen-clear:{error:?}")),
            ..unavailable_smithay_renderer_runtime_probe("offscreen-clear")
        };
    }

    match frame.finish() {
        Ok(sync) => {
            if let Err(error) = sync.wait() {
                return SmithayRendererRuntimeProbe {
                    renderer_node_opened: true,
                    gbm_device_created: true,
                    gbm_allocator_created: true,
                    egl_display_created: true,
                    egl_context_created: true,
                    gles_renderer_created: true,
                    offscreen_buffer_created: true,
                    failure: Some(format!("offscreen-sync:{error:?}")),
                    ..unavailable_smithay_renderer_runtime_probe("offscreen-sync")
                };
            }
        }
        Err(error) => {
            return SmithayRendererRuntimeProbe {
                renderer_node_opened: true,
                gbm_device_created: true,
                gbm_allocator_created: true,
                egl_display_created: true,
                egl_context_created: true,
                gles_renderer_created: true,
                offscreen_buffer_created: true,
                failure: Some(format!("offscreen-finish:{error:?}")),
                ..unavailable_smithay_renderer_runtime_probe("offscreen-finish")
            };
        }
    }

    let mapping = match renderer.copy_framebuffer(
        &framebuffer,
        Rectangle::from_size((OFFSCREEN_WIDTH, OFFSCREEN_HEIGHT).into()),
        Fourcc::Abgr8888,
    ) {
        Ok(mapping) => mapping,
        Err(error) => {
            return SmithayRendererRuntimeProbe {
                renderer_node_opened: true,
                gbm_device_created: true,
                gbm_allocator_created: true,
                egl_display_created: true,
                egl_context_created: true,
                gles_renderer_created: true,
                offscreen_buffer_created: true,
                offscreen_frame_rendered: true,
                failure: Some(format!("offscreen-copy:{error:?}")),
                ..unavailable_smithay_renderer_runtime_probe("offscreen-copy")
            };
        }
    };

    let pixels = match renderer.map_texture(&mapping) {
        Ok(pixels) => pixels,
        Err(error) => {
            return SmithayRendererRuntimeProbe {
                renderer_node_opened: true,
                gbm_device_created: true,
                gbm_allocator_created: true,
                egl_display_created: true,
                egl_context_created: true,
                gles_renderer_created: true,
                offscreen_buffer_created: true,
                offscreen_frame_rendered: true,
                offscreen_frame_copied: true,
                failure: Some(format!("offscreen-map:{error:?}")),
                ..unavailable_smithay_renderer_runtime_probe("offscreen-map")
            };
        }
    };

    let sample = pixels.get(0..4).unwrap_or(&[]);
    let expected_sample = [EXPECTED_RED, EXPECTED_GREEN, EXPECTED_BLUE, EXPECTED_ALPHA];
    let pixel_verified = sample == expected_sample.as_slice();
    let sample_red = sample.first().copied().unwrap_or(0);
    let sample_green = sample.get(1).copied().unwrap_or(0);
    let sample_blue = sample.get(2).copied().unwrap_or(0);
    let sample_alpha = sample.get(3).copied().unwrap_or(0);
    let expected_len = (OFFSCREEN_WIDTH * OFFSCREEN_HEIGHT * 4) as usize;
    if pixels.len() < expected_len || !pixel_verified {
        return SmithayRendererRuntimeProbe {
            renderer_node_opened: true,
            gbm_device_created: true,
            gbm_allocator_created: true,
            egl_display_created: true,
            egl_context_created: true,
            gles_renderer_created: true,
            offscreen_buffer_created: true,
            offscreen_frame_rendered: true,
            offscreen_frame_copied: true,
            offscreen_pixel_verified: false,
            offscreen_render_width: OFFSCREEN_WIDTH as u64,
            offscreen_render_height: OFFSCREEN_HEIGHT as u64,
            offscreen_render_pixels: (pixels.len() / 4) as u64,
            offscreen_sample_red: sample_red as u64,
            offscreen_sample_green: sample_green as u64,
            offscreen_sample_blue: sample_blue as u64,
            offscreen_sample_alpha: sample_alpha as u64,
            failure: Some(String::from("offscreen-pixel-verify")),
        };
    }

    SmithayRendererRuntimeProbe {
        renderer_node_opened: true,
        gbm_device_created: true,
        gbm_allocator_created: true,
        egl_display_created: true,
        egl_context_created: true,
        gles_renderer_created: true,
        offscreen_buffer_created: true,
        offscreen_frame_rendered: true,
        offscreen_frame_copied: true,
        offscreen_pixel_verified: true,
        offscreen_render_width: OFFSCREEN_WIDTH as u64,
        offscreen_render_height: OFFSCREEN_HEIGHT as u64,
        offscreen_render_pixels: (pixels.len() / 4) as u64,
        offscreen_sample_red: sample_red as u64,
        offscreen_sample_green: sample_green as u64,
        offscreen_sample_blue: sample_blue as u64,
        offscreen_sample_alpha: sample_alpha as u64,
        failure: None,
    }
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn smithay_renderer_runtime_probe(
    _target_os: &str,
    _renderer_node_path: Option<&str>,
) -> SmithayRendererRuntimeProbe {
    unavailable_smithay_renderer_runtime_probe("unavailable")
}

fn unavailable_smithay_renderer_runtime_probe(
    reason: impl Into<String>,
) -> SmithayRendererRuntimeProbe {
    SmithayRendererRuntimeProbe {
        renderer_node_opened: false,
        gbm_device_created: false,
        gbm_allocator_created: false,
        egl_display_created: false,
        egl_context_created: false,
        gles_renderer_created: false,
        offscreen_buffer_created: false,
        offscreen_frame_rendered: false,
        offscreen_frame_copied: false,
        offscreen_pixel_verified: false,
        offscreen_render_width: 0,
        offscreen_render_height: 0,
        offscreen_render_pixels: 0,
        offscreen_sample_red: 0,
        offscreen_sample_green: 0,
        offscreen_sample_blue: 0,
        offscreen_sample_alpha: 0,
        failure: Some(reason.into()),
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn smithay_drm_node_type_name(node_type: smithay::backend::drm::NodeType) -> &'static str {
    match node_type {
        smithay::backend::drm::NodeType::Primary => "primary",
        smithay::backend::drm::NodeType::Control => "control",
        smithay::backend::drm::NodeType::Render => "render",
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn smithay_input_runtime_probe(
    target_os: &str,
    input_broker_ready: bool,
    seat: Option<&str>,
) -> SmithayInputRuntimeProbe {
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };
    use std::time::Duration;

    use smithay::backend::libinput::{LibinputInputBackend, LibinputSessionInterface};
    use smithay::backend::session::{libseat::LibSeatSession, Session};
    use smithay::reexports::calloop::EventLoop;

    if target_os != "linux" {
        return unavailable_smithay_input_runtime_probe("non-linux-target");
    }

    if !input_broker_ready {
        return unavailable_smithay_input_runtime_probe("input-broker-not-ready");
    }

    let Some(expected_seat) = seat.filter(|value| !value.trim().is_empty()) else {
        return unavailable_smithay_input_runtime_probe("missing-seat");
    };

    let (session, session_notifier) = match LibSeatSession::new() {
        Ok(session) => session,
        Err(error) => {
            return unavailable_smithay_input_runtime_probe(format!("libseat-session:{error:?}"));
        }
    };
    let session_active = session.is_active();
    let session_seat = session.seat();
    if session_seat != expected_seat {
        return SmithayInputRuntimeProbe {
            libseat_session_created: true,
            libseat_session_active: session_active,
            libseat_session_seat: Some(session_seat),
            failure: Some(format!("seat-mismatch:expected-{expected_seat}")),
            ..unavailable_smithay_input_runtime_probe("seat-mismatch")
        };
    }

    let mut libinput_context =
        input::Libinput::new_with_udev(LibinputSessionInterface::from(session));
    if libinput_context.udev_assign_seat(expected_seat).is_err() {
        return SmithayInputRuntimeProbe {
            libseat_session_created: true,
            libseat_session_active: session_active,
            libseat_session_seat: Some(session_seat),
            libinput_context_created: true,
            failure: Some(String::from("libinput-assign-seat")),
            ..unavailable_smithay_input_runtime_probe("libinput-assign-seat")
        };
    }

    let libinput_backend = LibinputInputBackend::new(libinput_context);
    let mut event_loop = match EventLoop::<()>::try_new() {
        Ok(event_loop) => event_loop,
        Err(error) => {
            return SmithayInputRuntimeProbe {
                libseat_session_created: true,
                libseat_session_active: session_active,
                libseat_session_seat: Some(session_seat),
                libinput_context_created: true,
                libinput_seat_assigned: true,
                libinput_backend_created: true,
                failure: Some(format!("event-loop-new:{error}")),
                ..unavailable_smithay_input_runtime_probe("event-loop-new")
            };
        }
    };

    let libseat_session_events = Arc::new(AtomicU64::new(0));
    let libinput_events = Arc::new(AtomicU64::new(0));
    let session_events_for_callback = Arc::clone(&libseat_session_events);
    let input_events_for_callback = Arc::clone(&libinput_events);

    let libseat_event_source_inserted = event_loop
        .handle()
        .insert_source(session_notifier, move |_event, _metadata, _data| {
            session_events_for_callback.fetch_add(1, Ordering::SeqCst);
        })
        .is_ok();

    if !libseat_event_source_inserted {
        return SmithayInputRuntimeProbe {
            libseat_session_created: true,
            libseat_session_active: session_active,
            libseat_session_seat: Some(session_seat),
            libinput_context_created: true,
            libinput_seat_assigned: true,
            libinput_backend_created: true,
            failure: Some(String::from("libseat-event-source-insert")),
            ..unavailable_smithay_input_runtime_probe("libseat-event-source-insert")
        };
    }

    let libinput_event_source_inserted = event_loop
        .handle()
        .insert_source(libinput_backend, move |_event, _metadata, _data| {
            input_events_for_callback.fetch_add(1, Ordering::SeqCst);
        })
        .is_ok();

    if !libinput_event_source_inserted {
        return SmithayInputRuntimeProbe {
            libseat_session_created: true,
            libseat_session_active: session_active,
            libseat_session_seat: Some(session_seat),
            libseat_event_source_inserted: true,
            libinput_context_created: true,
            libinput_seat_assigned: true,
            libinput_backend_created: true,
            failure: Some(String::from("libinput-event-source-insert")),
            ..unavailable_smithay_input_runtime_probe("libinput-event-source-insert")
        };
    }

    let mut data = ();
    let dispatched = event_loop
        .dispatch(Some(Duration::from_millis(0)), &mut data)
        .is_ok();

    SmithayInputRuntimeProbe {
        libseat_session_created: true,
        libseat_session_active: session_active,
        libseat_session_seat: Some(session_seat),
        libseat_event_source_inserted: true,
        libseat_event_loop_dispatched: dispatched,
        libseat_session_event_count: libseat_session_events.load(Ordering::SeqCst),
        libinput_context_created: true,
        libinput_seat_assigned: true,
        libinput_backend_created: true,
        libinput_event_source_inserted: true,
        libinput_event_loop_dispatched: dispatched,
        libinput_event_count: libinput_events.load(Ordering::SeqCst),
        failure: if dispatched {
            None
        } else {
            Some(String::from("event-loop-dispatch"))
        },
    }
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn smithay_input_runtime_probe(
    _target_os: &str,
    _input_broker_ready: bool,
    _seat: Option<&str>,
) -> SmithayInputRuntimeProbe {
    unavailable_smithay_input_runtime_probe("unavailable")
}

fn unavailable_smithay_input_runtime_probe(reason: impl Into<String>) -> SmithayInputRuntimeProbe {
    SmithayInputRuntimeProbe {
        libseat_session_created: false,
        libseat_session_active: false,
        libseat_session_seat: None,
        libseat_event_source_inserted: false,
        libseat_event_loop_dispatched: false,
        libseat_session_event_count: 0,
        libinput_context_created: false,
        libinput_seat_assigned: false,
        libinput_backend_created: false,
        libinput_event_source_inserted: false,
        libinput_event_loop_dispatched: false,
        libinput_event_count: 0,
        failure: Some(reason.into()),
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn smithay_runtime_components() -> Vec<&'static str> {
    let _ = std::any::type_name::<smithay::backend::drm::DrmNode>();
    let _ = std::any::type_name::<smithay::backend::allocator::gbm::GbmAllocator<std::fs::File>>();
    let _ = std::any::type_name::<smithay::backend::egl::EGLDisplay>();
    let _ = std::any::type_name::<smithay::backend::renderer::gles::GlesRenderer>();
    let _ = std::any::type_name::<smithay::backend::libinput::LibinputInputBackend>();
    let _ = std::any::type_name::<smithay::backend::session::Event>();
    let _ = std::any::type_name::<smithay::reexports::calloop::LoopSignal>();
    let _ = std::any::type_name::<smithay::reexports::wayland_server::DisplayHandle>();

    vec![
        "smithay::backend::drm",
        "smithay::backend::allocator::gbm",
        "smithay::backend::egl",
        "smithay::backend::renderer::gles",
        "smithay::backend::libinput",
        "smithay::backend::session",
        "smithay::reexports::calloop",
        "smithay::reexports::wayland_server",
    ]
}

#[cfg(not(all(feature = "smithay-backend", target_os = "linux")))]
fn smithay_runtime_components() -> Vec<&'static str> {
    Vec::new()
}

pub fn backend_launch_plan(
    backend: BackendKind,
    report: &BackendPreflightReport,
    environment: &BackendPreflightEnvironment,
) -> BackendLaunchPlan {
    match backend {
        BackendKind::Headless => BackendLaunchPlan {
            backend,
            ready: report.ready,
            implementation: "headless-harness",
            display_driver: "headless",
            input_driver: "synthetic",
            device_access: "none",
            uses_parent_wayland: false,
            uses_drm: false,
            uses_logind: false,
            uses_libseat: false,
            uses_libinput: false,
            drm_card_selected: false,
            drm_render_selected: false,
            input_event_selected: false,
            primary_drm_card: None,
            primary_drm_render_node: None,
            primary_input_event: None,
            session_id: None,
            seat: None,
            session_type: None,
        },
        BackendKind::Wayland => BackendLaunchPlan {
            backend,
            ready: report.ready,
            implementation: "nested-wayland-harness",
            display_driver: if environment.wayland_display.is_some() {
                "parent-wayland"
            } else {
                "missing-parent-wayland"
            },
            input_driver: if report.ready {
                "parent-wayland-seat"
            } else {
                "unavailable"
            },
            device_access: if report.ready {
                "parent-wayland-socket"
            } else {
                "unavailable"
            },
            uses_parent_wayland: environment.wayland_display.is_some(),
            uses_drm: false,
            uses_logind: false,
            uses_libseat: false,
            uses_libinput: false,
            drm_card_selected: false,
            drm_render_selected: false,
            input_event_selected: false,
            primary_drm_card: None,
            primary_drm_render_node: None,
            primary_input_event: None,
            session_id: None,
            seat: None,
            session_type: None,
        },
        BackendKind::Drm => {
            let input_driver = if environment.input_event_nodes == 0 {
                "missing-input"
            } else if environment.input_event_readable > 0 {
                "direct-libinput"
            } else if environment.input_broker_ready() {
                "logind-libseat-libinput"
            } else {
                "unavailable"
            };
            let device_access = if !environment.drm_card_access_ready() {
                "missing-drm-card"
            } else if input_driver == "direct-libinput" {
                "drm-card-direct-input"
            } else if input_driver == "logind-libseat-libinput" {
                "drm-card-logind-libseat"
            } else {
                "unavailable"
            };

            BackendLaunchPlan {
                backend,
                ready: report.ready,
                implementation: "pre-smithay-policy-harness",
                display_driver: if environment.drm_card_access_ready() {
                    "drm-kms"
                } else {
                    "unavailable"
                },
                input_driver,
                device_access,
                uses_parent_wayland: false,
                uses_drm: environment.drm_card_access_ready(),
                uses_logind: environment.logind_session_verified || environment.logind_available,
                uses_libseat: input_driver == "logind-libseat-libinput",
                uses_libinput: input_driver == "direct-libinput"
                    || input_driver == "logind-libseat-libinput",
                drm_card_selected: environment.primary_drm_card.is_some(),
                drm_render_selected: environment.primary_drm_render_node.is_some(),
                input_event_selected: environment.primary_input_event.is_some(),
                primary_drm_card: environment.primary_drm_card.clone(),
                primary_drm_render_node: environment.primary_drm_render_node.clone(),
                primary_input_event: environment.primary_input_event.clone(),
                session_id: environment.session_id.clone(),
                seat: environment.seat.clone(),
                session_type: environment.session_type.clone(),
            }
        }
    }
}

impl BackendPreflightReport {
    pub fn ready(backend: BackendKind, code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            backend,
            ready: true,
            code,
            detail: detail.into(),
        }
    }

    pub fn blocked(backend: BackendKind, code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            backend,
            ready: false,
            code,
            detail: detail.into(),
        }
    }
}

pub fn preflight_backend(
    backend: BackendKind,
    wayland_display: Option<&str>,
    xdg_runtime_dir: Option<&str>,
    target_os: &str,
) -> BackendPreflightReport {
    let mut environment = BackendPreflightEnvironment::for_target(target_os);
    environment.wayland_display = wayland_display.map(str::to_string);
    if let Some(xdg_runtime_dir) = xdg_runtime_dir {
        environment = environment.with_xdg_runtime_dir(xdg_runtime_dir);
    }

    preflight_backend_with_environment(backend, &environment)
}

pub fn preflight_backend_with_environment(
    backend: BackendKind,
    environment: &BackendPreflightEnvironment,
) -> BackendPreflightReport {
    match backend {
        BackendKind::Headless => BackendPreflightReport::ready(
            backend,
            "ready",
            "headless backend does not require host display state",
        ),
        BackendKind::Wayland => preflight_wayland(environment),
        BackendKind::Drm => preflight_drm(environment),
    }
}

fn preflight_wayland(environment: &BackendPreflightEnvironment) -> BackendPreflightReport {
    if missing(environment.wayland_display.as_deref()) {
        return BackendPreflightReport::blocked(
            BackendKind::Wayland,
            "missing-wayland-display",
            "nested Wayland backend requires WAYLAND_DISPLAY from a parent compositor",
        );
    }

    if missing(environment.xdg_runtime_dir.as_deref()) || !environment.xdg_runtime_dir_present {
        return BackendPreflightReport::blocked(
            BackendKind::Wayland,
            "missing-xdg-runtime-dir",
            "nested Wayland backend requires XDG_RUNTIME_DIR for socket discovery",
        );
    }

    if !environment.xdg_runtime_dir_owned_by_user {
        return BackendPreflightReport::blocked(
            BackendKind::Wayland,
            "wrong-xdg-runtime-dir-owner",
            "nested Wayland backend requires XDG_RUNTIME_DIR owned by the launching user",
        );
    }

    BackendPreflightReport::ready(
        BackendKind::Wayland,
        "ready",
        "nested Wayland environment variables are present",
    )
}

fn preflight_drm(environment: &BackendPreflightEnvironment) -> BackendPreflightReport {
    if environment.target_os != "linux" {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "requires-linux",
            "DRM/KMS backend requires Linux with a real graphics/input stack",
        );
    }

    if missing(environment.xdg_runtime_dir.as_deref()) || !environment.xdg_runtime_dir_present {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "missing-xdg-runtime-dir",
            "DRM/KMS backend expects XDG_RUNTIME_DIR from the session environment",
        );
    }

    if !environment.xdg_runtime_dir_owned_by_user {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "wrong-xdg-runtime-dir-owner",
            "DRM/KMS backend requires XDG_RUNTIME_DIR owned by the launching user",
        );
    }

    if environment.drm_card_nodes == 0 {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "missing-drm-card",
            "DRM/KMS backend requires at least one /dev/dri/card* node for mode setting",
        );
    }

    if !environment.drm_card_access_ready() {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "unavailable-drm-card-access",
            "DRM/KMS backend requires read/write access to a DRM card node",
        );
    }

    if environment.input_event_nodes == 0 {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "missing-input-devices",
            "libinput backend requires /dev/input/event* devices",
        );
    }

    if missing(environment.session_id.as_deref()) {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "missing-logind-session",
            "DRM/KMS backend expects an XDG_SESSION_ID so logind/libseat can authorize devices",
        );
    }

    if !environment.logind_session_verified {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "unverified-logind-session",
            "DRM/KMS backend expects loginctl session metadata for active/local seat validation",
        );
    }

    if !environment.session_active {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "inactive-logind-session",
            "DRM/KMS backend requires an active logind session",
        );
    }

    if environment.session_remote {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "remote-logind-session",
            "DRM/KMS backend requires a local logind session, not a remote one",
        );
    }

    if missing(environment.seat.as_deref()) {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "missing-seat",
            "DRM/KMS backend requires a local logind seat",
        );
    }

    if missing(environment.session_type.as_deref())
        || environment.session_type.as_deref() == Some("unspecified")
    {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "unspecified-session-type",
            "DRM/KMS backend requires a concrete logind session type such as tty or wayland",
        );
    }

    if !environment.input_broker_ready() {
        return BackendPreflightReport::blocked(
            BackendKind::Drm,
            "missing-input-broker",
            "DRM/KMS backend requires direct input event access or logind/libseat/libinput brokering",
        );
    }

    BackendPreflightReport::ready(
        BackendKind::Drm,
        "ready-active-local-session-input-broker",
        format!(
            "Linux launch environment has XDG_RUNTIME_DIR, {} DRM card nodes, {} DRM render nodes, {} input event nodes, {} input broker, and active local {} session {} on {}",
            environment.drm_card_nodes,
            environment.drm_render_nodes,
            environment.input_event_nodes,
            environment.input_broker_mode(),
            environment.session_type.as_deref().unwrap_or("unknown"),
            environment.session_id.as_deref().unwrap_or("unknown"),
            environment.seat.as_deref().unwrap_or("unknown")
        ),
    )
}

fn count_entries_with_prefix(dir: &str, prefix: &str) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };

    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with(prefix))
                .unwrap_or(false)
        })
        .count() as u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccessMode {
    Read,
    Write,
}

fn count_openable_entries_with_prefix(dir: &str, prefix: &str, mode: AccessMode) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };

    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with(prefix))
                .unwrap_or(false)
        })
        .filter(|entry| open_device_node(entry.path().as_path(), mode))
        .count() as u64
}

fn first_entry_with_prefix(dir: &str, prefix: &str) -> Option<String> {
    sorted_entries_with_prefix(dir, prefix)
        .into_iter()
        .next()
        .map(path_to_string)
}

fn first_openable_entry_with_prefix(
    dir: &str,
    prefix: &str,
    access_modes: &[AccessMode],
) -> Option<String> {
    sorted_entries_with_prefix(dir, prefix)
        .into_iter()
        .find(|path| {
            access_modes
                .iter()
                .all(|mode| open_device_node(path.as_path(), *mode))
        })
        .map(path_to_string)
}

fn sorted_entries_with_prefix(dir: &str, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with(prefix))
                .unwrap_or(false)
        })
        .map(|entry| entry.path())
        .collect();
    paths.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
    paths
}

fn path_to_string(path: PathBuf) -> String {
    path.display().to_string()
}

fn open_device_node(path: &std::path::Path, mode: AccessMode) -> bool {
    let mut options = fs::OpenOptions::new();
    match mode {
        AccessMode::Read => {
            options.read(true);
        }
        AccessMode::Write => {
            options.write(true);
        }
    }

    options.open(path).is_ok()
}

#[cfg(target_os = "linux")]
fn command_available(command: &str, target_os: &str) -> bool {
    if target_os != "linux" {
        return false;
    }

    let script = format!("command -v {command} >/dev/null 2>&1");
    Command::new("sh")
        .arg("-c")
        .arg(script)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn command_available(_command: &str, _target_os: &str) -> bool {
    false
}

#[cfg(target_os = "linux")]
fn pkg_config_package_available(package: &str, target_os: &str) -> bool {
    if target_os != "linux" {
        return false;
    }

    Command::new("pkg-config")
        .args(["--exists", package])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn pkg_config_package_available(_package: &str, _target_os: &str) -> bool {
    false
}

fn missing(value: Option<&str>) -> bool {
    match value {
        Some(value) => value.trim().is_empty(),
        None => true,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeDirStatus {
    present: bool,
    owned_by_user: bool,
}

fn runtime_dir_status(path: Option<&str>, target_os: &str) -> RuntimeDirStatus {
    let Some(path) = path else {
        return RuntimeDirStatus {
            present: false,
            owned_by_user: false,
        };
    };

    let Ok(metadata) = fs::metadata(path) else {
        return RuntimeDirStatus {
            present: false,
            owned_by_user: false,
        };
    };

    if !metadata.is_dir() {
        return RuntimeDirStatus {
            present: false,
            owned_by_user: false,
        };
    }

    RuntimeDirStatus {
        present: true,
        owned_by_user: runtime_dir_owned_by_user(&metadata, target_os),
    }
}

#[cfg(target_os = "linux")]
fn runtime_dir_owned_by_user(metadata: &fs::Metadata, target_os: &str) -> bool {
    if target_os != "linux" {
        return true;
    }

    current_effective_uid()
        .map(|uid| metadata.uid() == uid)
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn runtime_dir_owned_by_user(_metadata: &fs::Metadata, _target_os: &str) -> bool {
    true
}

#[cfg(target_os = "linux")]
fn current_effective_uid() -> Option<u32> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let rest = line.strip_prefix("Uid:")?;
        rest.split_whitespace().nth(1)?.parse().ok()
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LogindSessionStatus {
    active: bool,
    remote: bool,
    seat: String,
    session_type: String,
    state: String,
}

#[cfg(target_os = "linux")]
fn logind_session_status(session_id: &str, target_os: &str) -> Option<LogindSessionStatus> {
    if target_os != "linux" {
        return None;
    }

    let output = Command::new("loginctl")
        .args([
            "show-session",
            session_id,
            "-p",
            "Active",
            "-p",
            "Remote",
            "-p",
            "Seat",
            "-p",
            "Type",
            "-p",
            "State",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut status = LogindSessionStatus {
        active: false,
        remote: false,
        seat: String::new(),
        session_type: String::new(),
        state: String::new(),
    };

    for line in stdout.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "Active" => status.active = value == "yes",
            "Remote" => status.remote = value == "yes",
            "Seat" => status.seat = value.to_string(),
            "Type" => status.session_type = value.to_string(),
            "State" => status.state = value.to_string(),
            _ => {}
        }
    }

    Some(status)
}

#[cfg(not(target_os = "linux"))]
fn logind_session_status(_session_id: &str, _target_os: &str) -> Option<LogindSessionStatus> {
    None
}

fn string_option(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

impl FromStr for BackendKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "headless" => Ok(Self::Headless),
            "wayland" | "nested" => Ok(Self::Wayland),
            "drm" | "kms" => Ok(Self::Drm),
            other => Err(format!("unknown backend '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKind {
    Headless,
    Smithay,
}

impl RuntimeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Headless => "headless",
            Self::Smithay => "smithay",
        }
    }
}

impl FromStr for RuntimeKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "headless" => Ok(Self::Headless),
            "smithay" => Ok(Self::Smithay),
            other => Err(format!("unknown runtime '{other}'")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    pub backend: BackendKind,
    pub runtime: RuntimeKind,
    pub socket: String,
    pub smoke_test: bool,
    pub scripted_client: bool,
    pub scripted_client_preview: Option<String>,
    pub smithay_client_smoke: bool,
    pub serve: bool,
    pub serve_for_ms: Option<u64>,
    pub idle_probe_ms: Option<u64>,
    pub help: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            backend: BackendKind::Headless,
            runtime: RuntimeKind::Headless,
            socket: String::from("backlit-0"),
            smoke_test: false,
            scripted_client: false,
            scripted_client_preview: None,
            smithay_client_smoke: false,
            serve: false,
            serve_for_ms: None,
            idle_probe_ms: None,
            help: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgError {
    InvalidBackend(String),
    InvalidRuntime(String),
    InvalidValue(&'static str, String),
    MissingValue(&'static str),
    UnknownFlag(String),
}

impl fmt::Display for ArgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBackend(value) => write!(f, "invalid backend: {value}"),
            Self::InvalidRuntime(value) => write!(f, "invalid runtime: {value}"),
            Self::InvalidValue(flag, value) => write!(f, "invalid value for {flag}: {value}"),
            Self::MissingValue(flag) => write!(f, "missing value for {flag}"),
            Self::UnknownFlag(flag) => write!(f, "unknown flag: {flag}"),
        }
    }
}

pub fn parse_args<I, S>(args: I) -> Result<RunConfig, ArgError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut config = RunConfig::default();
    let mut args = args.into_iter().map(Into::into);

    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            config.help = true;
        } else if arg == "--smoke-test" {
            config.smoke_test = true;
        } else if arg == "--scripted-client" {
            config.scripted_client = true;
        } else if arg == "--smithay-client-smoke" {
            config.smithay_client_smoke = true;
        } else if let Some(value) = arg.strip_prefix("--scripted-client-preview=") {
            config.scripted_client = true;
            config.scripted_client_preview = Some(value.to_string());
        } else if arg == "--scripted-client-preview" {
            config.scripted_client = true;
            config.scripted_client_preview = Some(
                args.next()
                    .ok_or(ArgError::MissingValue("--scripted-client-preview"))?,
            );
        } else if arg == "--serve" {
            config.serve = true;
        } else if let Some(value) = arg.strip_prefix("--backend=") {
            config.backend = parse_backend(value)?;
        } else if arg == "--backend" {
            let value = args.next().ok_or(ArgError::MissingValue("--backend"))?;
            config.backend = parse_backend(&value)?;
        } else if let Some(value) = arg.strip_prefix("--runtime=") {
            config.runtime = parse_runtime(value)?;
        } else if arg == "--runtime" {
            let value = args.next().ok_or(ArgError::MissingValue("--runtime"))?;
            config.runtime = parse_runtime(&value)?;
        } else if let Some(value) = arg.strip_prefix("--socket=") {
            config.socket = value.to_string();
        } else if arg == "--socket" {
            config.socket = args.next().ok_or(ArgError::MissingValue("--socket"))?;
        } else if let Some(value) = arg.strip_prefix("--idle-probe-ms=") {
            config.idle_probe_ms = Some(parse_u64("--idle-probe-ms", value)?);
        } else if arg == "--idle-probe-ms" {
            let value = args
                .next()
                .ok_or(ArgError::MissingValue("--idle-probe-ms"))?;
            config.idle_probe_ms = Some(parse_u64("--idle-probe-ms", &value)?);
        } else if let Some(value) = arg.strip_prefix("--serve-for-ms=") {
            config.serve = true;
            config.serve_for_ms = Some(parse_u64("--serve-for-ms", value)?);
        } else if arg == "--serve-for-ms" {
            let value = args
                .next()
                .ok_or(ArgError::MissingValue("--serve-for-ms"))?;
            config.serve = true;
            config.serve_for_ms = Some(parse_u64("--serve-for-ms", &value)?);
        } else {
            return Err(ArgError::UnknownFlag(arg));
        }
    }

    Ok(config)
}

fn parse_backend(value: &str) -> Result<BackendKind, ArgError> {
    value
        .parse()
        .map_err(|_| ArgError::InvalidBackend(value.to_string()))
}

fn parse_runtime(value: &str) -> Result<RuntimeKind, ArgError> {
    value
        .parse()
        .map_err(|_| ArgError::InvalidRuntime(value.to_string()))
}

fn parse_u64(flag: &'static str, value: &str) -> Result<u64, ArgError> {
    value
        .parse::<u64>()
        .map_err(|_| ArgError::InvalidValue(flag, value.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessSurface {
    pub id: SurfaceId,
    pub client: ClientId,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub damaged: bool,
    pub options: SurfaceOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessClient {
    pub id: ClientId,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferKind {
    Shm,
    Dmabuf,
}

impl BufferKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shm => "shm",
            Self::Dmabuf => "dmabuf",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceOptions {
    pub buffer_kind: BufferKind,
    pub opaque: bool,
    pub fullscreen: bool,
}

impl Default for SurfaceOptions {
    fn default() -> Self {
        Self {
            buffer_kind: BufferKind::Shm,
            opaque: true,
            fullscreen: false,
        }
    }
}

impl SurfaceOptions {
    pub const fn dmabuf_fullscreen() -> Self {
        Self {
            buffer_kind: BufferKind::Dmabuf,
            opaque: true,
            fullscreen: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameReport {
    pub frame: u64,
    pub client_count: u64,
    pub surface_count: u64,
    pub damaged_surfaces: u64,
    pub total_pixels: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectScanoutReport {
    pub surface: SurfaceId,
    pub eligible: bool,
    pub reason: &'static str,
    pub buffer_kind: BufferKind,
    pub surface_count: u64,
    pub output_pixels: u64,
    pub surface_pixels: u64,
}

pub trait CompositorRuntime {
    type Error: fmt::Display;

    fn runtime_name(&self) -> &'static str;
    fn smithay_protocol_global_count(&self) -> u64 {
        0
    }
    fn inserted_wayland_clients(&self) -> u64 {
        0
    }
    fn wayland_dispatch_count(&self) -> u64 {
        0
    }
    fn calloop_dispatch_count(&self) -> u64 {
        0
    }
    fn connect_client(&mut self, name: &str) -> ClientId;
    fn submit_surface(
        &mut self,
        client: ClientId,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<SurfaceId, Self::Error>;
    fn submit_surface_with_options(
        &mut self,
        client: ClientId,
        title: &str,
        width: u32,
        height: u32,
        options: SurfaceOptions,
    ) -> Result<SurfaceId, Self::Error>;
    fn mark_damaged(&mut self, surface: SurfaceId) -> Result<(), Self::Error>;
    fn close_surface(&mut self, surface: SurfaceId) -> Result<(), Self::Error>;
    fn disconnect_client(&mut self, client: ClientId) -> Result<u64, Self::Error>;
    fn present(&mut self) -> FrameReport;
    fn direct_scanout_candidate(
        &self,
        surface: SurfaceId,
        output_width: u32,
        output_height: u32,
    ) -> Result<DirectScanoutReport, Self::Error>;
    fn client_count(&self) -> u64;
    fn surface_count(&self) -> u64;
}

#[derive(Debug, Clone)]
pub struct HeadlessCompositor {
    clients: Vec<HeadlessClient>,
    surfaces: Vec<HeadlessSurface>,
    next_client_id: u64,
    next_surface_id: u64,
    frame: u64,
    pending_damage_events: u64,
}

impl Default for HeadlessCompositor {
    fn default() -> Self {
        Self {
            clients: Vec::new(),
            surfaces: Vec::new(),
            next_client_id: 1,
            next_surface_id: 1,
            frame: 0,
            pending_damage_events: 0,
        }
    }
}

impl HeadlessCompositor {
    pub fn connect_client(&mut self, name: impl Into<String>) -> ClientId {
        let id = ClientId(self.next_client_id);
        self.next_client_id += 1;
        self.clients.push(HeadlessClient {
            id,
            name: name.into(),
        });
        id
    }

    pub fn submit_surface(
        &mut self,
        client: ClientId,
        title: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<SurfaceId, HeadlessError> {
        self.submit_surface_with_options(client, title, width, height, SurfaceOptions::default())
    }

    pub fn submit_surface_with_options(
        &mut self,
        client: ClientId,
        title: impl Into<String>,
        width: u32,
        height: u32,
        options: SurfaceOptions,
    ) -> Result<SurfaceId, HeadlessError> {
        if !self.clients.iter().any(|known| known.id == client) {
            return Err(HeadlessError::UnknownClient(client));
        }

        let id = SurfaceId(self.next_surface_id);
        self.next_surface_id += 1;
        self.surfaces.push(HeadlessSurface {
            id,
            client,
            title: title.into(),
            width,
            height,
            damaged: true,
            options,
        });
        Ok(id)
    }

    pub fn mark_damaged(&mut self, surface: SurfaceId) -> Result<(), HeadlessError> {
        match self.surfaces.iter_mut().find(|known| known.id == surface) {
            Some(surface) => {
                surface.damaged = true;
                Ok(())
            }
            None => Err(HeadlessError::UnknownSurface(surface)),
        }
    }

    pub fn close_surface(&mut self, surface: SurfaceId) -> Result<(), HeadlessError> {
        let Some(index) = self.surfaces.iter().position(|known| known.id == surface) else {
            return Err(HeadlessError::UnknownSurface(surface));
        };

        self.surfaces.remove(index);
        self.pending_damage_events += 1;
        Ok(())
    }

    pub fn disconnect_client(&mut self, client: ClientId) -> Result<u64, HeadlessError> {
        if !self.clients.iter().any(|known| known.id == client) {
            return Err(HeadlessError::UnknownClient(client));
        }

        self.clients.retain(|known| known.id != client);
        let surface_count = self.surfaces.len();
        self.surfaces.retain(|surface| surface.client != client);
        let removed_surfaces = (surface_count - self.surfaces.len()) as u64;
        self.pending_damage_events += removed_surfaces;
        Ok(removed_surfaces)
    }

    pub fn present(&mut self) -> FrameReport {
        self.frame += 1;

        let damaged_surfaces = self.pending_damage_events
            + self
                .surfaces
                .iter()
                .filter(|surface| surface.damaged)
                .count() as u64;
        let total_pixels = self
            .surfaces
            .iter()
            .map(|surface| surface.width as u64 * surface.height as u64)
            .sum();

        for surface in &mut self.surfaces {
            surface.damaged = false;
        }
        self.pending_damage_events = 0;

        FrameReport {
            frame: self.frame,
            client_count: self.clients.len() as u64,
            surface_count: self.surfaces.len() as u64,
            damaged_surfaces,
            total_pixels,
        }
    }

    pub fn direct_scanout_candidate(
        &self,
        surface: SurfaceId,
        output_width: u32,
        output_height: u32,
    ) -> Result<DirectScanoutReport, HeadlessError> {
        let candidate = self
            .surfaces
            .iter()
            .find(|known| known.id == surface)
            .ok_or(HeadlessError::UnknownSurface(surface))?;
        let output_pixels = output_width as u64 * output_height as u64;
        let surface_pixels = candidate.width as u64 * candidate.height as u64;
        let surface_count = self.surfaces.len() as u64;

        let (eligible, reason) = if !candidate.options.fullscreen {
            (false, "not-fullscreen")
        } else if candidate.options.buffer_kind != BufferKind::Dmabuf {
            (false, "not-dmabuf")
        } else if !candidate.options.opaque {
            (false, "not-opaque")
        } else if candidate.width != output_width || candidate.height != output_height {
            (false, "does-not-cover-output")
        } else if self.surfaces.iter().any(|known| known.id != surface) {
            (false, "occluded-by-other-surface")
        } else {
            (true, "eligible")
        };

        Ok(DirectScanoutReport {
            surface,
            eligible,
            reason,
            buffer_kind: candidate.options.buffer_kind,
            surface_count,
            output_pixels,
            surface_pixels,
        })
    }

    pub fn clients(&self) -> &[HeadlessClient] {
        &self.clients
    }

    pub fn surfaces(&self) -> &[HeadlessSurface] {
        &self.surfaces
    }
}

impl CompositorRuntime for HeadlessCompositor {
    type Error = HeadlessError;

    fn runtime_name(&self) -> &'static str {
        "headless-compositor"
    }

    fn connect_client(&mut self, name: &str) -> ClientId {
        HeadlessCompositor::connect_client(self, name)
    }

    fn submit_surface(
        &mut self,
        client: ClientId,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<SurfaceId, Self::Error> {
        HeadlessCompositor::submit_surface(self, client, title, width, height)
    }

    fn submit_surface_with_options(
        &mut self,
        client: ClientId,
        title: &str,
        width: u32,
        height: u32,
        options: SurfaceOptions,
    ) -> Result<SurfaceId, Self::Error> {
        HeadlessCompositor::submit_surface_with_options(self, client, title, width, height, options)
    }

    fn mark_damaged(&mut self, surface: SurfaceId) -> Result<(), Self::Error> {
        HeadlessCompositor::mark_damaged(self, surface)
    }

    fn close_surface(&mut self, surface: SurfaceId) -> Result<(), Self::Error> {
        HeadlessCompositor::close_surface(self, surface)
    }

    fn disconnect_client(&mut self, client: ClientId) -> Result<u64, Self::Error> {
        HeadlessCompositor::disconnect_client(self, client)
    }

    fn present(&mut self) -> FrameReport {
        HeadlessCompositor::present(self)
    }

    fn direct_scanout_candidate(
        &self,
        surface: SurfaceId,
        output_width: u32,
        output_height: u32,
    ) -> Result<DirectScanoutReport, Self::Error> {
        HeadlessCompositor::direct_scanout_candidate(self, surface, output_width, output_height)
    }

    fn client_count(&self) -> u64 {
        self.clients.len() as u64
    }

    fn surface_count(&self) -> u64 {
        self.surfaces.len() as u64
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
pub struct SmithayCompositorRuntime {
    inner: HeadlessCompositor,
    display: smithay::reexports::wayland_server::Display<SmithayCompositorState>,
    event_loop: smithay::reexports::calloop::EventLoop<'static, SmithayCompositorState>,
    state: SmithayCompositorState,
    listening_socket: smithay::reexports::wayland_server::ListeningSocket,
    socket_name: String,
    inserted_wayland_clients: u64,
    wayland_dispatch_count: u64,
    calloop_dispatch_count: u64,
    last_error: Option<String>,
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
#[derive(Debug)]
struct SmithayCompositorState {
    compositor_state: smithay::wayland::compositor::CompositorState,
    shm_state: smithay::wayland::shm::ShmState,
    xdg_shell_state: smithay::wayland::shell::xdg::XdgShellState,
    seat_state: smithay::input::SeatState<SmithayCompositorState>,
    protocol_global_count: u64,
    surface_commit_count: u64,
    xdg_toplevel_count: u64,
    xdg_popup_count: u64,
    title_changed_count: u64,
    app_id_changed_count: u64,
    observed_title: Option<String>,
    observed_app_id: Option<String>,
    title_matched: bool,
    app_id_matched: bool,
    shm_buffer_commit_count: u64,
    shm_buffer_width: u64,
    shm_buffer_height: u64,
    shm_buffer_pixels: u64,
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
#[derive(Debug, Default)]
struct SmithayClientData {
    compositor_state: smithay::wayland::compositor::CompositorClientState,
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl smithay::reexports::wayland_server::backend::ClientData for SmithayClientData {}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
const SMITHAY_SMOKE_TITLE: &str = "Backlit Smithay smoke";

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
const SMITHAY_SMOKE_APP_ID: &str = "org.backlit.SmithaySmoke";

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
const SMITHAY_SMOKE_WIDTH: i32 = 320;

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
const SMITHAY_SMOKE_HEIGHT: i32 = 240;

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmithayWaylandClientSmokeReport {
    pub protocol_globals: u64,
    pub registry_global_count: u64,
    pub registry_announced: bool,
    pub compositor_bound: bool,
    pub shm_bound: bool,
    pub shm_buffer_created: bool,
    pub shm_buffer_attached: bool,
    pub xdg_wm_base_bound: bool,
    pub surface_created: bool,
    pub xdg_toplevel_created: bool,
    pub configure_received: bool,
    pub configure_acked: bool,
    pub surface_committed: bool,
    pub inserted_wayland_clients: u64,
    pub wayland_dispatch_count: u64,
    pub calloop_dispatch_count: u64,
    pub surface_commit_count: u64,
    pub xdg_toplevel_count: u64,
    pub xdg_popup_count: u64,
    pub title_changed_count: u64,
    pub app_id_changed_count: u64,
    pub observed_title: String,
    pub observed_app_id: String,
    pub title_matched: bool,
    pub app_id_matched: bool,
    pub shm_buffer_commit_count: u64,
    pub shm_buffer_width: u64,
    pub shm_buffer_height: u64,
    pub shm_buffer_pixels: u64,
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl SmithayWaylandClientSmokeReport {
    pub fn passed(&self) -> bool {
        self.protocol_globals >= 4
            && self.registry_global_count >= 4
            && self.registry_announced
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
            && self.surface_commit_count >= 1
            && self.xdg_toplevel_count >= 1
            && self.title_changed_count >= 1
            && self.app_id_changed_count >= 1
            && self.title_matched
            && self.app_id_matched
            && self.observed_title == SMITHAY_SMOKE_TITLE
            && self.observed_app_id == SMITHAY_SMOKE_APP_ID
            && self.shm_buffer_commit_count >= 1
            && self.shm_buffer_width == SMITHAY_SMOKE_WIDTH as u64
            && self.shm_buffer_height == SMITHAY_SMOKE_HEIGHT as u64
            && self.shm_buffer_pixels == (SMITHAY_SMOKE_WIDTH * SMITHAY_SMOKE_HEIGHT) as u64
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct WaylandGlobal {
    name: u32,
    interface: String,
    version: u32,
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
#[derive(Debug, Default)]
struct WaylandClientEventState {
    globals: Vec<WaylandGlobal>,
    wm_base_ping_serials: Vec<u32>,
    failure: Option<String>,
    compositor_bound: bool,
    shm_bound: bool,
    shm_buffer_created: bool,
    shm_buffer_attached: bool,
    xdg_wm_base_bound: bool,
    surface_created: bool,
    xdg_surface_created: bool,
    xdg_toplevel_created: bool,
    xdg_toplevel_configures: u64,
    xdg_surface_configure_serial: Option<u32>,
    configure_acked: bool,
    surface_committed: bool,
    compositor: Option<wayland_client::protocol::wl_compositor::WlCompositor>,
    shm: Option<wayland_client::protocol::wl_shm::WlShm>,
    shm_pool: Option<wayland_client::protocol::wl_shm_pool::WlShmPool>,
    shm_buffer: Option<wayland_client::protocol::wl_buffer::WlBuffer>,
    shm_file: Option<std::fs::File>,
    base_surface: Option<wayland_client::protocol::wl_surface::WlSurface>,
    wm_base: Option<wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase>,
    xdg_surface: Option<wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface>,
    xdg_toplevel: Option<wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel>,
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl WaylandClientEventState {
    fn global(&self, interface: &str) -> Option<&WaylandGlobal> {
        self.globals
            .iter()
            .find(|global| global.interface == interface)
    }

    fn registry_announced(&self) -> bool {
        self.global("wl_compositor").is_some()
            && self.global("wl_subcompositor").is_some()
            && self.global("wl_shm").is_some()
            && self.global("xdg_wm_base").is_some()
    }

    fn init_xdg_toplevel(&mut self, qh: &wayland_client::QueueHandle<Self>) {
        if self.base_surface.is_none() {
            if let Some(compositor) = self.compositor.as_ref() {
                self.base_surface = Some(compositor.create_surface(qh, ()));
                self.surface_created = true;
            }
        }

        if self.xdg_surface.is_none() {
            let (Some(wm_base), Some(base_surface)) =
                (self.wm_base.as_ref(), self.base_surface.as_ref())
            else {
                return;
            };

            let xdg_surface = wm_base.get_xdg_surface(base_surface, qh, ());
            let xdg_toplevel = xdg_surface.get_toplevel(qh, ());
            xdg_surface.set_window_geometry(0, 0, SMITHAY_SMOKE_WIDTH, SMITHAY_SMOKE_HEIGHT);
            xdg_toplevel.set_title(String::from(SMITHAY_SMOKE_TITLE));
            xdg_toplevel.set_app_id(String::from(SMITHAY_SMOKE_APP_ID));
            base_surface.commit();
            self.xdg_surface = Some(xdg_surface);
            self.xdg_toplevel = Some(xdg_toplevel);
            self.xdg_surface_created = true;
            self.xdg_toplevel_created = true;
        }
    }

    fn init_shm_buffer(&mut self, qh: &wayland_client::QueueHandle<Self>) {
        if self.shm_buffer.is_some() || self.failure.is_some() {
            return;
        }

        let Some(shm) = self.shm.as_ref() else {
            return;
        };

        match create_smoke_shm_file() {
            Ok(mut file) => {
                use std::os::fd::AsFd;

                let stride = SMITHAY_SMOKE_WIDTH * 4;
                let byte_len = stride * SMITHAY_SMOKE_HEIGHT;
                let pool = shm.create_pool(file.as_fd(), byte_len, qh, ());
                let buffer = pool.create_buffer(
                    0,
                    SMITHAY_SMOKE_WIDTH,
                    SMITHAY_SMOKE_HEIGHT,
                    stride,
                    wayland_client::protocol::wl_shm::Format::Argb8888,
                    qh,
                    (),
                );
                if let Err(error) = std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(0)) {
                    self.failure = Some(format!("shm-file-seek:{error}"));
                    return;
                }
                self.shm_pool = Some(pool);
                self.shm_buffer = Some(buffer);
                self.shm_file = Some(file);
                self.shm_buffer_created = true;
            }
            Err(error) => {
                self.failure = Some(error);
            }
        }
    }

    fn attach_shm_buffer_if_configured(&mut self) {
        if self.shm_buffer_attached || self.failure.is_some() || !self.configure_acked {
            return;
        }

        let (Some(base_surface), Some(buffer)) =
            (self.base_surface.as_ref(), self.shm_buffer.as_ref())
        else {
            return;
        };

        base_surface.attach(Some(buffer), 0, 0);
        base_surface.damage(0, 0, SMITHAY_SMOKE_WIDTH, SMITHAY_SMOKE_HEIGHT);
        base_surface.commit();
        self.shm_buffer_attached = true;
        self.surface_committed = true;
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn create_smoke_shm_file() -> Result<std::fs::File, String> {
    use std::io::{Seek, SeekFrom, Write};

    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = dir.join(format!(
        "backlit-smithay-smoke-{}-{}.shm",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("shm-file-open:{error}"))?;
    let _ = fs::remove_file(&path);

    let stride = SMITHAY_SMOKE_WIDTH as usize * 4;
    let byte_len = stride * SMITHAY_SMOKE_HEIGHT as usize;
    file.set_len(byte_len as u64)
        .map_err(|error| format!("shm-file-len:{error}"))?;

    for y in 0..SMITHAY_SMOKE_HEIGHT {
        for x in 0..SMITHAY_SMOKE_WIDTH {
            let alpha = 0xffu8;
            let red = ((x * 255) / SMITHAY_SMOKE_WIDTH) as u8;
            let green = ((y * 255) / SMITHAY_SMOKE_HEIGHT) as u8;
            let blue = 0x66u8;
            file.write_all(&[blue, green, red, alpha])
                .map_err(|error| format!("shm-file-write:{error}"))?;
        }
    }
    file.flush()
        .map_err(|error| format!("shm-file-flush:{error}"))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("shm-file-rewind:{error}"))?;
    Ok(file)
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl wayland_client::Dispatch<wayland_client::protocol::wl_registry::WlRegistry, ()>
    for WaylandClientEventState
{
    fn event(
        state: &mut Self,
        registry: &wayland_client::protocol::wl_registry::WlRegistry,
        event: wayland_client::protocol::wl_registry::Event,
        _: &(),
        _: &wayland_client::Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        use wayland_client::protocol::{wl_compositor, wl_registry, wl_shm};
        use wayland_protocols::xdg::shell::client::xdg_wm_base;

        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };

        if state
            .globals
            .iter()
            .all(|global| global.name != name || global.interface != interface)
        {
            state.globals.push(WaylandGlobal {
                name,
                interface: interface.clone(),
                version,
            });
        }

        match interface.as_str() {
            "wl_compositor" if !state.compositor_bound => {
                let compositor =
                    registry.bind::<wl_compositor::WlCompositor, _, _>(name, 1, qh, ());
                state.compositor = Some(compositor);
                state.compositor_bound = true;
                state.init_xdg_toplevel(qh);
            }
            "wl_shm" if !state.shm_bound => {
                let shm = registry.bind::<wl_shm::WlShm, _, _>(name, 1, qh, ());
                state.shm = Some(shm);
                state.shm_bound = true;
                state.init_shm_buffer(qh);
                state.attach_shm_buffer_if_configured();
            }
            "xdg_wm_base" if !state.xdg_wm_base_bound => {
                let wm_base = registry.bind::<xdg_wm_base::XdgWmBase, _, _>(name, 1, qh, ());
                state.wm_base = Some(wm_base);
                state.xdg_wm_base_bound = true;
                state.init_xdg_toplevel(qh);
            }
            _ => {}
        }
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl wayland_client::Dispatch<wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase, ()>
    for WaylandClientEventState
{
    fn event(
        state: &mut Self,
        wm_base: &wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase,
        event: wayland_protocols::xdg::shell::client::xdg_wm_base::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        if let wayland_protocols::xdg::shell::client::xdg_wm_base::Event::Ping { serial } = event {
            state.wm_base_ping_serials.push(serial);
            wm_base.pong(serial);
        }
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl wayland_client::Dispatch<wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface, ()>
    for WaylandClientEventState
{
    fn event(
        state: &mut Self,
        xdg_surface: &wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface,
        event: wayland_protocols::xdg::shell::client::xdg_surface::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        if let wayland_protocols::xdg::shell::client::xdg_surface::Event::Configure { serial } =
            event
        {
            state.xdg_surface_configure_serial = Some(serial);
            xdg_surface.ack_configure(serial);
            state.configure_acked = true;
            state.attach_shm_buffer_if_configured();
        }
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl wayland_client::Dispatch<wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel, ()>
    for WaylandClientEventState
{
    fn event(
        state: &mut Self,
        _: &wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel,
        event: wayland_protocols::xdg::shell::client::xdg_toplevel::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        if let wayland_protocols::xdg::shell::client::xdg_toplevel::Event::Configure { .. } = event
        {
            state.xdg_toplevel_configures += 1;
        }
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
wayland_client::delegate_noop!(WaylandClientEventState: ignore wayland_client::protocol::wl_compositor::WlCompositor);

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
wayland_client::delegate_noop!(WaylandClientEventState: ignore wayland_client::protocol::wl_surface::WlSurface);

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
wayland_client::delegate_noop!(WaylandClientEventState: ignore wayland_client::protocol::wl_shm::WlShm);

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
wayland_client::delegate_noop!(WaylandClientEventState: ignore wayland_client::protocol::wl_shm_pool::WlShmPool);

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
wayland_client::delegate_noop!(WaylandClientEventState: ignore wayland_client::protocol::wl_buffer::WlBuffer);

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl SmithayCompositorState {
    fn new(display: &smithay::reexports::wayland_server::DisplayHandle) -> Self {
        let compositor_state = smithay::wayland::compositor::CompositorState::new::<Self>(display);
        let shm_state = smithay::wayland::shm::ShmState::new::<Self>(
            display,
            std::iter::empty::<smithay::reexports::wayland_server::protocol::wl_shm::Format>(),
        );
        let xdg_shell_state = smithay::wayland::shell::xdg::XdgShellState::new::<Self>(display);

        Self {
            compositor_state,
            shm_state,
            xdg_shell_state,
            seat_state: Default::default(),
            protocol_global_count: 4,
            surface_commit_count: 0,
            xdg_toplevel_count: 0,
            xdg_popup_count: 0,
            title_changed_count: 0,
            app_id_changed_count: 0,
            observed_title: None,
            observed_app_id: None,
            title_matched: false,
            app_id_matched: false,
            shm_buffer_commit_count: 0,
            shm_buffer_width: 0,
            shm_buffer_height: 0,
            shm_buffer_pixels: 0,
        }
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl smithay::wayland::compositor::CompositorHandler for SmithayCompositorState {
    fn compositor_state(&mut self) -> &mut smithay::wayland::compositor::CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(
        &self,
        client: &'a smithay::reexports::wayland_server::Client,
    ) -> &'a smithay::wayland::compositor::CompositorClientState {
        client
            .get_data::<SmithayClientData>()
            .map(|data| &data.compositor_state)
            .expect("Smithay compositor clients must carry Backlit client data")
    }

    fn commit(
        &mut self,
        surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) {
        self.surface_commit_count += 1;
        if let Some((width, height)) = smithay_committed_buffer_dimensions(surface) {
            self.shm_buffer_commit_count += 1;
            self.shm_buffer_width = width;
            self.shm_buffer_height = height;
            self.shm_buffer_pixels = width * height;
        }
        smithay::backend::renderer::utils::on_commit_buffer_handler::<Self>(surface);
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn smithay_committed_buffer_dimensions(
    surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
) -> Option<(u64, u64)> {
    smithay::wayland::compositor::with_states(surface, |states| {
        let mut guard = states
            .cached_state
            .get::<smithay::wayland::compositor::SurfaceAttributes>();
        let attributes = guard.current();
        let Some(smithay::wayland::compositor::BufferAssignment::NewBuffer(buffer)) =
            attributes.buffer.as_ref()
        else {
            return None;
        };

        smithay::backend::renderer::buffer_dimensions(buffer)
            .map(|size| (size.w.max(0) as u64, size.h.max(0) as u64))
    })
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl smithay::wayland::buffer::BufferHandler for SmithayCompositorState {
    fn buffer_destroyed(
        &mut self,
        _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    ) {
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl smithay::wayland::shm::ShmHandler for SmithayCompositorState {
    fn shm_state(&self) -> &smithay::wayland::shm::ShmState {
        &self.shm_state
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl smithay::input::SeatHandler for SmithayCompositorState {
    type KeyboardFocus = smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
    type PointerFocus = smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
    type TouchFocus = smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

    fn seat_state(&mut self) -> &mut smithay::input::SeatState<Self> {
        &mut self.seat_state
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl smithay::wayland::shell::xdg::XdgShellHandler for SmithayCompositorState {
    fn xdg_shell_state(&mut self) -> &mut smithay::wayland::shell::xdg::XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {
        self.xdg_toplevel_count += 1;
        surface.send_configure();
    }

    fn new_popup(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        _positioner: smithay::wayland::shell::xdg::PositionerState,
    ) {
        self.xdg_popup_count += 1;
        let _ = surface.send_configure();
    }

    fn grab(
        &mut self,
        _surface: smithay::wayland::shell::xdg::PopupSurface,
        _seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        _serial: smithay::utils::Serial,
    ) {
    }

    fn reposition_request(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        _positioner: smithay::wayland::shell::xdg::PositionerState,
        token: u32,
    ) {
        surface.send_repositioned(token);
    }

    fn title_changed(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {
        self.title_changed_count += 1;
        let (title, _app_id) = smithay_toplevel_metadata(&surface);
        if let Some(title) = title.as_ref() {
            self.observed_title = Some(title.clone());
        }
        if title.as_deref() == Some(SMITHAY_SMOKE_TITLE) {
            self.title_matched = true;
        }
    }

    fn app_id_changed(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {
        self.app_id_changed_count += 1;
        let (_title, app_id) = smithay_toplevel_metadata(&surface);
        if let Some(app_id) = app_id.as_ref() {
            self.observed_app_id = Some(app_id.clone());
        }
        if app_id.as_deref() == Some(SMITHAY_SMOKE_APP_ID) {
            self.app_id_matched = true;
        }
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn smithay_toplevel_metadata(
    surface: &smithay::wayland::shell::xdg::ToplevelSurface,
) -> (Option<String>, Option<String>) {
    smithay::wayland::compositor::with_states(surface.wl_surface(), |states| {
        let Some(data) = states
            .data_map
            .get::<smithay::wayland::shell::xdg::XdgToplevelSurfaceData>()
        else {
            return (None, None);
        };
        let attributes = data.lock().unwrap();
        (attributes.title.clone(), attributes.app_id.clone())
    })
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
smithay::delegate_compositor!(SmithayCompositorState);

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
smithay::delegate_shm!(SmithayCompositorState);

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
smithay::delegate_xdg_shell!(SmithayCompositorState);

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl SmithayCompositorRuntime {
    pub fn try_new() -> Result<Self, SmithayRuntimeError> {
        use smithay::reexports::calloop::EventLoop;
        use smithay::reexports::wayland_server::{Display, ListeningSocket};

        let display = Display::<SmithayCompositorState>::new()
            .map_err(|error| SmithayRuntimeError(format!("display-new:{error}")))?;
        let display_handle = display.handle();
        let state = SmithayCompositorState::new(&display_handle);
        let event_loop = EventLoop::<SmithayCompositorState>::try_new()
            .map_err(|error| SmithayRuntimeError(format!("event-loop-new:{error}")))?;
        let listening_socket = ListeningSocket::bind_auto("backlit-smithay-runtime", 0..64)
            .map_err(|error| SmithayRuntimeError(format!("socket-bind:{error}")))?;
        let socket_name = listening_socket
            .socket_name()
            .and_then(|name| name.to_str())
            .map(String::from)
            .ok_or_else(|| SmithayRuntimeError(String::from("socket-name:unavailable")))?;

        Ok(Self {
            inner: HeadlessCompositor::default(),
            display,
            event_loop,
            state,
            listening_socket,
            socket_name,
            inserted_wayland_clients: 0,
            wayland_dispatch_count: 0,
            calloop_dispatch_count: 0,
            last_error: None,
        })
    }

    pub fn socket_name(&self) -> &str {
        self.socket_name.as_str()
    }

    pub fn inserted_wayland_clients(&self) -> u64 {
        self.inserted_wayland_clients
    }

    pub fn smithay_protocol_global_count(&self) -> u64 {
        self.state.protocol_global_count
    }

    pub fn wayland_dispatch_count(&self) -> u64 {
        self.wayland_dispatch_count
    }

    pub fn calloop_dispatch_count(&self) -> u64 {
        self.calloop_dispatch_count
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn run_wayland_client_smoke(
        &mut self,
    ) -> Result<SmithayWaylandClientSmokeReport, SmithayRuntimeError> {
        let client_stream = self.connect_and_insert_wayland_client("real-wayland-smoke")?;
        let client_connection = wayland_client::Connection::from_socket(client_stream)
            .map_err(|error| SmithayRuntimeError(format!("client-connect:{error}")))?;
        let mut event_queue = client_connection.new_event_queue::<WaylandClientEventState>();
        let qh = event_queue.handle();
        let mut client_state = WaylandClientEventState::default();

        client_connection.display().get_registry(&qh, ());
        for _ in 0..24 {
            pump_wayland_client(
                self,
                &client_connection,
                &mut event_queue,
                &mut client_state,
            )?;
            if let Some(error) = client_state.failure.as_ref() {
                return Err(SmithayRuntimeError(error.clone()));
            }
            if client_state.registry_announced()
                && client_state.configure_acked
                && client_state.surface_committed
                && self.state.surface_commit_count >= 2
                && self.state.title_matched
                && self.state.app_id_matched
                && self.state.shm_buffer_commit_count >= 1
            {
                break;
            }
        }

        let report = SmithayWaylandClientSmokeReport {
            protocol_globals: self.state.protocol_global_count,
            registry_global_count: client_state.globals.len() as u64,
            registry_announced: client_state.registry_announced(),
            compositor_bound: client_state.compositor_bound,
            shm_bound: client_state.shm_bound,
            shm_buffer_created: client_state.shm_buffer_created,
            shm_buffer_attached: client_state.shm_buffer_attached,
            xdg_wm_base_bound: client_state.xdg_wm_base_bound,
            surface_created: client_state.surface_created,
            xdg_toplevel_created: client_state.xdg_toplevel_created
                && self.state.xdg_toplevel_count >= 1,
            configure_received: client_state.xdg_surface_configure_serial.is_some()
                && client_state.xdg_toplevel_configures >= 1,
            configure_acked: client_state.configure_acked,
            surface_committed: client_state.surface_committed
                && self.state.surface_commit_count >= 2,
            inserted_wayland_clients: self.inserted_wayland_clients,
            wayland_dispatch_count: self.wayland_dispatch_count,
            calloop_dispatch_count: self.calloop_dispatch_count,
            surface_commit_count: self.state.surface_commit_count,
            xdg_toplevel_count: self.state.xdg_toplevel_count,
            xdg_popup_count: self.state.xdg_popup_count,
            title_changed_count: self.state.title_changed_count,
            app_id_changed_count: self.state.app_id_changed_count,
            observed_title: self.state.observed_title.clone().unwrap_or_default(),
            observed_app_id: self.state.observed_app_id.clone().unwrap_or_default(),
            title_matched: self.state.title_matched,
            app_id_matched: self.state.app_id_matched,
            shm_buffer_commit_count: self.state.shm_buffer_commit_count,
            shm_buffer_width: self.state.shm_buffer_width,
            shm_buffer_height: self.state.shm_buffer_height,
            shm_buffer_pixels: self.state.shm_buffer_pixels,
        };

        Ok(report)
    }

    fn connect_and_insert_wayland_client(
        &mut self,
        name: &str,
    ) -> Result<std::os::unix::net::UnixStream, SmithayRuntimeError> {
        use std::{env, os::unix::net::UnixStream, path::PathBuf, sync::Arc};

        use smithay::reexports::wayland_server::backend::ClientData;

        let runtime_dir = env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
            SmithayRuntimeError(String::from("socket-connect:missing-runtime-dir"))
        })?;
        let socket_path = PathBuf::from(runtime_dir).join(&self.socket_name);
        let client_stream = UnixStream::connect(&socket_path)
            .map_err(|error| SmithayRuntimeError(format!("socket-connect:{name}:{error}")))?;
        let accepted_stream = accept_bootstrap_client(&self.listening_socket)
            .map_err(|error| SmithayRuntimeError(format!("{name}:{error}")))?;
        let client_data: Arc<dyn ClientData> = Arc::new(SmithayClientData::default());
        let mut display_handle = self.display.handle();
        display_handle
            .insert_client(accepted_stream, client_data)
            .map_err(|error| SmithayRuntimeError(format!("client-insert:{name}:{error}")))?;
        self.inserted_wayland_clients += 1;
        Ok(client_stream)
    }

    fn insert_wayland_client(&mut self, name: &str) -> Result<(), SmithayRuntimeError> {
        let _client_stream = self.connect_and_insert_wayland_client(name)?;
        Ok(())
    }

    fn dispatch_wayland(&mut self) {
        use std::time::Duration;

        match self.display.dispatch_clients(&mut self.state) {
            Ok(_count) => {
                self.wayland_dispatch_count += 1;
                if let Err(error) = self.display.flush_clients() {
                    self.last_error = Some(format!("display-flush:{error}"));
                }
            }
            Err(error) => {
                self.last_error = Some(format!("display-dispatch:{error}"));
            }
        }

        match self
            .event_loop
            .dispatch(Some(Duration::from_millis(0)), &mut self.state)
        {
            Ok(()) => {
                self.calloop_dispatch_count += 1;
            }
            Err(error) => {
                self.last_error = Some(format!("event-loop-dispatch:{error}"));
            }
        }
    }
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn pump_wayland_client(
    runtime: &mut SmithayCompositorRuntime,
    client_connection: &wayland_client::Connection,
    event_queue: &mut wayland_client::EventQueue<WaylandClientEventState>,
    client_state: &mut WaylandClientEventState,
) -> Result<(), SmithayRuntimeError> {
    client_connection
        .flush()
        .map_err(|error| SmithayRuntimeError(format!("client-flush:{error}")))?;
    runtime.dispatch_wayland();
    drain_wayland_client_queue(event_queue, client_state)?;
    client_connection
        .flush()
        .map_err(|error| SmithayRuntimeError(format!("client-flush:{error}")))?;
    runtime.dispatch_wayland();
    drain_wayland_client_queue(event_queue, client_state)?;
    Ok(())
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
fn drain_wayland_client_queue(
    event_queue: &mut wayland_client::EventQueue<WaylandClientEventState>,
    client_state: &mut WaylandClientEventState,
) -> Result<(), SmithayRuntimeError> {
    use std::io::ErrorKind;

    for _ in 0..16 {
        let dispatched = event_queue
            .dispatch_pending(client_state)
            .map_err(|error| SmithayRuntimeError(format!("client-dispatch:{error}")))?;

        let Some(read_guard) = event_queue.prepare_read() else {
            continue;
        };

        match read_guard.read() {
            Ok(read) => {
                let pending_after_read = event_queue
                    .dispatch_pending(client_state)
                    .map_err(|error| SmithayRuntimeError(format!("client-dispatch:{error}")))?;
                if dispatched == 0 && read == 0 && pending_after_read == 0 {
                    break;
                }
            }
            Err(wayland_client::backend::WaylandError::Io(error))
                if error.kind() == ErrorKind::WouldBlock =>
            {
                break;
            }
            Err(error) => return Err(SmithayRuntimeError(format!("client-read:{error}"))),
        }
    }

    Ok(())
}

#[cfg(all(feature = "smithay-backend", target_os = "linux"))]
impl CompositorRuntime for SmithayCompositorRuntime {
    type Error = SmithayRuntimeError;

    fn runtime_name(&self) -> &'static str {
        "smithay-compositor-runtime"
    }

    fn smithay_protocol_global_count(&self) -> u64 {
        SmithayCompositorRuntime::smithay_protocol_global_count(self)
    }

    fn inserted_wayland_clients(&self) -> u64 {
        SmithayCompositorRuntime::inserted_wayland_clients(self)
    }

    fn wayland_dispatch_count(&self) -> u64 {
        SmithayCompositorRuntime::wayland_dispatch_count(self)
    }

    fn calloop_dispatch_count(&self) -> u64 {
        SmithayCompositorRuntime::calloop_dispatch_count(self)
    }

    fn connect_client(&mut self, name: &str) -> ClientId {
        match self.insert_wayland_client(name) {
            Ok(()) => self.inner.connect_client(name),
            Err(error) => {
                self.last_error = Some(error.to_string());
                ClientId(0)
            }
        }
    }

    fn submit_surface(
        &mut self,
        client: ClientId,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<SurfaceId, Self::Error> {
        self.submit_surface_with_options(client, title, width, height, SurfaceOptions::default())
    }

    fn submit_surface_with_options(
        &mut self,
        client: ClientId,
        title: &str,
        width: u32,
        height: u32,
        options: SurfaceOptions,
    ) -> Result<SurfaceId, Self::Error> {
        self.inner
            .submit_surface_with_options(client, title, width, height, options)
            .map_err(|error| SmithayRuntimeError(error.to_string()))
    }

    fn mark_damaged(&mut self, surface: SurfaceId) -> Result<(), Self::Error> {
        self.inner
            .mark_damaged(surface)
            .map_err(|error| SmithayRuntimeError(error.to_string()))
    }

    fn close_surface(&mut self, surface: SurfaceId) -> Result<(), Self::Error> {
        self.inner
            .close_surface(surface)
            .map_err(|error| SmithayRuntimeError(error.to_string()))
    }

    fn disconnect_client(&mut self, client: ClientId) -> Result<u64, Self::Error> {
        self.inner
            .disconnect_client(client)
            .map_err(|error| SmithayRuntimeError(error.to_string()))
    }

    fn present(&mut self) -> FrameReport {
        self.dispatch_wayland();
        self.inner.present()
    }

    fn direct_scanout_candidate(
        &self,
        surface: SurfaceId,
        output_width: u32,
        output_height: u32,
    ) -> Result<DirectScanoutReport, Self::Error> {
        self.inner
            .direct_scanout_candidate(surface, output_width, output_height)
            .map_err(|error| SmithayRuntimeError(error.to_string()))
    }

    fn client_count(&self) -> u64 {
        self.inner.client_count()
    }

    fn surface_count(&self) -> u64 {
        self.inner.surface_count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithayRuntimeError(pub String);

impl fmt::Display for SmithayRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadlessError {
    UnknownClient(ClientId),
    UnknownSurface(SurfaceId),
}

impl fmt::Display for HeadlessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownClient(id) => write!(f, "unknown headless client {}", id.0),
            Self::UnknownSurface(id) => write!(f, "unknown headless surface {}", id.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        backend_launch_plan, parse_args, BackendKind, BackendPreflightEnvironment, BufferKind,
        ClientId, CompositorRuntime, HeadlessCompositor, RunConfig, RuntimeKind, SurfaceOptions,
    };

    #[test]
    fn defaults_to_headless() {
        assert_eq!(
            parse_args(std::iter::empty::<String>()).unwrap(),
            RunConfig::default()
        );
    }

    #[test]
    fn parses_backend_socket_and_smoke_test() {
        let config = parse_args([
            "--backend=wayland",
            "--runtime",
            "smithay",
            "--socket",
            "backlit-test",
            "--smoke-test",
            "--scripted-client",
            "--smithay-client-smoke",
            "--scripted-client-preview",
            "target/compositor-runtime/preview.ppm",
            "--idle-probe-ms",
            "250",
            "--serve",
            "--serve-for-ms=25",
        ])
        .unwrap();

        assert_eq!(config.backend, BackendKind::Wayland);
        assert_eq!(config.runtime, RuntimeKind::Smithay);
        assert_eq!(config.socket, "backlit-test");
        assert!(config.smoke_test);
        assert!(config.scripted_client);
        assert!(config.smithay_client_smoke);
        assert_eq!(
            config.scripted_client_preview.as_deref(),
            Some("target/compositor-runtime/preview.ppm")
        );
        assert_eq!(config.idle_probe_ms, Some(250));
        assert!(config.serve);
        assert_eq!(config.serve_for_ms, Some(25));
    }

    #[test]
    fn accepts_nested_alias_for_wayland_backend() {
        let config = parse_args(["--backend", "nested"]).unwrap();

        assert_eq!(config.backend, BackendKind::Wayland);
    }

    #[test]
    fn headless_backend_accepts_clients_and_surfaces() {
        let mut compositor = HeadlessCompositor::default();
        let client = compositor.connect_client("demo-client");
        let surface = compositor
            .submit_surface(client, "demo-window", 640, 480)
            .unwrap();

        assert_eq!(compositor.clients().len(), 1);
        assert_eq!(compositor.surfaces()[0].id, surface);

        let first_frame = compositor.present();
        assert_eq!(first_frame.client_count, 1);
        assert_eq!(first_frame.surface_count, 1);
        assert_eq!(first_frame.damaged_surfaces, 1);
        assert_eq!(first_frame.total_pixels, 640 * 480);

        let second_frame = compositor.present();
        assert_eq!(second_frame.damaged_surfaces, 0);
    }

    #[test]
    fn headless_backend_satisfies_compositor_runtime_contract() {
        fn exercise_runtime<R: CompositorRuntime>(runtime: &mut R) {
            assert_eq!(runtime.runtime_name(), "headless-compositor");
            let client = runtime.connect_client("contract-client");
            let surface = runtime
                .submit_surface(client, "contract-window", 320, 200)
                .unwrap_or_else(|error| panic!("{error}"));
            assert_eq!(runtime.client_count(), 1);
            assert_eq!(runtime.surface_count(), 1);

            let first_frame = runtime.present();
            assert_eq!(first_frame.client_count, 1);
            assert_eq!(first_frame.surface_count, 1);
            assert_eq!(first_frame.damaged_surfaces, 1);

            runtime
                .mark_damaged(surface)
                .unwrap_or_else(|error| panic!("{error}"));
            let damage_frame = runtime.present();
            assert_eq!(damage_frame.damaged_surfaces, 1);

            assert_eq!(
                runtime
                    .disconnect_client(client)
                    .unwrap_or_else(|error| panic!("{error}")),
                1
            );
            let cleanup_frame = runtime.present();
            assert_eq!(cleanup_frame.client_count, 0);
            assert_eq!(cleanup_frame.surface_count, 0);
        }

        let mut compositor = HeadlessCompositor::default();
        exercise_runtime(&mut compositor);
    }

    #[test]
    fn headless_backend_tracks_surface_close_and_client_disconnect_damage() {
        let mut compositor = HeadlessCompositor::default();
        let client = compositor.connect_client("scripted-client");
        let terminal = compositor
            .submit_surface(client, "terminal", 800, 600)
            .unwrap();
        let browser = compositor
            .submit_surface(client, "browser", 1024, 768)
            .unwrap();

        let first_frame = compositor.present();
        assert_eq!(first_frame.surface_count, 2);
        assert_eq!(first_frame.damaged_surfaces, 2);
        assert_eq!(compositor.present().damaged_surfaces, 0);

        compositor.close_surface(browser).unwrap();
        let close_frame = compositor.present();
        assert_eq!(close_frame.surface_count, 1);
        assert_eq!(close_frame.damaged_surfaces, 1);

        let removed = compositor.disconnect_client(client).unwrap();
        assert_eq!(removed, 1);
        let disconnect_frame = compositor.present();
        assert_eq!(disconnect_frame.client_count, 0);
        assert_eq!(disconnect_frame.surface_count, 0);
        assert_eq!(disconnect_frame.damaged_surfaces, 1);
        assert_eq!(compositor.present().damaged_surfaces, 0);

        assert_eq!(
            compositor.mark_damaged(terminal).unwrap_err().to_string(),
            "unknown headless surface 1"
        );
    }

    #[test]
    fn headless_backend_rejects_unknown_clients() {
        let mut compositor = HeadlessCompositor::default();
        let error = compositor
            .submit_surface(ClientId(99), "ghost", 10, 10)
            .unwrap_err();

        assert_eq!(error.to_string(), "unknown headless client 99");
    }

    #[test]
    fn smithay_runtime_probe_tracks_feature_and_launch_environment() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_drm_render_access(1, 1)
            .with_input_event_nodes(1)
            .with_input_event_access(0)
            .with_primary_drm_card("/dev/dri/card0")
            .with_primary_drm_render_node("/dev/dri/renderD128")
            .with_primary_input_event("/dev/input/event0")
            .with_seat_broker_tools(true, true, true)
            .with_active_local_session("3", "seat0", "wayland");
        let probe = super::smithay_runtime_probe(&environment);

        assert_eq!(probe.feature_enabled, cfg!(feature = "smithay-backend"));
        if cfg!(all(feature = "smithay-backend", target_os = "linux")) {
            assert!(probe.compiled, "{probe:?}");
            assert_eq!(probe.runtime_backend, "smithay-drm-probe");
            assert_eq!(probe.display_driver, "smithay-drm-kms");
            assert_eq!(probe.input_driver, "smithay-libinput");
            assert_eq!(probe.session_driver, "smithay-libseat-logind");
            assert_eq!(probe.event_loop, "calloop");
            assert_eq!(probe.components.len(), 8);
            assert!(probe.gbm_allocator_component);
            assert!(probe.egl_display_component);
            assert!(probe.gles_renderer_component);
            assert_eq!(
                probe.launch_ready,
                probe.drm_node_resolved
                    && probe.kms_resource_failure.is_none()
                    && probe.kms_scanout_plan_ready
                    && probe.kms_surface_created
                    && probe.kms_surface_failure.is_none()
                    && probe.kms_framebuffer_created
                    && (probe.kms_framebuffer_test_state_succeeded
                        || probe.kms_framebuffer_test_state_permission_denied)
                    && probe.kms_framebuffer_failure.is_none()
                    && probe.kms_first_present_framebuffer_filled
                    && probe.kms_first_present_plane_state_ready
                    && (probe.kms_first_present_blocked_by_drm_master
                        || probe.kms_first_present_commit_succeeded)
                    && (!probe.kms_first_present_commit_succeeded
                        || probe.kms_first_present_vblank_event_received)
                    && probe.kms_first_present_failure.is_none()
                    && probe.renderer_node_selected
                    && probe.renderer_runtime_failure.is_none()
                    && probe.input_runtime_failure.is_none()
            );
            assert_eq!(probe.passed(), probe.launch_ready);
        } else {
            assert!(!probe.compiled, "{probe:?}");
            assert!(!probe.passed(), "{probe:?}");
            assert!(probe.components.is_empty());
            assert!(!probe.drm_node_resolved);
            assert!(!probe.kms_card_opened);
            assert!(!probe.kms_device_created);
            assert!(!probe.kms_event_source_inserted);
            assert!(!probe.kms_event_loop_dispatched);
            assert!(!probe.kms_atomic_modesetting);
            assert_eq!(probe.kms_crtc_count, 0);
            assert_eq!(probe.kms_connector_count, 0);
            assert_eq!(probe.kms_connected_connector_count, 0);
            assert_eq!(probe.kms_mode_count, 0);
            assert_eq!(probe.kms_primary_plane_count, 0);
            assert_eq!(probe.kms_cursor_plane_count, 0);
            assert_eq!(probe.kms_overlay_plane_count, 0);
            assert!(!probe.kms_scanout_plan_ready);
            assert_eq!(probe.kms_scanout_connector_id, 0);
            assert!(probe.kms_scanout_connector_name.is_none());
            assert_eq!(probe.kms_scanout_crtc_id, 0);
            assert_eq!(probe.kms_scanout_primary_plane_id, 0);
            assert_eq!(probe.kms_scanout_mode_width, 0);
            assert_eq!(probe.kms_scanout_mode_height, 0);
            assert_eq!(probe.kms_scanout_mode_refresh_hz, 0);
            assert!(!probe.kms_scanout_mode_preferred);
            assert!(!probe.kms_surface_created);
            assert!(!probe.kms_surface_legacy);
            assert!(!probe.kms_surface_crtc_matches_plan);
            assert!(!probe.kms_surface_primary_plane_matches_plan);
            assert_eq!(probe.kms_surface_pending_connector_count, 0);
            assert_eq!(probe.kms_surface_current_connector_count, 0);
            assert!(!probe.kms_surface_pending_mode_matches_plan);
            assert!(!probe.kms_surface_commit_pending);
            assert!(!probe.kms_surface_dropped_after_pause);
            assert!(!probe.kms_framebuffer_created);
            assert!(!probe.kms_framebuffer_added);
            assert!(!probe.kms_framebuffer_test_state_succeeded);
            assert!(!probe.kms_framebuffer_test_state_permission_denied);
            assert!(!probe.kms_framebuffer_test_allow_modeset);
            assert!(!probe.kms_framebuffer_primary_plane_matches_surface);
            assert_eq!(probe.kms_framebuffer_width, 0);
            assert_eq!(probe.kms_framebuffer_height, 0);
            assert!(!probe.kms_framebuffer_released_before_surface_drop);
            assert!(probe.kms_framebuffer_failure.is_some());
            assert!(!probe.kms_first_present_framebuffer_filled);
            assert!(!probe.kms_first_present_plane_state_ready);
            assert!(!probe.kms_first_present_commit_attempted);
            assert!(!probe.kms_first_present_commit_succeeded);
            assert!(!probe.kms_first_present_vblank_event_received);
            assert!(!probe.kms_first_present_blocked_by_drm_master);
            assert!(probe.kms_first_present_failure.is_some());
            assert!(probe.kms_surface_failure.is_some());
            assert!(probe.kms_resource_failure.is_some());
            assert!(!probe.renderer_node_selected);
            assert!(!probe.gbm_allocator_component);
            assert!(!probe.egl_display_component);
            assert!(!probe.gles_renderer_component);
            assert!(!probe.renderer_node_opened);
            assert!(!probe.gbm_device_created);
            assert!(!probe.gbm_allocator_created);
            assert!(!probe.egl_display_created);
            assert!(!probe.egl_context_created);
            assert!(!probe.gles_renderer_created);
            assert!(!probe.offscreen_buffer_created);
            assert!(!probe.offscreen_frame_rendered);
            assert!(!probe.offscreen_frame_copied);
            assert!(!probe.offscreen_pixel_verified);
            assert_eq!(probe.offscreen_render_width, 0);
            assert_eq!(probe.offscreen_render_height, 0);
            assert_eq!(probe.offscreen_render_pixels, 0);
            assert_eq!(probe.offscreen_sample_red, 0);
            assert_eq!(probe.offscreen_sample_green, 0);
            assert_eq!(probe.offscreen_sample_blue, 0);
            assert_eq!(probe.offscreen_sample_alpha, 0);
            assert!(probe.renderer_runtime_failure.is_some());
            assert!(!probe.libseat_session_created);
            assert!(!probe.libseat_event_source_inserted);
            assert!(!probe.libseat_event_loop_dispatched);
            assert!(!probe.libinput_context_created);
            assert!(!probe.libinput_seat_assigned);
            assert!(!probe.libinput_backend_created);
            assert!(!probe.libinput_event_source_inserted);
            assert!(!probe.libinput_event_loop_dispatched);
            assert!(probe.input_runtime_failure.is_some());
        }
    }

    #[test]
    fn smithay_runtime_bootstrap_tracks_feature_and_display_event_loop_state() {
        let bootstrap = super::smithay_runtime_bootstrap();

        assert_eq!(bootstrap.feature_enabled, cfg!(feature = "smithay-backend"));
        if cfg!(all(feature = "smithay-backend", target_os = "linux")) {
            assert!(bootstrap.compiled, "{bootstrap:?}");
            assert!(bootstrap.passed(), "{bootstrap:?}");
            assert_eq!(bootstrap.runtime_backend, "smithay-drm-bootstrap");
            assert!(bootstrap.display_created);
            assert!(bootstrap.display_handle_created);
            assert!(bootstrap.listening_socket_bound);
            assert!(bootstrap
                .socket_name
                .starts_with("backlit-smithay-bootstrap-"));
            assert!(bootstrap.socket_connect_succeeded);
            assert!(bootstrap.socket_accept_succeeded);
            assert!(bootstrap.client_inserted);
            assert!(bootstrap.display_clients_dispatched);
            assert!(bootstrap.display_clients_flushed);
            assert!(bootstrap.event_loop_created);
            assert!(bootstrap.event_loop_dispatched);
            assert!(bootstrap.failure.is_empty());
        } else {
            assert!(!bootstrap.compiled, "{bootstrap:?}");
            assert!(!bootstrap.passed(), "{bootstrap:?}");
            assert_eq!(bootstrap.failure, "unavailable");
            assert!(!bootstrap.listening_socket_bound);
            assert!(bootstrap.socket_name.is_empty());
            assert!(!bootstrap.socket_connect_succeeded);
            assert!(!bootstrap.socket_accept_succeeded);
            assert!(!bootstrap.client_inserted);
        }
    }

    #[test]
    fn direct_scanout_requires_fullscreen_dmabuf_without_overlays() {
        let mut compositor = HeadlessCompositor::default();
        let client = compositor.connect_client("video-client");
        let video = compositor
            .submit_surface_with_options(
                client,
                "video",
                1920,
                1080,
                SurfaceOptions::dmabuf_fullscreen(),
            )
            .unwrap();

        let eligible = compositor
            .direct_scanout_candidate(video, 1920, 1080)
            .unwrap();
        assert!(eligible.eligible, "{eligible:?}");
        assert_eq!(eligible.reason, "eligible");
        assert_eq!(eligible.buffer_kind, BufferKind::Dmabuf);

        compositor
            .submit_surface(client, "panel", 1920, 42)
            .expect("client should be registered");

        let occluded = compositor
            .direct_scanout_candidate(video, 1920, 1080)
            .unwrap();
        assert!(!occluded.eligible);
        assert_eq!(occluded.reason, "occluded-by-other-surface");

        let mut shm_compositor = HeadlessCompositor::default();
        let client = shm_compositor.connect_client("shm-client");
        let shm = shm_compositor
            .submit_surface_with_options(
                client,
                "video-shm",
                1920,
                1080,
                SurfaceOptions {
                    buffer_kind: BufferKind::Shm,
                    opaque: true,
                    fullscreen: true,
                },
            )
            .unwrap();
        let blocked = shm_compositor
            .direct_scanout_candidate(shm, 1920, 1080)
            .unwrap();

        assert!(!blocked.eligible);
        assert_eq!(blocked.reason, "not-dmabuf");
    }

    #[test]
    fn headless_preflight_is_always_ready() {
        let report = super::preflight_backend(BackendKind::Headless, None, None, "macos");

        assert!(report.ready);
        assert_eq!(report.code, "ready");
    }

    #[test]
    fn wayland_preflight_requires_parent_display_and_runtime_dir() {
        let no_display =
            super::preflight_backend(BackendKind::Wayland, None, Some("/run/user/1000"), "linux");
        assert!(!no_display.ready);
        assert_eq!(no_display.code, "missing-wayland-display");

        let no_runtime =
            super::preflight_backend(BackendKind::Wayland, Some("wayland-0"), None, "linux");
        assert!(!no_runtime.ready);
        assert_eq!(no_runtime.code, "missing-xdg-runtime-dir");

        let ready = super::preflight_backend(
            BackendKind::Wayland,
            Some("wayland-0"),
            Some("/run/user/1000"),
            "linux",
        );
        assert!(ready.ready);
    }

    #[test]
    fn wayland_preflight_rejects_runtime_dir_owned_by_another_user() {
        let mut environment = BackendPreflightEnvironment::for_target("linux")
            .with_wayland_display("wayland-0")
            .with_xdg_runtime_dir("/run/user/0");
        environment.xdg_runtime_dir_owned_by_user = false;

        let report = super::preflight_backend_with_environment(BackendKind::Wayland, &environment);

        assert!(!report.ready);
        assert_eq!(report.code, "wrong-xdg-runtime-dir-owner");
    }

    #[test]
    fn launch_plan_records_nested_wayland_parent_socket() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_wayland_display("wayland-1")
            .with_xdg_runtime_dir("/run/user/1000");
        let report = super::preflight_backend_with_environment(BackendKind::Wayland, &environment);
        let plan = backend_launch_plan(BackendKind::Wayland, &report, &environment);

        assert!(plan.ready);
        assert_eq!(plan.implementation, "nested-wayland-harness");
        assert_eq!(plan.display_driver, "parent-wayland");
        assert_eq!(plan.input_driver, "parent-wayland-seat");
        assert_eq!(plan.device_access, "parent-wayland-socket");
        assert!(plan.uses_parent_wayland);
        assert!(!plan.uses_drm);
    }

    #[test]
    fn drm_preflight_requires_linux() {
        let report =
            super::preflight_backend(BackendKind::Drm, None, Some("/run/user/1000"), "macos");

        assert!(!report.ready);
        assert_eq!(report.code, "requires-linux");
    }

    #[test]
    fn drm_preflight_requires_runtime_dir() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_drm_nodes(1, 1)
            .with_input_event_nodes(2)
            .with_session_id("1");

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);

        assert!(!report.ready);
        assert_eq!(report.code, "missing-xdg-runtime-dir");
    }

    #[test]
    fn drm_preflight_rejects_runtime_dir_owned_by_another_user() {
        let mut environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/0")
            .with_drm_nodes(1, 1)
            .with_input_event_nodes(2)
            .with_session_id("1");
        environment.xdg_runtime_dir_owned_by_user = false;

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);

        assert!(!report.ready);
        assert_eq!(report.code, "wrong-xdg-runtime-dir-owner");
    }

    #[test]
    fn drm_preflight_requires_drm_card_nodes() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_input_event_nodes(2)
            .with_session_id("1");

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);

        assert!(!report.ready);
        assert_eq!(report.code, "missing-drm-card");
    }

    #[test]
    fn drm_preflight_requires_drm_card_access() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_input_event_nodes(2)
            .with_session_id("1");

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);

        assert!(!report.ready);
        assert_eq!(report.code, "unavailable-drm-card-access");
    }

    #[test]
    fn drm_preflight_requires_input_event_nodes() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_session_id("1");

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);

        assert!(!report.ready);
        assert_eq!(report.code, "missing-input-devices");
    }

    #[test]
    fn drm_preflight_requires_logind_session_identity() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_input_event_nodes(2);

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);

        assert!(!report.ready);
        assert_eq!(report.code, "missing-logind-session");
    }

    #[test]
    fn drm_preflight_requires_verified_logind_session_state() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_input_event_nodes(2)
            .with_session_id("1");

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);

        assert!(!report.ready);
        assert_eq!(report.code, "unverified-logind-session");
    }

    #[test]
    fn drm_preflight_requires_active_local_session() {
        let mut inactive = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_input_event_nodes(2)
            .with_active_local_session("1", "seat0", "wayland");
        inactive.session_active = false;

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &inactive);

        assert!(!report.ready);
        assert_eq!(report.code, "inactive-logind-session");

        let mut remote = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_input_event_nodes(2)
            .with_active_local_session("1", "seat0", "wayland");
        remote.session_remote = true;

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &remote);

        assert!(!report.ready);
        assert_eq!(report.code, "remote-logind-session");
    }

    #[test]
    fn drm_preflight_requires_seat_and_specific_session_type() {
        let mut missing_seat = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_input_event_nodes(2)
            .with_active_local_session("1", "seat0", "wayland");
        missing_seat.seat = None;

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &missing_seat);

        assert!(!report.ready);
        assert_eq!(report.code, "missing-seat");

        let unspecified_type = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_input_event_nodes(2)
            .with_active_local_session("1", "seat0", "unspecified");

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &unspecified_type);

        assert!(!report.ready);
        assert_eq!(report.code, "unspecified-session-type");
    }

    #[test]
    fn drm_preflight_requires_input_broker_when_direct_input_unavailable() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_input_event_nodes(2)
            .with_active_local_session("1", "seat0", "wayland");

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);

        assert!(!report.ready);
        assert_eq!(report.code, "missing-input-broker");
    }

    #[test]
    fn drm_preflight_allows_direct_input_without_broker_tools() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_input_event_nodes(2)
            .with_input_event_access(1)
            .with_active_local_session("1", "seat0", "wayland")
            .with_primary_drm_card("/dev/dri/card0")
            .with_primary_drm_render_node("/dev/dri/renderD128")
            .with_primary_input_event("/dev/input/event0");

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);
        let plan = backend_launch_plan(BackendKind::Drm, &report, &environment);

        assert!(report.ready, "{report:?}");
        assert_eq!(report.code, "ready-active-local-session-input-broker");
        assert_eq!(environment.input_broker_mode(), "direct");
        assert!(plan.ready);
        assert_eq!(plan.implementation, "pre-smithay-policy-harness");
        assert_eq!(plan.display_driver, "drm-kms");
        assert_eq!(plan.input_driver, "direct-libinput");
        assert_eq!(plan.device_access, "drm-card-direct-input");
        assert!(plan.uses_drm);
        assert!(plan.uses_libinput);
        assert!(!plan.uses_libseat);
        assert!(plan.drm_card_selected);
        assert!(plan.drm_render_selected);
        assert!(plan.input_event_selected);
        assert_eq!(plan.primary_drm_card.as_deref(), Some("/dev/dri/card0"));
    }

    #[test]
    fn drm_preflight_is_ready_with_runtime_devices_and_session() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_input_event_nodes(2)
            .with_active_local_session("1", "seat0", "wayland")
            .with_seat_broker_tools(true, true, true)
            .with_primary_drm_card("/dev/dri/card0")
            .with_primary_drm_render_node("/dev/dri/renderD128")
            .with_primary_input_event("/dev/input/event0");

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);
        let plan = backend_launch_plan(BackendKind::Drm, &report, &environment);

        assert!(report.ready, "{report:?}");
        assert_eq!(report.code, "ready-active-local-session-input-broker");
        assert_eq!(environment.input_broker_mode(), "logind-libseat");
        assert!(plan.ready);
        assert_eq!(plan.display_driver, "drm-kms");
        assert_eq!(plan.input_driver, "logind-libseat-libinput");
        assert_eq!(plan.device_access, "drm-card-logind-libseat");
        assert!(plan.uses_logind);
        assert!(plan.uses_libseat);
        assert!(plan.uses_libinput);
        assert_eq!(plan.seat.as_deref(), Some("seat0"));
        assert_eq!(plan.session_type.as_deref(), Some("wayland"));
    }
}
