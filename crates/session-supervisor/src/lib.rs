#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionProcessRole {
    Compositor,
    ShellWallpaper,
    ShellPanel,
    ShellLauncher,
    ShellAppSwitcher,
}

impl SessionProcessRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compositor => "compositor",
            Self::ShellWallpaper => "shell-wallpaper",
            Self::ShellPanel => "shell-panel",
            Self::ShellLauncher => "shell-launcher",
            Self::ShellAppSwitcher => "shell-app-switcher",
        }
    }

    pub fn critical(self) -> bool {
        matches!(self, Self::Compositor)
    }

    pub fn restartable(self) -> bool {
        !self.critical()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionProcess {
    pub role: SessionProcessRole,
    pub running: bool,
    pub restart_count: u32,
}

impl SessionProcess {
    pub const fn new(role: SessionProcessRole) -> Self {
        Self {
            role,
            running: true,
            restart_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSupervisor {
    processes: Vec<SessionProcess>,
}

impl Default for SessionSupervisor {
    fn default() -> Self {
        Self {
            processes: vec![
                SessionProcess::new(SessionProcessRole::Compositor),
                SessionProcess::new(SessionProcessRole::ShellWallpaper),
                SessionProcess::new(SessionProcessRole::ShellPanel),
                SessionProcess::new(SessionProcessRole::ShellLauncher),
                SessionProcess::new(SessionProcessRole::ShellAppSwitcher),
            ],
        }
    }
}

impl SessionSupervisor {
    pub fn crash(&mut self, role: SessionProcessRole) -> CrashReport {
        let compositor_alive_before = self.compositor_alive();

        let Some(process) = self
            .processes
            .iter_mut()
            .find(|process| process.role == role)
        else {
            return CrashReport {
                role,
                known_process: false,
                compositor_alive: compositor_alive_before,
                restarted: false,
                session_alive: compositor_alive_before,
            };
        };

        process.running = false;
        let restarted = if process.role.restartable() {
            process.running = true;
            process.restart_count += 1;
            true
        } else {
            false
        };

        let compositor_alive = self.compositor_alive();

        CrashReport {
            role,
            known_process: true,
            compositor_alive,
            restarted,
            session_alive: compositor_alive,
        }
    }

    pub fn compositor_alive(&self) -> bool {
        self.processes
            .iter()
            .any(|process| process.role == SessionProcessRole::Compositor && process.running)
    }

    pub fn processes(&self) -> &[SessionProcess] {
        &self.processes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrashReport {
    pub role: SessionProcessRole,
    pub known_process: bool,
    pub compositor_alive: bool,
    pub restarted: bool,
    pub session_alive: bool,
}

impl CrashReport {
    pub fn shell_crash_isolated(self) -> bool {
        self.known_process && self.role.restartable() && self.compositor_alive && self.restarted
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrashSmokeReport {
    pub shell_crash_isolated: bool,
    pub compositor_crash_ends_session: bool,
    pub restarted_shells: u64,
}

impl CrashSmokeReport {
    pub fn passed(self) -> bool {
        self.shell_crash_isolated
            && self.compositor_crash_ends_session
            && self.restarted_shells == 1
    }
}

pub fn run_crash_smoke() -> CrashSmokeReport {
    let mut supervisor = SessionSupervisor::default();
    let shell_crash = supervisor.crash(SessionProcessRole::ShellPanel);
    let restarted_shells = supervisor
        .processes()
        .iter()
        .filter(|process| process.role.restartable() && process.restart_count > 0)
        .count() as u64;

    let compositor_crash = supervisor.crash(SessionProcessRole::Compositor);

    CrashSmokeReport {
        shell_crash_isolated: shell_crash.shell_crash_isolated(),
        compositor_crash_ends_session: !compositor_crash.session_alive,
        restarted_shells,
    }
}

#[cfg(test)]
mod tests {
    use super::{run_crash_smoke, SessionProcessRole, SessionSupervisor};

    #[test]
    fn shell_crash_restarts_without_killing_compositor() {
        let mut supervisor = SessionSupervisor::default();
        let report = supervisor.crash(SessionProcessRole::ShellPanel);

        assert!(report.shell_crash_isolated(), "{report:?}");
        assert!(supervisor.compositor_alive());
    }

    #[test]
    fn compositor_crash_ends_session() {
        let mut supervisor = SessionSupervisor::default();
        let report = supervisor.crash(SessionProcessRole::Compositor);

        assert!(!report.restarted);
        assert!(!report.session_alive);
    }

    #[test]
    fn crash_smoke_passes() {
        let report = run_crash_smoke();

        assert!(report.passed(), "{report:?}");
    }
}
