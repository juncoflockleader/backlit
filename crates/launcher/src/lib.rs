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

#[cfg(test)]
mod tests {
    use super::{default_catalog, verify_catalog, LaunchTarget, REQUIRED_TARGETS};

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
}
