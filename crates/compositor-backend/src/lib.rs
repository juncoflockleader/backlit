use std::fmt;
use std::fs;
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    pub backend: BackendKind,
    pub socket: String,
    pub smoke_test: bool,
    pub serve: bool,
    pub serve_for_ms: Option<u64>,
    pub idle_probe_ms: Option<u64>,
    pub help: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            backend: BackendKind::Headless,
            socket: String::from("backlit-0"),
            smoke_test: false,
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
    InvalidValue(&'static str, String),
    MissingValue(&'static str),
    UnknownFlag(String),
}

impl fmt::Display for ArgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBackend(value) => write!(f, "invalid backend: {value}"),
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
        } else if arg == "--serve" {
            config.serve = true;
        } else if let Some(value) = arg.strip_prefix("--backend=") {
            config.backend = parse_backend(value)?;
        } else if arg == "--backend" {
            let value = args.next().ok_or(ArgError::MissingValue("--backend"))?;
            config.backend = parse_backend(&value)?;
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

#[derive(Debug, Clone)]
pub struct HeadlessCompositor {
    clients: Vec<HeadlessClient>,
    surfaces: Vec<HeadlessSurface>,
    next_client_id: u64,
    next_surface_id: u64,
    frame: u64,
}

impl Default for HeadlessCompositor {
    fn default() -> Self {
        Self {
            clients: Vec::new(),
            surfaces: Vec::new(),
            next_client_id: 1,
            next_surface_id: 1,
            frame: 0,
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

    pub fn present(&mut self) -> FrameReport {
        self.frame += 1;

        let damaged_surfaces = self
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
        parse_args, BackendKind, BackendPreflightEnvironment, BufferKind, ClientId,
        HeadlessCompositor, RunConfig, SurfaceOptions,
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
            "--socket",
            "backlit-test",
            "--smoke-test",
            "--idle-probe-ms",
            "250",
            "--serve",
            "--serve-for-ms=25",
        ])
        .unwrap();

        assert_eq!(config.backend, BackendKind::Wayland);
        assert_eq!(config.socket, "backlit-test");
        assert!(config.smoke_test);
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
    fn headless_backend_rejects_unknown_clients() {
        let mut compositor = HeadlessCompositor::default();
        let error = compositor
            .submit_surface(ClientId(99), "ghost", 10, 10)
            .unwrap_err();

        assert_eq!(error.to_string(), "unknown headless client 99");
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
            .with_active_local_session("1", "seat0", "wayland");

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);

        assert!(report.ready, "{report:?}");
        assert_eq!(report.code, "ready-active-local-session-input-broker");
        assert_eq!(environment.input_broker_mode(), "direct");
    }

    #[test]
    fn drm_preflight_is_ready_with_runtime_devices_and_session() {
        let environment = BackendPreflightEnvironment::for_target("linux")
            .with_xdg_runtime_dir("/run/user/1000")
            .with_drm_nodes(1, 1)
            .with_drm_card_access(1, 1)
            .with_input_event_nodes(2)
            .with_active_local_session("1", "seat0", "wayland")
            .with_seat_broker_tools(true, true, true);

        let report = super::preflight_backend_with_environment(BackendKind::Drm, &environment);

        assert!(report.ready, "{report:?}");
        assert_eq!(report.code, "ready-active-local-session-input-broker");
        assert_eq!(environment.input_broker_mode(), "logind-libseat");
    }
}
