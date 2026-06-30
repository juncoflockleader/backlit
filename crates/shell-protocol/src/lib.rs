use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellSurfaceRole {
    Wallpaper,
    Panel,
    Launcher,
    AppSwitcher,
    NotificationHost,
    LockScreen,
}

impl ShellSurfaceRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wallpaper => "wallpaper",
            Self::Panel => "panel",
            Self::Launcher => "launcher",
            Self::AppSwitcher => "app-switcher",
            Self::NotificationHost => "notification-host",
            Self::LockScreen => "lock-screen",
        }
    }

    pub fn mvp_required(self) -> bool {
        matches!(
            self,
            Self::Wallpaper | Self::Panel | Self::Launcher | Self::AppSwitcher
        )
    }
}

impl FromStr for ShellSurfaceRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "wallpaper" => Ok(Self::Wallpaper),
            "panel" => Ok(Self::Panel),
            "launcher" => Ok(Self::Launcher),
            "app-switcher" | "switcher" => Ok(Self::AppSwitcher),
            "notification-host" | "notifications" => Ok(Self::NotificationHost),
            "lock-screen" => Ok(Self::LockScreen),
            other => Err(format!("unknown shell role '{other}'")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRegistration {
    pub role: ShellSurfaceRole,
    pub output: Option<String>,
}

impl ShellRegistration {
    pub fn new(role: ShellSurfaceRole) -> Self {
        Self { role, output: None }
    }

    pub fn for_output(mut self, output: impl Into<String>) -> Self {
        self.output = Some(output.into());
        self
    }
}

pub const MVP_SHELL_ROLES: &[ShellSurfaceRole] = &[
    ShellSurfaceRole::Wallpaper,
    ShellSurfaceRole::Panel,
    ShellSurfaceRole::Launcher,
    ShellSurfaceRole::AppSwitcher,
];

#[cfg(test)]
mod tests {
    use super::{ShellRegistration, ShellSurfaceRole, MVP_SHELL_ROLES};

    #[test]
    fn parses_shell_surface_roles() {
        assert_eq!("panel".parse(), Ok(ShellSurfaceRole::Panel));
        assert_eq!(
            "notifications".parse(),
            Ok(ShellSurfaceRole::NotificationHost)
        );
        assert_eq!("switcher".parse(), Ok(ShellSurfaceRole::AppSwitcher));
    }

    #[test]
    fn records_optional_output_binding() {
        let registration = ShellRegistration::new(ShellSurfaceRole::Panel).for_output("eDP-1");

        assert_eq!(registration.output.as_deref(), Some("eDP-1"));
    }

    #[test]
    fn mvp_shell_roles_are_explicit() {
        assert_eq!(MVP_SHELL_ROLES.len(), 4);
        assert!(MVP_SHELL_ROLES.iter().all(|role| role.mvp_required()));
    }
}
