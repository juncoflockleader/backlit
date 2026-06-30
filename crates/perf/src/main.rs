use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_perf::{run_perf_smoke, PerfBudgets};

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-perf: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    let report = run_perf_smoke(config.width, config.height, config.budgets);

    println!(
        "{}",
        event_json(
            "perf.smoke",
            &[
                ("passed", FieldValue::Bool(report.passed())),
                ("render_ms", FieldValue::U64(report.render_ms)),
                (
                    "render_budget_ms",
                    FieldValue::U64(report.budgets.render_budget_ms)
                ),
                ("present_us", FieldValue::U64(report.present_us)),
                (
                    "present_budget_us",
                    FieldValue::U64(report.budgets.present_budget_us),
                ),
                (
                    "pointer_frame_budget_us",
                    FieldValue::U64(report.budgets.pointer_frame_budget_us),
                ),
                (
                    "non_background_pixels",
                    FieldValue::U64(report.non_background_pixels),
                ),
                ("checksum", FieldValue::U64(report.checksum)),
                ("golden_ok", FieldValue::Bool(report.golden_ok)),
                ("protocol_count", FieldValue::U64(report.protocol_count)),
                ("surface_count", FieldValue::U64(report.surface_count)),
                (
                    "initial_damaged_surfaces",
                    FieldValue::U64(report.initial_damaged_surfaces),
                ),
                (
                    "idle_damaged_surfaces",
                    FieldValue::U64(report.idle_damaged_surfaces),
                ),
                (
                    "targeted_damage_surfaces",
                    FieldValue::U64(report.targeted_damage_surfaces),
                ),
                (
                    "post_damage_idle_surfaces",
                    FieldValue::U64(report.post_damage_idle_surfaces),
                ),
                ("frames_presented", FieldValue::U64(report.frames_presented)),
                ("no_idle_redraw", FieldValue::Bool(report.no_idle_redraw)),
                (
                    "targeted_damage_ok",
                    FieldValue::Bool(report.targeted_damage_ok),
                ),
                ("drag_frames", FieldValue::U64(report.drag_frames)),
                (
                    "drag_dropped_frames",
                    FieldValue::U64(report.drag_dropped_frames),
                ),
                (
                    "drag_dropped_frame_budget",
                    FieldValue::U64(report.drag_dropped_frame_budget),
                ),
                (
                    "drag_max_frame_us",
                    FieldValue::U64(report.drag_max_frame_us),
                ),
                (
                    "pointer_frame_p99_us",
                    FieldValue::U64(report.pointer_frame_p99_us),
                ),
                ("drag_damage_ok", FieldValue::Bool(report.drag_damage_ok)),
                (
                    "drag_frame_pacing_ok",
                    FieldValue::Bool(report.drag_frame_pacing_ok),
                ),
                (
                    "screenshot_verified",
                    FieldValue::Bool(report.screenshot_verified),
                ),
                (
                    "protocols_verified",
                    FieldValue::Bool(report.protocols_verified),
                ),
            ],
        )
    );

    if config.verify && !report.passed() {
        return Err(String::from("performance smoke verification failed"));
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Config {
    width: u32,
    height: u32,
    budgets: PerfBudgets,
    verify: bool,
    help: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 800,
            height: 520,
            budgets: PerfBudgets::default(),
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
            } else if let Some(value) = arg.strip_prefix("--width=") {
                config.width = parse_u32("--width", value)?;
            } else if arg == "--width" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --width"))?;
                config.width = parse_u32("--width", &value)?;
            } else if let Some(value) = arg.strip_prefix("--height=") {
                config.height = parse_u32("--height", value)?;
            } else if arg == "--height" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --height"))?;
                config.height = parse_u32("--height", &value)?;
            } else if let Some(value) = arg.strip_prefix("--render-budget-ms=") {
                config.budgets.render_budget_ms = parse_u64("--render-budget-ms", value)?;
            } else if arg == "--render-budget-ms" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --render-budget-ms"))?;
                config.budgets.render_budget_ms = parse_u64("--render-budget-ms", &value)?;
            } else if let Some(value) = arg.strip_prefix("--present-budget-us=") {
                config.budgets.present_budget_us = parse_u64("--present-budget-us", value)?;
            } else if arg == "--present-budget-us" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --present-budget-us"))?;
                config.budgets.present_budget_us = parse_u64("--present-budget-us", &value)?;
            } else if let Some(value) = arg.strip_prefix("--pointer-frame-budget-us=") {
                config.budgets.pointer_frame_budget_us =
                    parse_u64("--pointer-frame-budget-us", value)?;
            } else if arg == "--pointer-frame-budget-us" {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("missing value for --pointer-frame-budget-us"))?;
                config.budgets.pointer_frame_budget_us =
                    parse_u64("--pointer-frame-budget-us", &value)?;
            } else {
                return Err(format!("unknown flag: {arg}"));
            }
        }

        Ok(config)
    }
}

fn parse_u32(flag: &str, value: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn parse_u64(flag: &str, value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn print_help() {
    println!(
        "\
backlit-perf

Usage:
  backlit-perf [--verify] [--width=800] [--height=520] [--render-budget-ms=500] [--present-budget-us=50000] [--pointer-frame-budget-us=16000]

Flags:
  --verify             Fail when the smoke report exceeds the configured budgets.
  --width              Demo GUI render width.
  --height             Demo GUI render height.
  --render-budget-ms   Maximum headless GUI render time.
  --present-budget-us  Maximum headless backend present time.
  --pointer-frame-budget-us
                       Maximum pointer-to-frame latency for drag frames.
"
    );
}
