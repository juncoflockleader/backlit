use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchTarget {
    Terminal,
    Browser,
    Settings,
}

impl LaunchTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Browser => "browser",
            Self::Settings => "settings",
        }
    }

    pub fn default_command(self) -> LaunchCommand {
        match self {
            Self::Terminal => LaunchCommand::new(self, "foot", &[]),
            Self::Browser => LaunchCommand::new(self, "xdg-open", &["https://start.backlit.local"]),
            Self::Settings => LaunchCommand::new(self, "backlit-settings", &[]),
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            Self::Terminal => "Super+Enter",
            Self::Browser => "Super+B",
            Self::Settings => "Super+Comma",
        }
    }
}

impl FromStr for LaunchTarget {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "terminal" | "term" => Ok(Self::Terminal),
            "browser" | "web" => Ok(Self::Browser),
            "settings" => Ok(Self::Settings),
            other => Err(format!("unknown launch target '{other}'")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchCommand {
    pub target: LaunchTarget,
    pub program: &'static str,
    pub args: &'static [&'static str],
}

impl LaunchCommand {
    pub const fn new(
        target: LaunchTarget,
        program: &'static str,
        args: &'static [&'static str],
    ) -> Self {
        Self {
            target,
            program,
            args,
        }
    }

    pub fn shell_words(&self) -> String {
        if self.args.is_empty() {
            self.program.to_string()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherVerification {
    pub required_targets: usize,
    pub command_count: usize,
    pub missing_targets: Vec<LaunchTarget>,
    pub empty_programs: Vec<LaunchTarget>,
}

impl LauncherVerification {
    pub fn passed(&self) -> bool {
        self.missing_targets.is_empty()
            && self.empty_programs.is_empty()
            && self.command_count == self.required_targets
    }
}

pub const REQUIRED_TARGETS: &[LaunchTarget] = &[
    LaunchTarget::Terminal,
    LaunchTarget::Browser,
    LaunchTarget::Settings,
];

pub fn default_catalog() -> Vec<LaunchCommand> {
    REQUIRED_TARGETS
        .iter()
        .copied()
        .map(LaunchTarget::default_command)
        .collect()
}

pub fn resolve_command(commands: &[LaunchCommand], target: LaunchTarget) -> Option<&LaunchCommand> {
    commands.iter().find(|command| command.target == target)
}

pub fn verify_catalog(commands: &[LaunchCommand]) -> LauncherVerification {
    let mut missing_targets = Vec::new();
    let mut empty_programs = Vec::new();

    for target in REQUIRED_TARGETS {
        match commands.iter().find(|command| command.target == *target) {
            Some(command) if command.program.trim().is_empty() => empty_programs.push(*target),
            Some(_) => {}
            None => missing_targets.push(*target),
        }
    }

    LauncherVerification {
        required_targets: REQUIRED_TARGETS.len(),
        command_count: commands.len(),
        missing_targets,
        empty_programs,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub terminal: bool,
}

impl DesktopEntry {
    pub fn command_program(&self) -> &str {
        self.exec
            .split_whitespace()
            .next()
            .unwrap_or(self.exec.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopEntryError {
    MissingName,
    MissingExec,
    NotApplication,
    Hidden,
}

pub fn parse_desktop_entry(
    id: impl Into<String>,
    contents: &str,
) -> Result<DesktopEntry, DesktopEntryError> {
    let mut in_desktop_entry = false;
    let mut entry_type = None;
    let mut name = None;
    let mut exec = None;
    let mut terminal = false;
    let mut hidden = false;
    let mut no_display = false;

    for line in contents.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        match key {
            "Type" => entry_type = Some(value.to_string()),
            "Name" => name = Some(value.to_string()),
            "Exec" => exec = Some(strip_exec_field_codes(value)),
            "Terminal" => terminal = value.eq_ignore_ascii_case("true"),
            "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
            "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }

    if entry_type.as_deref() != Some("Application") {
        return Err(DesktopEntryError::NotApplication);
    }

    if hidden || no_display {
        return Err(DesktopEntryError::Hidden);
    }

    Ok(DesktopEntry {
        id: id.into(),
        name: name
            .filter(|name| !name.trim().is_empty())
            .ok_or(DesktopEntryError::MissingName)?,
        exec: exec
            .filter(|exec| !exec.trim().is_empty())
            .ok_or(DesktopEntryError::MissingExec)?,
        terminal,
    })
}

pub fn discover_desktop_entries(dir: impl AsRef<Path>) -> io::Result<Vec<DesktopEntry>> {
    let mut paths = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("desktop") {
            paths.push(path);
        }
    }

    paths.sort();

    let mut entries = Vec::new();
    for path in paths {
        let contents = fs::read_to_string(&path)?;
        let id = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown.desktop")
            .to_string();

        if let Ok(entry) = parse_desktop_entry(id, &contents) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

pub fn discover_desktop_entries_in_dirs<I, P>(dirs: I) -> io::Result<Vec<DesktopEntry>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    for dir in dirs {
        match discover_desktop_entries(dir.as_ref()) {
            Ok(discovered) => {
                for entry in discovered {
                    if seen.insert(entry.id.clone()) {
                        entries.push(entry);
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }

    Ok(entries)
}

pub fn default_desktop_entry_dirs() -> Vec<PathBuf> {
    default_desktop_entry_dirs_from(
        std::env::var("HOME").ok().as_deref(),
        std::env::var("XDG_DATA_HOME").ok().as_deref(),
        std::env::var("XDG_DATA_DIRS").ok().as_deref(),
    )
}

fn default_desktop_entry_dirs_from(
    home: Option<&str>,
    xdg_data_home: Option<&str>,
    xdg_data_dirs: Option<&str>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(xdg_data_home) = xdg_data_home.filter(|value| !value.trim().is_empty()) {
        dirs.push(Path::new(xdg_data_home).join("applications"));
    } else if let Some(home) = home.filter(|value| !value.trim().is_empty()) {
        dirs.push(Path::new(home).join(".local/share/applications"));
    }

    let data_dirs = xdg_data_dirs
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("/usr/local/share:/usr/share");
    for dir in data_dirs
        .split(':')
        .filter(|value| !value.trim().is_empty())
    {
        dirs.push(Path::new(dir).join("applications"));
    }

    dirs
}

fn strip_exec_field_codes(value: &str) -> String {
    value
        .split_whitespace()
        .filter(|word| !matches!(*word, "%f" | "%F" | "%u" | "%U" | "%i" | "%c" | "%k"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{
        default_catalog, default_desktop_entry_dirs_from, discover_desktop_entries_in_dirs,
        parse_desktop_entry, resolve_command, verify_catalog, DesktopEntryError, LaunchTarget,
        REQUIRED_TARGETS,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn default_catalog_covers_required_targets() {
        let catalog = default_catalog();
        let report = verify_catalog(&catalog);

        assert!(report.passed(), "{report:?}");
        assert_eq!(catalog.len(), REQUIRED_TARGETS.len());
    }

    #[test]
    fn launcher_shortcuts_cover_core_targets() {
        assert_eq!(LaunchTarget::Terminal.shortcut(), "Super+Enter");
        assert_eq!(LaunchTarget::Browser.shortcut(), "Super+B");
        assert_eq!(LaunchTarget::Settings.shortcut(), "Super+Comma");
    }

    #[test]
    fn detects_missing_targets() {
        let mut catalog = default_catalog();
        catalog.retain(|command| command.target != LaunchTarget::Browser);
        let report = verify_catalog(&catalog);

        assert!(!report.passed());
        assert_eq!(report.missing_targets, vec![LaunchTarget::Browser]);
    }

    #[test]
    fn resolves_launch_targets() {
        let catalog = default_catalog();
        let command = resolve_command(&catalog, LaunchTarget::Terminal).unwrap();

        assert_eq!(command.program, "foot");
        assert_eq!(command.target.shortcut(), "Super+Enter");
    }

    #[test]
    fn parses_visible_desktop_entries() {
        let entry = parse_desktop_entry(
            "org.backlit.Terminal.desktop",
            "\
[Desktop Entry]
Type=Application
Name=Terminal
Exec=foot %F
Terminal=false
",
        )
        .unwrap();

        assert_eq!(entry.name, "Terminal");
        assert_eq!(entry.exec, "foot");
        assert_eq!(entry.command_program(), "foot");
    }

    #[test]
    fn rejects_hidden_desktop_entries() {
        let error = parse_desktop_entry(
            "hidden.desktop",
            "\
[Desktop Entry]
Type=Application
Name=Hidden
Exec=hidden-app
NoDisplay=true
",
        )
        .unwrap_err();

        assert_eq!(error, DesktopEntryError::Hidden);
    }

    #[test]
    fn builds_default_desktop_entry_dirs_from_xdg_environment() {
        let dirs = default_desktop_entry_dirs_from(
            Some("/home/backlit"),
            Some("/tmp/backlit-data"),
            Some("/opt/share:/usr/share"),
        );

        assert_eq!(
            dirs,
            vec![
                std::path::PathBuf::from("/tmp/backlit-data/applications"),
                std::path::PathBuf::from("/opt/share/applications"),
                std::path::PathBuf::from("/usr/share/applications"),
            ]
        );
    }

    #[test]
    fn discovers_desktop_entries_across_dirs_with_user_precedence() {
        let root = unique_test_dir("launcher-desktop-dirs");
        let user_dir = root.join("user");
        let system_dir = root.join("system");
        fs::create_dir_all(&user_dir).expect("user dir should be created");
        fs::create_dir_all(&system_dir).expect("system dir should be created");
        fs::write(
            user_dir.join("org.backlit.Terminal.desktop"),
            "\
[Desktop Entry]
Type=Application
Name=User Terminal
Exec=user-terminal
",
        )
        .expect("user entry should be written");
        fs::write(
            system_dir.join("org.backlit.Terminal.desktop"),
            "\
[Desktop Entry]
Type=Application
Name=System Terminal
Exec=system-terminal
",
        )
        .expect("system duplicate should be written");
        fs::write(
            system_dir.join("org.backlit.Browser.desktop"),
            "\
[Desktop Entry]
Type=Application
Name=Browser
Exec=browser %U
",
        )
        .expect("system entry should be written");

        let entries = discover_desktop_entries_in_dirs([user_dir, system_dir])
            .expect("desktop entries should be discovered");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "User Terminal");
        assert_eq!(entries[0].exec, "user-terminal");
        assert_eq!(entries[1].name, "Browser");
        assert_eq!(entries[1].exec, "browser");
    }

    fn unique_test_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("backlit-{name}-{}-{nanos}", std::process::id()))
    }
}
