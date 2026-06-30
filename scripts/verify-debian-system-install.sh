#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/debian-system-install}"
package_build_dir="${2:-$out_dir/package-build}"
debs_dir="$package_build_dir/debs"
manifest="$out_dir/manifest.json"
dpkg_install_log="$out_dir/dpkg-install.log"
dpkg_purge_log="$out_dir/dpkg-purge.log"
systemd_units_log="$out_dir/systemd-units.jsonl"
session_log="$out_dir/session.jsonl"
settings_app_log="$out_dir/settings-app.jsonl"
session_replay_dir="$out_dir/session-replay"
session_screenshot="$out_dir/system-installed-session.ppm"
service_log_dir="$out_dir/session-services"
mkdir -p "$out_dir"

packages="fastgui-compositor fastgui-shell fastgui-settings fastgui-session fastgui-core"
purge_packages="fastgui-dev-tools fastgui-desktop fastgui-core fastgui-portal fastgui-session fastgui-settings fastgui-shell fastgui-compositor"
installed_packages=false

fail() {
  echo "Debian system package install verification failed: $*" >&2
  exit 1
}

require_file() {
  test -f "$1" || fail "missing file $1"
}

require_executable() {
  test -x "$1" || fail "missing executable $1"
}

require_line() {
  file="$1"
  line="$2"
  grep -Fx "$line" "$file" >/dev/null || fail "missing line in $file: $line"
}

write_skipped_manifest() {
  reason="$1"
  cat > "$manifest" <<EOF
{
  "name": "backlit-debian-system-install",
  "passed": true,
  "system_install_checked": true,
  "system_install_performed": false,
  "install_blocked_expected": true,
  "install_blocked_reason": "$reason",
  "artifacts": {
    "package_build_manifest": "$package_build_dir/manifest.json"
  },
  "checks": {
    "system_install_checked": true,
    "install_blocked_expected": true,
    "actual_system_dpkg_install": false,
    "dpkg_database_status": false,
    "usr_bin_session_launch": false,
    "systemd_units_from_system_install": false,
    "session_gui_from_system_install": false,
    "session_services_from_system_install": false,
    "session_replay_from_system_install": false,
    "session_clean_exit_from_system_install": false,
    "settings_app_from_system_install": false,
    "packages_purged_after_verification": false
  }
}
EOF
  printf 'Backlit Debian system package install verification skipped as expected: %s. Artifacts: %s\n' "$reason" "$out_dir"
}

cleanup_packages() {
  if [ "$installed_packages" = true ]; then
    dpkg --purge $purge_packages >> "$dpkg_purge_log" 2>&1 || true
  fi
}

require_status_installed() {
  package="$1"
  status="$(dpkg-query -W -f='${Status}' "$package" 2>/dev/null || true)"
  test "$status" = "install ok installed" || fail "dpkg database missing installed package $package"
}

require_status_not_installed() {
  package="$1"
  status="$(dpkg-query -W -f='${Status}' "$package" 2>/dev/null || true)"
  if [ "$status" = "install ok installed" ]; then
    fail "package still installed after cleanup: $package"
  fi
}

deb_for() {
  package="$1"
  match="$(find "$debs_dir" -maxdepth 1 -name "${package}_*.deb" -type f | sort | sed -n '1p')"
  test -n "$match" || fail "missing deb for $package"
  printf '%s\n' "$match"
}

if [ "$(uname -s)" != "Linux" ]; then
  write_skipped_manifest "non-linux-host"
  exit 0
fi

if ! command -v dpkg >/dev/null 2>&1 || ! command -v dpkg-query >/dev/null 2>&1; then
  write_skipped_manifest "missing-dpkg-tools"
  exit 0
fi

if [ "$(id -u)" != "0" ]; then
  write_skipped_manifest "requires-root"
  exit 0
fi

if [ "${BACKLIT_ALLOW_SYSTEM_PACKAGE_INSTALL:-}" != "1" ]; then
  write_skipped_manifest "requires-explicit-allow"
  exit 0
fi

if [ ! -f "$package_build_dir/manifest.json" ]; then
  ./scripts/verify-debian-package-build.sh "$package_build_dir" >/dev/null
fi

build_manifest="$package_build_dir/manifest.json"
require_file "$build_manifest"
if ! grep '"debs_built": true' "$build_manifest" >/dev/null; then
  write_skipped_manifest "package-build-blocked"
  exit 0
fi

install_debs=""
for package in $packages; do
  install_debs="$install_debs $(deb_for "$package")"
done

: > "$dpkg_purge_log"
dpkg --purge $purge_packages >> "$dpkg_purge_log" 2>&1 || true

installed_packages=true
trap cleanup_packages EXIT INT TERM
dpkg --install $install_debs > "$dpkg_install_log" 2>&1

for package in $packages; do
  require_status_installed "$package"
done

require_executable /usr/bin/backlit-compositor
require_executable /usr/bin/backlit-shell
require_executable /usr/bin/backlit-notification-daemon
require_executable /usr/bin/backlit-session
require_executable /usr/bin/backlit-settings
require_executable /usr/bin/backlit-settings-daemon

session_desktop="/usr/share/wayland-sessions/backlit.desktop"
settings_desktop="/usr/share/applications/org.backlit.Settings.desktop"
systemd_dir="/usr/lib/systemd/user"

require_file "$session_desktop"
require_file "$settings_desktop"
require_file "$systemd_dir/backlit-session.target"
require_file "$systemd_dir/backlit-compositor.service"
require_file "$systemd_dir/backlit-shell.service"
require_file "$systemd_dir/backlit-notification-daemon.service"
require_file "$systemd_dir/backlit-settings-daemon.service"

require_line "$session_desktop" "Exec=backlit-session --backend=drm --activate-systemd"
require_line "$settings_desktop" "Exec=backlit-settings"
require_line "$systemd_dir/backlit-session.target" "Wants=backlit-compositor.service backlit-shell.service backlit-notification-daemon.service backlit-settings-daemon.service"
require_line "$systemd_dir/backlit-compositor.service" "ExecStart=/usr/bin/backlit-compositor --backend=drm --socket=backlit-0 --serve"
require_line "$systemd_dir/backlit-shell.service" "ExecStart=/usr/bin/backlit-shell --component=all --socket=backlit-0 --serve"
require_line "$systemd_dir/backlit-notification-daemon.service" "ExecStart=/usr/bin/backlit-notification-daemon --serve"
require_line "$systemd_dir/backlit-settings-daemon.service" "ExecStart=/usr/bin/backlit-settings-daemon --serve"

/usr/bin/backlit-session \
  --backend=headless \
  --socket=backlit-system-install \
  --preflight-only \
  --verify-systemd-units \
  --systemd-unit-dir "$systemd_dir" > "$systemd_units_log"

grep -F '"event":"session.systemd_units_verified"' "$systemd_units_log" >/dev/null || fail "missing systemd unit verification event"
grep -F '"event":"session.systemd_launch_plan"' "$systemd_units_log" >/dev/null || fail "missing systemd launch plan event"
grep -F '"passed":true' "$systemd_units_log" >/dev/null || fail "systemd unit verification did not pass"
grep -F '"session_target_ready":true' "$systemd_units_log" >/dev/null || fail "session target was not ready"

/usr/bin/backlit-session \
  --backend=headless \
  --socket=backlit-system-install \
  --screenshot "$session_screenshot" \
  --verify \
  --verify-launch-spawn \
  --launch-spawn-program true \
  --wayland-display backlit-system-install \
  --verify-services \
  --verify-clean-exit \
  --service-log-dir "$service_log_dir" > "$session_log"

require_file "$session_screenshot"
grep -F '"event":"session.gui_ready"' "$session_log" >/dev/null || fail "missing session gui ready event"
grep -F '"event":"session.verified"' "$session_log" >/dev/null || fail "missing session verified event"
grep -F '"event":"session.services_verified"' "$session_log" >/dev/null || fail "missing session services verification event"
grep -F '"event":"session.clean_exit"' "$session_log" >/dev/null || fail "missing session clean exit event"
grep -F '"passed":true' "$session_log" >/dev/null || fail "session verification did not pass"
grep -F '"golden_ok":true' "$session_log" >/dev/null || fail "session golden verification did not pass"
grep -F '"spawned":true' "$session_log" >/dev/null || fail "session launch target did not spawn"
grep -F '"wayland_display_set":true' "$session_log" >/dev/null || fail "session launch target did not receive WAYLAND_DISPLAY"
grep -F '"compositor_resolved":true' "$session_log" >/dev/null || fail "session compositor binary did not resolve from /usr/bin"
grep -F '"compositor_ready":true' "$session_log" >/dev/null || fail "session compositor service did not become ready"
grep -F '"shell_resolved":true' "$session_log" >/dev/null || fail "session shell binary did not resolve from /usr/bin"
grep -F '"shell_ready":true' "$session_log" >/dev/null || fail "session shell service did not become ready"
grep -F '"notification_resolved":true' "$session_log" >/dev/null || fail "notification daemon did not resolve from /usr/bin"
grep -F '"notification_ready":true' "$session_log" >/dev/null || fail "notification daemon did not become ready"
grep -F '"settings_resolved":true' "$session_log" >/dev/null || fail "settings daemon did not resolve from /usr/bin"
grep -F '"settings_ready":true' "$session_log" >/dev/null || fail "settings daemon did not become ready"
grep -F '"children_exited_cleanly":true' "$session_log" >/dev/null || fail "session service probes did not exit cleanly"
grep -F '"windows_after_shutdown":0' "$session_log" >/dev/null || fail "session did not close all windows"
grep -F '"focus_cleared":true' "$session_log" >/dev/null || fail "session did not clear focus"

BACKLIT_SESSION_BIN=/usr/bin/backlit-session ./scripts/verify-session-replay.sh "$session_replay_dir"
grep -F '"session_replay_event": true' "$session_replay_dir/manifest.json" >/dev/null || fail "missing system-installed session replay event"
grep -F '"frame_count": 9' "$session_replay_dir/manifest.json" >/dev/null || fail "system-installed session replay frame count mismatch"
grep -F '"launcher_overlay_frame": true' "$session_replay_dir/manifest.json" >/dev/null || fail "system-installed session replay launcher overlay missing"
grep -F '"app_switcher_overlay_frame": true' "$session_replay_dir/manifest.json" >/dev/null || fail "system-installed session replay app switcher overlay missing"

/usr/bin/backlit-settings --verify > "$settings_app_log"
grep -F '"event":"settings_app.verified"' "$settings_app_log" >/dev/null || fail "missing settings app verification event"
grep -F '"passed":true' "$settings_app_log" >/dev/null || fail "settings app verification did not pass"
grep -F '"launcher_target_ready":true' "$settings_app_log" >/dev/null || fail "settings launcher target did not verify"

cleanup_packages
installed_packages=false
trap - EXIT INT TERM

for package in $purge_packages; do
  require_status_not_installed "$package"
done

cat > "$manifest" <<EOF
{
  "name": "backlit-debian-system-install",
  "passed": true,
  "system_install_checked": true,
  "system_install_performed": true,
  "install_blocked_expected": false,
  "artifacts": {
    "package_build_manifest": "$build_manifest",
    "dpkg_install_log": "$dpkg_install_log",
    "dpkg_purge_log": "$dpkg_purge_log",
    "session_desktop": "$session_desktop",
    "settings_desktop": "$settings_desktop",
    "systemd_units_log": "$systemd_units_log",
    "session_log": "$session_log",
    "session_screenshot": "$session_screenshot",
    "session_replay_manifest": "$session_replay_dir/manifest.json",
    "session_replay_frames": "$session_replay_dir/frames",
    "service_log_dir": "$service_log_dir",
    "settings_app_log": "$settings_app_log"
  },
  "checks": {
    "system_install_checked": true,
    "install_blocked_expected": false,
    "actual_system_dpkg_install": true,
    "dpkg_database_status": true,
    "usr_bin_session_launch": true,
    "systemd_units_from_system_install": true,
    "session_gui_from_system_install": true,
    "session_services_from_system_install": true,
    "session_replay_from_system_install": true,
    "session_clean_exit_from_system_install": true,
    "settings_app_from_system_install": true,
    "packages_purged_after_verification": true
  }
}
EOF

printf 'Backlit Debian system package install verification passed. Artifacts: %s\n' "$out_dir"
