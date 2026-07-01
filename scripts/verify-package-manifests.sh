#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/package-manifests}"
mkdir -p "$out_dir"

fail() {
  echo "package manifest verification failed: $*" >&2
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

require_absent_file() {
  test ! -e "$1" || fail "unexpected file $1"
}

require_unique_install_paths() {
  duplicates="$(
    sed '/^[[:space:]]*$/d' packaging/debian/*.install \
      | awk '{ print $1 }' \
      | sort \
      | uniq -d
  )"
  test -z "$duplicates" || fail "duplicate install paths: $duplicates"
}

for manifest in \
  packaging/debian/fastgui-compositor.install \
  packaging/debian/fastgui-shell.install \
  packaging/debian/fastgui-session.install \
  packaging/debian/fastgui-settings.install \
  packaging/debian/fastgui-portal.install \
  packaging/debian/fastgui-dev-tools.install
do
  require_file "$manifest"
done

require_absent_file packaging/debian/fastgui-core.install
require_absent_file packaging/debian/fastgui-desktop.install

require_contains packaging/debian/control.stub "Package: fastgui-core"
require_contains packaging/debian/control.stub "Depends: \${misc:Depends}, fastgui-session"
require_contains packaging/debian/control.stub "Depends: \${misc:Depends}, fastgui-compositor, fastgui-shell, fastgui-settings"
require_contains packaging/debian/control.stub "Depends: \${misc:Depends}, fastgui-core, fastgui-portal"
require_contains packaging/debian/control.stub "Depends: \${misc:Depends}, fastgui-core"

require_line packaging/debian/fastgui-compositor.install "usr/bin/backlit-compositor"

require_line packaging/debian/fastgui-shell.install "usr/bin/backlit-shell"
require_line packaging/debian/fastgui-shell.install "usr/bin/backlit-notification-daemon"

require_line packaging/debian/fastgui-session.install "usr/bin/backlit-session"
require_line packaging/debian/fastgui-session.install "usr/bin/backlit-demo-client"
require_line packaging/debian/fastgui-session.install "usr/share/wayland-sessions/backlit.desktop"
require_line packaging/debian/fastgui-session.install "usr/lib/systemd/user/backlit-session.target"
require_line packaging/debian/fastgui-session.install "usr/lib/systemd/user/backlit-compositor.service"
require_line packaging/debian/fastgui-session.install "usr/lib/systemd/user/backlit-shell.service"
require_line packaging/debian/fastgui-session.install "usr/lib/systemd/user/backlit-notification-daemon.service"
require_line packaging/debian/fastgui-session.install "usr/lib/systemd/user/backlit-settings-daemon.service"

require_line packaging/debian/fastgui-settings.install "usr/bin/backlit-settings"
require_line packaging/debian/fastgui-settings.install "usr/bin/backlit-settings-daemon"
require_line packaging/debian/fastgui-settings.install "usr/share/applications/org.backlit.Settings.desktop"

require_line packaging/debian/fastgui-portal.install "usr/bin/backlit-portal-backend"

require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-gui-smoke.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-compositor-runtime.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-smithay-compositor-runtime.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-compositor-socket.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-package-manifests.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-debian-package-build.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-debian-package-install.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-debian-system-install.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-launch-performance.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-launch-readiness.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-session-launch.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-session-replay.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-drm-session-smoke.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-drm-master-boundary.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-dedicated-drm-session.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-parallels-dedicated-drm-e2e.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-mvp1-contract.sh"
require_line packaging/debian/fastgui-dev-tools.install "usr/lib/backlit/dev-tools/verify-linux-e2e.sh"

require_unique_install_paths

manifest_count="$(find packaging/debian -name '*.install' -type f | wc -l | tr -d ' ')"
package_count="$(grep -c '^Package: ' packaging/debian/control.stub)"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-package-manifests",
  "passed": true,
  "package_count": $package_count,
  "install_manifest_count": $manifest_count,
  "artifacts": {
    "control_stub": "packaging/debian/control.stub",
    "compositor_install": "packaging/debian/fastgui-compositor.install",
    "shell_install": "packaging/debian/fastgui-shell.install",
    "session_install": "packaging/debian/fastgui-session.install",
    "settings_install": "packaging/debian/fastgui-settings.install",
    "portal_install": "packaging/debian/fastgui-portal.install",
    "dev_tools_install": "packaging/debian/fastgui-dev-tools.install"
  },
  "checks": {
    "fastgui_core_package": true,
    "session_depends_on_settings_service": true,
    "desktop_depends_on_core": true,
    "core_is_meta_package": true,
    "desktop_is_meta_package": true,
    "session_installs_desktop_entry": true,
    "session_installs_systemd_units": true,
    "runtime_binaries_split": true,
    "dev_tools_manifest": true,
    "unique_install_paths": true
  }
}
EOF

printf 'Backlit package manifest verification passed. Artifacts: %s\n' "$out_dir"
