#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/debian-package-install}"
package_build_dir="$out_dir/package-build"
install_root="$out_dir/install-root"
dpkg_status_file="$install_root/var/lib/dpkg/status"
dpkg_install_log="$out_dir/dpkg-install.log"
session_log="$out_dir/session.jsonl"
systemd_units_log="$out_dir/systemd-units.jsonl"
settings_app_log="$out_dir/settings-app.jsonl"
session_screenshot="$out_dir/deb-installed-session.ppm"
service_log_dir="$out_dir/session-services"
manifest="$out_dir/manifest.json"
mkdir -p "$out_dir"

fail() {
  echo "Debian package install verification failed: $*" >&2
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
  "name": "backlit-debian-package-install",
  "passed": true,
  "package_install_checked": true,
  "debs_extracted": false,
  "debs_installed": false,
  "install_blocked_expected": true,
  "install_blocked_reason": "$reason",
  "artifacts": {
    "package_build_manifest": "$package_build_dir/manifest.json"
  },
  "checks": {
    "package_install_checked": true,
    "install_blocked_expected": true,
    "dpkg_root_install": false,
    "fastgui_core_closure": false,
    "session_exec_from_extracted_debs": false,
    "session_systemd_units_from_extracted_debs": false,
    "session_gui_from_extracted_debs": false,
    "session_services_from_extracted_debs": false,
    "session_clean_exit_from_extracted_debs": false,
    "settings_app_from_extracted_debs": false
  }
}
EOF
  printf 'Backlit Debian package install verification skipped as expected: %s. Artifacts: %s\n' "$reason" "$out_dir"
}

if [ "$(uname -s)" != "Linux" ]; then
  write_skipped_manifest "non-linux-host"
  exit 0
fi

if ! command -v dpkg-deb >/dev/null 2>&1 || ! command -v dpkg >/dev/null 2>&1; then
  write_skipped_manifest "missing-dpkg-tools"
  exit 0
fi

./scripts/verify-debian-package-build.sh "$package_build_dir" >/dev/null

build_manifest="$package_build_dir/manifest.json"
require_file "$build_manifest"
if ! grep '"debs_built": true' "$build_manifest" >/dev/null; then
  write_skipped_manifest "package-build-blocked"
  exit 0
fi

debs_dir="$package_build_dir/debs"

deb_for() {
  package="$1"
  match="$(find "$debs_dir" -maxdepth 1 -name "${package}_*.deb" -type f | sort | sed -n '1p')"
  test -n "$match" || fail "missing deb for $package"
  printf '%s\n' "$match"
}

rm -rf "$install_root"
mkdir -p "$install_root/var/lib/dpkg" \
  "$install_root/var/lib/dpkg/info" \
  "$install_root/var/lib/dpkg/updates" \
  "$install_root/var/lib/dpkg/triggers"
touch "$dpkg_status_file"

install_debs=""
for package in \
  fastgui-compositor \
  fastgui-shell \
  fastgui-settings \
  fastgui-session \
  fastgui-core
do
  deb="$(deb_for "$package")"
  dpkg-deb -f "$deb" > "$out_dir/$package.fields"
  install_debs="$install_debs $deb"
done

dpkg --force-not-root --root="$install_root" --install $install_debs > "$dpkg_install_log" 2>&1

require_status_installed() {
  package="$1"
  awk -v package="$package" '
    /^Package: / {
      in_package = ($2 == package)
      status_ok = 0
      next
    }
    in_package && /^Status: install ok installed$/ {
      status_ok = 1
      next
    }
    in_package && NF == 0 {
      if (status_ok) {
        found = 1
      }
      in_package = 0
    }
    END {
      if (in_package && status_ok) {
        found = 1
      }
      exit(found ? 0 : 1)
    }
  ' "$dpkg_status_file" || fail "dpkg status missing installed package $package"
}

for package in \
  fastgui-compositor \
  fastgui-shell \
  fastgui-settings \
  fastgui-session \
  fastgui-core
do
  require_status_installed "$package"
done

bin_dir="$install_root/usr/bin"
session_desktop="$install_root/usr/share/wayland-sessions/backlit.desktop"
settings_desktop="$install_root/usr/share/applications/org.backlit.Settings.desktop"
systemd_dir="$install_root/usr/lib/systemd/user"

require_executable "$bin_dir/backlit-compositor"
require_executable "$bin_dir/backlit-shell"
require_executable "$bin_dir/backlit-notification-daemon"
require_executable "$bin_dir/backlit-session"
require_executable "$bin_dir/backlit-settings"
require_executable "$bin_dir/backlit-settings-daemon"

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

"$bin_dir/backlit-session" \
  --backend=headless \
  --socket=backlit-deb-install \
  --preflight-only \
  --verify-systemd-units \
  --systemd-unit-dir "$systemd_dir" > "$systemd_units_log"

grep -F '"event":"session.systemd_units_verified"' "$systemd_units_log" >/dev/null || fail "missing systemd unit verification event"
grep -F '"event":"session.systemd_launch_plan"' "$systemd_units_log" >/dev/null || fail "missing systemd launch plan event"
grep -F '"passed":true' "$systemd_units_log" >/dev/null || fail "systemd unit verification did not pass"
grep -F '"session_target_ready":true' "$systemd_units_log" >/dev/null || fail "session target was not ready"
grep -F '"service_units":4' "$systemd_units_log" >/dev/null || fail "systemd plan did not include all services"

"$bin_dir/backlit-session" \
  --backend=headless \
  --socket=backlit-deb-install \
  --screenshot "$session_screenshot" \
  --verify \
  --verify-launch-spawn \
  --launch-spawn-program true \
  --wayland-display backlit-deb-install \
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
grep -F '"compositor_resolved":true' "$session_log" >/dev/null || fail "session compositor binary did not resolve from extracted debs"
grep -F '"compositor_ready":true' "$session_log" >/dev/null || fail "session compositor service did not become ready"
grep -F '"shell_resolved":true' "$session_log" >/dev/null || fail "session shell binary did not resolve from extracted debs"
grep -F '"shell_ready":true' "$session_log" >/dev/null || fail "session shell service did not become ready"
grep -F '"notification_resolved":true' "$session_log" >/dev/null || fail "notification daemon did not resolve from extracted debs"
grep -F '"notification_ready":true' "$session_log" >/dev/null || fail "notification daemon did not become ready"
grep -F '"settings_resolved":true' "$session_log" >/dev/null || fail "settings daemon did not resolve from extracted debs"
grep -F '"settings_ready":true' "$session_log" >/dev/null || fail "settings daemon did not become ready"
grep -F '"children_exited_cleanly":true' "$session_log" >/dev/null || fail "session service probes did not exit cleanly"
grep -F '"windows_after_shutdown":0' "$session_log" >/dev/null || fail "session did not close all windows"
grep -F '"focus_cleared":true' "$session_log" >/dev/null || fail "session did not clear focus"

"$bin_dir/backlit-settings" --verify > "$settings_app_log"
grep -F '"event":"settings_app.verified"' "$settings_app_log" >/dev/null || fail "missing settings app verification event"
grep -F '"passed":true' "$settings_app_log" >/dev/null || fail "settings app verification did not pass"
grep -F '"launcher_target_ready":true' "$settings_app_log" >/dev/null || fail "settings launcher target did not verify"

cat > "$manifest" <<EOF
{
  "name": "backlit-debian-package-install",
  "passed": true,
  "package_install_checked": true,
  "debs_extracted": true,
  "debs_installed": true,
  "install_blocked_expected": false,
  "install_root": "$install_root",
  "artifacts": {
    "package_build_manifest": "$build_manifest",
    "dpkg_install_log": "$dpkg_install_log",
    "dpkg_status_file": "$dpkg_status_file",
    "session_desktop": "$session_desktop",
    "settings_desktop": "$settings_desktop",
    "systemd_units_log": "$systemd_units_log",
    "session_log": "$session_log",
    "session_screenshot": "$session_screenshot",
    "service_log_dir": "$service_log_dir",
    "settings_app_log": "$settings_app_log"
  },
  "checks": {
    "package_install_checked": true,
    "install_blocked_expected": false,
    "dpkg_root_install": true,
    "fastgui_core_closure": true,
    "session_exec_from_extracted_debs": true,
    "session_systemd_units_from_extracted_debs": true,
    "session_gui_from_extracted_debs": true,
    "session_services_from_extracted_debs": true,
    "session_clean_exit_from_extracted_debs": true,
    "settings_app_from_extracted_debs": true
  }
}
EOF

printf 'Backlit Debian package install verification passed. Artifacts: %s\n' "$out_dir"
