#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/packaging-contract}"
mkdir -p "$out_dir"

fail() {
  echo "packaging contract verification failed: $*" >&2
  exit 1
}

require_file() {
  test -f "$1" || fail "missing file $1"
}

require_line() {
  file="$1"
  line="$2"
  grep -Fx "$line" "$file" >/dev/null || fail "missing line in $file: $line"
}

require_contains() {
  file="$1"
  value="$2"
  grep -F "$value" "$file" >/dev/null || fail "missing text in $file: $value"
}

require_package() {
  package="$1"
  grep -Fx "Package: $package" packaging/debian/control.stub >/dev/null || fail "missing package $package"
}

require_file packaging/sessions/backlit.desktop
require_file packaging/systemd/backlit-compositor.service
require_file packaging/systemd/backlit-shell.service
require_file packaging/systemd/backlit-notification-daemon.service
require_file packaging/systemd/backlit-settings-daemon.service
require_file packaging/debian/control.stub

require_line packaging/sessions/backlit.desktop "[Desktop Entry]"
require_line packaging/sessions/backlit.desktop "Name=Backlit"
require_line packaging/sessions/backlit.desktop "Exec=backlit-session"
require_line packaging/sessions/backlit.desktop "Type=Application"
require_line packaging/sessions/backlit.desktop "DesktopNames=Backlit"

require_line packaging/systemd/backlit-compositor.service "PartOf=graphical-session.target"
require_line packaging/systemd/backlit-compositor.service "Type=simple"
require_line packaging/systemd/backlit-compositor.service "ExecStart=/usr/bin/backlit-compositor --backend=drm --socket=backlit-0"
require_line packaging/systemd/backlit-compositor.service "Environment=RUST_BACKTRACE=1"
require_line packaging/systemd/backlit-compositor.service "SyslogIdentifier=backlit-compositor"
require_line packaging/systemd/backlit-compositor.service "StandardOutput=journal"
require_line packaging/systemd/backlit-compositor.service "StandardError=journal"
require_line packaging/systemd/backlit-compositor.service "Restart=on-failure"
require_line packaging/systemd/backlit-compositor.service "WantedBy=graphical-session.target"

require_line packaging/systemd/backlit-shell.service "After=backlit-compositor.service"
require_line packaging/systemd/backlit-shell.service "PartOf=graphical-session.target"
require_line packaging/systemd/backlit-shell.service "Type=simple"
require_line packaging/systemd/backlit-shell.service "ExecStart=/usr/bin/backlit-shell --component=all --socket=backlit-0"
require_line packaging/systemd/backlit-shell.service "Environment=RUST_BACKTRACE=1"
require_line packaging/systemd/backlit-shell.service "SyslogIdentifier=backlit-shell"
require_line packaging/systemd/backlit-shell.service "StandardOutput=journal"
require_line packaging/systemd/backlit-shell.service "StandardError=journal"
require_line packaging/systemd/backlit-shell.service "Restart=on-failure"
require_line packaging/systemd/backlit-shell.service "WantedBy=graphical-session.target"

require_line packaging/systemd/backlit-notification-daemon.service "After=backlit-compositor.service"
require_line packaging/systemd/backlit-notification-daemon.service "PartOf=graphical-session.target"
require_line packaging/systemd/backlit-notification-daemon.service "Type=simple"
require_line packaging/systemd/backlit-notification-daemon.service "ExecStart=/usr/bin/backlit-notification-daemon"
require_line packaging/systemd/backlit-notification-daemon.service "Environment=RUST_BACKTRACE=1"
require_line packaging/systemd/backlit-notification-daemon.service "SyslogIdentifier=backlit-notification-daemon"
require_line packaging/systemd/backlit-notification-daemon.service "StandardOutput=journal"
require_line packaging/systemd/backlit-notification-daemon.service "StandardError=journal"
require_line packaging/systemd/backlit-notification-daemon.service "Restart=on-failure"
require_line packaging/systemd/backlit-notification-daemon.service "WantedBy=graphical-session.target"

require_line packaging/systemd/backlit-settings-daemon.service "After=backlit-compositor.service"
require_line packaging/systemd/backlit-settings-daemon.service "PartOf=graphical-session.target"
require_line packaging/systemd/backlit-settings-daemon.service "Type=simple"
require_line packaging/systemd/backlit-settings-daemon.service "ExecStart=/usr/bin/backlit-settings-daemon"
require_line packaging/systemd/backlit-settings-daemon.service "Environment=RUST_BACKTRACE=1"
require_line packaging/systemd/backlit-settings-daemon.service "SyslogIdentifier=backlit-settings-daemon"
require_line packaging/systemd/backlit-settings-daemon.service "StandardOutput=journal"
require_line packaging/systemd/backlit-settings-daemon.service "StandardError=journal"
require_line packaging/systemd/backlit-settings-daemon.service "Restart=on-failure"
require_line packaging/systemd/backlit-settings-daemon.service "WantedBy=graphical-session.target"

for package in \
  fastgui-compositor \
  fastgui-shell \
  fastgui-session \
  fastgui-portal \
  fastgui-settings \
  fastgui-desktop \
  fastgui-dev-tools
do
  require_package "$package"
done

require_contains packaging/debian/control.stub "fastgui-session, fastgui-portal, fastgui-settings"
require_contains Cargo.toml "\"crates/compositor\""
require_contains Cargo.toml "\"crates/notification-daemon\""
require_contains Cargo.toml "\"crates/session\""
require_contains Cargo.toml "\"crates/shell\""
require_contains Cargo.toml "\"crates/settings-daemon\""

package_count="$(grep -c '^Package: ' packaging/debian/control.stub)"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-packaging-contract",
  "passed": true,
  "package_count": $package_count,
  "artifacts": {
    "session_desktop": "packaging/sessions/backlit.desktop",
    "compositor_service": "packaging/systemd/backlit-compositor.service",
    "shell_service": "packaging/systemd/backlit-shell.service",
    "notification_daemon_service": "packaging/systemd/backlit-notification-daemon.service",
    "settings_daemon_service": "packaging/systemd/backlit-settings-daemon.service",
    "debian_control_stub": "packaging/debian/control.stub"
  },
  "checks": {
    "desktop_entry": true,
    "systemd_services": true,
    "journal_logging": true,
    "package_split": true,
    "workspace_binaries": true
  }
}
EOF

printf 'Backlit packaging contract verification passed. Artifacts: %s\n' "$out_dir"
