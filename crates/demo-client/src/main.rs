use std::env;
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
struct Config {
    output: String,
    width: u32,
    height: u32,
    verify: bool,
    help: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output: String::from("target/backlit-demo-client.ppm"),
            width: DEFAULT_DEMO_WIDTH,
            height: DEFAULT_DEMO_HEIGHT,
            verify: false,
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
  backlit-demo-client [--output=target/backlit-demo-client.ppm] [--width=800] [--height=520] [--verify]

Flags:
  --output   PPM screenshot output path.
  --width    Screenshot width in pixels.
  --height   Screenshot height in pixels.
  --verify   Fail if expected GUI regions are missing.
"
    );
}
