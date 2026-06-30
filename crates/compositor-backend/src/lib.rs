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

#[cfg(test)]
mod tests {
    use super::{parse_args, BackendKind, RunConfig};

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
}
