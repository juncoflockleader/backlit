use std::str::FromStr;

use backlit_launcher::LaunchTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    OpenLauncher,
    Launch(LaunchTarget),
    AppSwitcherNext,
    AppSwitcherPrevious,
    PowerMenu,
}

impl ShortcutAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenLauncher => "open-launcher",
            Self::Launch(LaunchTarget::Terminal) => "launch-terminal",
            Self::Launch(LaunchTarget::Browser) => "launch-browser",
            Self::Launch(LaunchTarget::Settings) => "open-settings",
            Self::AppSwitcherNext => "app-switcher-next",
            Self::AppSwitcherPrevious => "app-switcher-previous",
            Self::PowerMenu => "power-menu",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutBinding {
    pub shortcut: &'static str,
    pub action: ShortcutAction,
    pub mvp_required: bool,
}

impl ShortcutBinding {
    pub const fn new(shortcut: &'static str, action: ShortcutAction, mvp_required: bool) -> Self {
        Self {
            shortcut,
            action,
            mvp_required,
        }
    }
}

pub const MVP_SHORTCUTS: &[ShortcutBinding] = &[
    ShortcutBinding::new("Super+Space", ShortcutAction::OpenLauncher, true),
    ShortcutBinding::new(
        "Super+Enter",
        ShortcutAction::Launch(LaunchTarget::Terminal),
        true,
    ),
    ShortcutBinding::new(
        "Super+B",
        ShortcutAction::Launch(LaunchTarget::Browser),
        true,
    ),
    ShortcutBinding::new(
        "Super+Comma",
        ShortcutAction::Launch(LaunchTarget::Settings),
        true,
    ),
    ShortcutBinding::new("Alt+Tab", ShortcutAction::AppSwitcherNext, true),
    ShortcutBinding::new("Alt+Shift+Tab", ShortcutAction::AppSwitcherPrevious, true),
    ShortcutBinding::new("Super+Escape", ShortcutAction::PowerMenu, false),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutVerification {
    pub required_bindings: usize,
    pub binding_count: usize,
    pub duplicate_shortcuts: Vec<&'static str>,
    pub missing_actions: Vec<ShortcutAction>,
}

impl ShortcutVerification {
    pub fn passed(&self) -> bool {
        self.duplicate_shortcuts.is_empty() && self.missing_actions.is_empty()
    }
}

pub fn resolve_shortcut(shortcut: &str) -> Option<ShortcutAction> {
    MVP_SHORTCUTS
        .iter()
        .find(|binding| binding.shortcut == shortcut)
        .map(|binding| binding.action)
}

pub fn verify_shortcuts(bindings: &[ShortcutBinding]) -> ShortcutVerification {
    let required_actions = [
        ShortcutAction::OpenLauncher,
        ShortcutAction::Launch(LaunchTarget::Terminal),
        ShortcutAction::Launch(LaunchTarget::Browser),
        ShortcutAction::Launch(LaunchTarget::Settings),
        ShortcutAction::AppSwitcherNext,
        ShortcutAction::AppSwitcherPrevious,
    ];

    let mut duplicate_shortcuts = Vec::new();
    for (index, binding) in bindings.iter().enumerate() {
        if bindings[..index]
            .iter()
            .any(|seen| seen.shortcut == binding.shortcut)
        {
            duplicate_shortcuts.push(binding.shortcut);
        }
    }

    let missing_actions = required_actions
        .iter()
        .copied()
        .filter(|action| !bindings.iter().any(|binding| binding.action == *action))
        .collect();

    ShortcutVerification {
        required_bindings: required_actions.len(),
        binding_count: bindings.len(),
        duplicate_shortcuts,
        missing_actions,
    }
}

impl FromStr for ShortcutAction {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "open-launcher" => Ok(Self::OpenLauncher),
            "launch-terminal" => Ok(Self::Launch(LaunchTarget::Terminal)),
            "launch-browser" => Ok(Self::Launch(LaunchTarget::Browser)),
            "open-settings" => Ok(Self::Launch(LaunchTarget::Settings)),
            "app-switcher-next" => Ok(Self::AppSwitcherNext),
            "app-switcher-previous" => Ok(Self::AppSwitcherPrevious),
            "power-menu" => Ok(Self::PowerMenu),
            other => Err(format!("unknown shortcut action '{other}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_shortcut, verify_shortcuts, ShortcutAction, ShortcutBinding, MVP_SHORTCUTS,
    };
    use backlit_launcher::LaunchTarget;

    #[test]
    fn resolves_core_mvp_shortcuts() {
        assert_eq!(
            resolve_shortcut("Super+Enter"),
            Some(ShortcutAction::Launch(LaunchTarget::Terminal))
        );
        assert_eq!(
            resolve_shortcut("Alt+Tab"),
            Some(ShortcutAction::AppSwitcherNext)
        );
    }

    #[test]
    fn verifies_default_shortcuts() {
        let report = verify_shortcuts(MVP_SHORTCUTS);

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.required_bindings, 6);
    }

    #[test]
    fn detects_duplicate_shortcuts() {
        let bindings = [
            ShortcutBinding::new("Super+Enter", ShortcutAction::OpenLauncher, true),
            ShortcutBinding::new(
                "Super+Enter",
                ShortcutAction::Launch(LaunchTarget::Terminal),
                true,
            ),
        ];
        let report = verify_shortcuts(&bindings);

        assert!(!report.passed());
        assert_eq!(report.duplicate_shortcuts, vec!["Super+Enter"]);
    }
}
