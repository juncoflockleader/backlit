use std::env;
use std::process;

use backlit_common::metrics::{event_json, FieldValue};
use backlit_session_supervisor::{run_crash_smoke, CrashLogRecord};

fn main() {
    if let Err(error) = run() {
        eprintln!("backlit-session-supervisor: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse(env::args().skip(1))?;

    if config.help {
        print_help();
        return Ok(());
    }

    let report = run_crash_smoke();
    emit_crash_log(report.shell_log);
    emit_crash_log(report.compositor_log);

    println!(
        "{}",
        event_json(
            "supervisor.crash_smoke",
            &[
                ("passed", FieldValue::Bool(report.passed())),
                (
                    "shell_crash_isolated",
                    FieldValue::Bool(report.shell_crash_isolated),
                ),
                (
                    "compositor_crash_ends_session",
                    FieldValue::Bool(report.compositor_crash_ends_session),
                ),
                ("restarted_shells", FieldValue::U64(report.restarted_shells)),
                (
                    "crash_logs_recorded",
                    FieldValue::Bool(report.crash_logs_recorded()),
                ),
                (
                    "journalctl_user_scope",
                    FieldValue::Bool(report.journalctl_user_scope()),
                ),
                (
                    "shell_journal_unit",
                    FieldValue::Str(report.shell_log.journal_unit),
                ),
                (
                    "compositor_journal_unit",
                    FieldValue::Str(report.compositor_log.journal_unit),
                ),
                (
                    "shell_syslog_identifier",
                    FieldValue::Str(report.shell_log.syslog_identifier),
                ),
                (
                    "compositor_syslog_identifier",
                    FieldValue::Str(report.compositor_log.syslog_identifier),
                ),
            ],
        )
    );

    if config.verify && !report.passed() {
        return Err(String::from("session supervisor crash smoke failed"));
    }

    Ok(())
}

fn emit_crash_log(record: CrashLogRecord) {
    println!(
        "{}",
        event_json(
            "supervisor.crash_log",
            &[
                ("role", FieldValue::Str(record.role.as_str())),
                ("journal_unit", FieldValue::Str(record.journal_unit)),
                (
                    "syslog_identifier",
                    FieldValue::Str(record.syslog_identifier),
                ),
                (
                    "journalctl_user_scope",
                    FieldValue::Bool(record.journalctl_user_scope),
                ),
                ("critical", FieldValue::Bool(record.critical)),
                ("restartable", FieldValue::Bool(record.restartable)),
                ("known_process", FieldValue::Bool(record.known_process)),
                ("restarted", FieldValue::Bool(record.restarted)),
                ("session_alive", FieldValue::Bool(record.session_alive)),
                ("recorded", FieldValue::Bool(record.recorded())),
            ],
        )
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Config {
    verify: bool,
    help: bool,
}

impl Config {
    fn parse<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut config = Self::default();

        for arg in args.into_iter().map(Into::into) {
            if arg == "--help" || arg == "-h" {
                config.help = true;
            } else if arg == "--verify" {
                config.verify = true;
            } else {
                return Err(format!("unknown flag: {arg}"));
            }
        }

        Ok(config)
    }
}

fn print_help() {
    println!(
        "\
backlit-session-supervisor

Usage:
  backlit-session-supervisor [--verify]

Flags:
  --verify  Fail if shell crash isolation or compositor crash handling is incorrect.
"
    );
}
