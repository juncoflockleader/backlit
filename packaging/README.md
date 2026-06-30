# Packaging

Backlit ships as packages before it ships as an ISO.

Initial package split from the design:

- `fastgui-compositor`
- `fastgui-shell` for panel, launcher, wallpaper, notifications, and shell support processes.
- `fastgui-session`
- `fastgui-portal`
- `fastgui-settings` for the settings daemon and Backlit Settings app entry.
- `fastgui-desktop`
- `fastgui-dev-tools`

The package names can keep the design vocabulary while the source repository and crate names use Backlit branding.

The session package installs `packaging/sessions/backlit.desktop`, `packaging/systemd/backlit-session.target`, and the user services for compositor, shell, notification daemon, and settings daemon. The desktop entry launches `backlit-session --backend=drm --activate-systemd`; headless remains the development default for direct commands and CI harnesses. The session launcher verifies a dry-run `systemctl --user import-environment`, `start backlit-session.target`, and `stop backlit-session.target` plan, and the activation verifier executes that sequence through a fake `systemctl` before real service activation is used on a host. The import plan carries the runtime and logind context required by the DRM service path: `XDG_RUNTIME_DIR`, `XDG_SESSION_ID`, `XDG_SEAT`, `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`, `XDG_CURRENT_DESKTOP`, and `DESKTOP_SESSION`.
