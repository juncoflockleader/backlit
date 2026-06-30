use std::fmt;
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
    pub help: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            backend: BackendKind::Headless,
            socket: String::from("backlit-0"),
            smoke_test: false,
            help: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgError {
    InvalidBackend(String),
    MissingValue(&'static str),
    UnknownFlag(String),
}

impl fmt::Display for ArgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBackend(value) => write!(f, "invalid backend: {value}"),
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
        } else if let Some(value) = arg.strip_prefix("--backend=") {
            config.backend = parse_backend(value)?;
        } else if arg == "--backend" {
            let value = args.next().ok_or(ArgError::MissingValue("--backend"))?;
            config.backend = parse_backend(&value)?;
        } else if let Some(value) = arg.strip_prefix("--socket=") {
            config.socket = value.to_string();
        } else if arg == "--socket" {
            config.socket = args.next().ok_or(ArgError::MissingValue("--socket"))?;
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessClient {
    pub id: ClientId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameReport {
    pub frame: u64,
    pub client_count: u64,
    pub surface_count: u64,
    pub damaged_surfaces: u64,
    pub total_pixels: u64,
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
    use super::{parse_args, BackendKind, ClientId, HeadlessCompositor, RunConfig};

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
        ])
        .unwrap();

        assert_eq!(config.backend, BackendKind::Wayland);
        assert_eq!(config.socket, "backlit-test");
        assert!(config.smoke_test);
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
}
