use std::env;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_demo_client::{
    render_demo_gui, verify_demo_gui, DEFAULT_DEMO_HEIGHT, DEFAULT_DEMO_WIDTH,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-demo-client: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    if let Some(socket_name) = config.connect_socket.as_deref() {
        let report = connect_to_compositor_socket(&config, socket_name)?;
        println!(
            "{}",
            event_json(
                "demo_client.socket_connected",
                &[
                    ("socket_name", FieldValue::Str(socket_name)),
                    ("socket_path", FieldValue::Str(report.socket_path.as_str())),
                    ("title", FieldValue::Str(config.connect_title.as_str())),
                    ("app_id", FieldValue::Str(config.connect_app_id.as_str())),
                    ("width", FieldValue::U64(config.width as u64)),
                    ("height", FieldValue::U64(config.height as u64)),
                    ("connected", FieldValue::Bool(report.connected)),
                    ("bytes_written", FieldValue::U64(report.bytes_written)),
                ],
            )
        );

        if config.connect_only {
            return Ok(());
        }
    }

    let canvas = render_demo_gui(config.width, config.height);
    canvas
        .write_ppm(&config.output)
        .map_err(|error| format!("failed to write {}: {error}", config.output))?;

    let report = verify_demo_gui(&canvas);

    println!(
        "{}",
        event_json(
            "demo_client.rendered",
            &[
                ("output", FieldValue::Str(config.output.as_str())),
                ("width", FieldValue::U64(canvas.width() as u64)),
                ("height", FieldValue::U64(canvas.height() as u64)),
                (
                    "non_background_pixels",
                    FieldValue::U64(report.non_background_pixels),
                ),
                ("checksum", FieldValue::U64(report.checksum)),
            ],
        )
    );

    if config.verify {
        println!(
            "{}",
            event_json(
                "demo_client.verified",
                &[
                    ("passed", FieldValue::Bool(report.passed())),
                    ("golden_ok", FieldValue::Bool(report.golden_ok)),
                    ("panel_ok", FieldValue::Bool(report.panel_ok)),
                    ("launcher_ok", FieldValue::Bool(report.launcher_ok)),
                    ("window_ok", FieldValue::Bool(report.window_ok)),
                    ("pointer_ok", FieldValue::Bool(report.pointer_ok)),
                ],
            )
        );

        if !report.passed() {
            return Err(String::from("rendered GUI failed verification"));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SocketConnectReport {
    socket_path: String,
    connected: bool,
    bytes_written: u64,
}

fn connect_to_compositor_socket(
    config: &Config,
    socket_name: &str,
) -> Result<SocketConnectReport, String> {
    let socket_path = resolve_socket_path(socket_name)?;
    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|error| format!("failed to connect to {}: {error}", socket_path.display()))?;
    let message = format!(
        "BACKLIT_DEMO_CLIENT surface title={} app_id={} width={} height={}\n",
        protocol_token(config.connect_title.as_str()),
        protocol_token(config.connect_app_id.as_str()),
        config.width.max(1),
        config.height.max(1),
    );
    stream.write_all(message.as_bytes()).map_err(|error| {
        format!(
            "failed to write demo surface to {}: {error}",
            socket_path.display()
        )
    })?;

    Ok(SocketConnectReport {
        socket_path: socket_path.display().to_string(),
        connected: true,
        bytes_written: message.len() as u64,
    })
}

fn resolve_socket_path(socket_name: &str) -> Result<PathBuf, String> {
    if Path::new(socket_name).is_absolute() {
        return Ok(PathBuf::from(socket_name));
    }

    let runtime_dir = env::var("XDG_RUNTIME_DIR")
        .map_err(|_| String::from("XDG_RUNTIME_DIR is required for relative compositor sockets"))?;
    if runtime_dir.trim().is_empty() {
        return Err(String::from(
            "XDG_RUNTIME_DIR is required for relative compositor sockets",
        ));
    }

    Ok(Path::new(runtime_dir.as_str()).join(socket_name))
}

fn protocol_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Config {
    output: String,
    width: u32,
    height: u32,
    verify: bool,
    connect_socket: Option<String>,
    connect_title: String,
    connect_app_id: String,
    connect_only: bool,
    help: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output: String::from("target/backlit-demo-client.ppm"),
            width: DEFAULT_DEMO_WIDTH,
            height: DEFAULT_DEMO_HEIGHT,
            verify: false,
            connect_socket: None,
            connect_title: String::from("demo-client"),
            connect_app_id: String::from("org.backlit.DemoClient"),
            connect_only: false,
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
            } else if let Some(value) = arg.strip_prefix("--output=") {
                config.output = value.to_string();
            } else if arg == "--output" {
                config.output = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --output"))?;
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
            } else if let Some(value) = arg.strip_prefix("--connect-socket=") {
                config.connect_socket = Some(value.to_string());
            } else if arg == "--connect-socket" {
                config.connect_socket = Some(
                    args.next()
                        .ok_or_else(|| String::from("missing value for --connect-socket"))?,
                );
            } else if let Some(value) = arg.strip_prefix("--connect-title=") {
                config.connect_title = value.to_string();
            } else if arg == "--connect-title" {
                config.connect_title = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --connect-title"))?;
            } else if let Some(value) = arg.strip_prefix("--connect-app-id=") {
                config.connect_app_id = value.to_string();
            } else if arg == "--connect-app-id" {
                config.connect_app_id = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --connect-app-id"))?;
            } else if arg == "--connect-only" {
                config.connect_only = true;
            } else {
                return Err(format!("unknown flag: {arg}"));
            }
        }

        Ok(config)
    }
}

fn parse_dimension(flag: &str, value: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn print_help() {
    println!(
        "\
backlit-demo-client

Usage:
  backlit-demo-client [--output=target/backlit-demo-client.ppm] [--width=800] [--height=520] [--verify] [--connect-socket=backlit-0] [--connect-app-id=org.backlit.DemoClient] [--connect-only]

Flags:
  --output          PPM screenshot output path.
  --width           Screenshot or announced surface width in pixels.
  --height          Screenshot or announced surface height in pixels.
  --verify          Fail if expected GUI regions are missing.
  --connect-socket  Connect to a compositor Unix socket and announce a demo surface.
  --connect-title   Surface title to announce when connecting.
  --connect-app-id  Application id to announce when connecting.
  --connect-only    Skip screenshot rendering after the socket announcement.
"
    );
}

#[cfg(test)]
mod tests {
    use super::{protocol_token, Config};

    #[test]
    fn parses_socket_connection_flags() {
        let config = Config::parse([
            "--connect-socket",
            "backlit-test",
            "--connect-title=hello world",
            "--connect-app-id=org.backlit.HelloWorld",
            "--connect-only",
            "--width=640",
            "--height=480",
        ])
        .unwrap();

        assert_eq!(config.connect_socket.as_deref(), Some("backlit-test"));
        assert_eq!(config.connect_title, "hello world");
        assert_eq!(config.connect_app_id, "org.backlit.HelloWorld");
        assert!(config.connect_only);
        assert_eq!(config.width, 640);
        assert_eq!(config.height, 480);
    }

    #[test]
    fn protocol_token_removes_whitespace() {
        assert_eq!(
            protocol_token("hello world/settings"),
            "hello-world-settings"
        );
    }
}
