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
