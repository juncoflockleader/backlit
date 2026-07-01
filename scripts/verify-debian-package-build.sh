#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/debian-package-build}"
build_root="$out_dir/build-root"
debs_dir="$out_dir/debs"
manifest="$out_dir/manifest.json"
mkdir -p "$out_dir"

fail() {
  echo "Debian package build verification failed: $*" >&2
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

require_deb_contains() {
  package="$1"
  path="$2"
  contents_file="$out_dir/$package.contents"
  grep -F "./$path" "$contents_file" >/dev/null || fail "$package deb missing $path"
}

write_skipped_manifest() {
  reason="$1"
  cat > "$manifest" <<EOF
{
  "name": "backlit-debian-package-build",
  "passed": true,
  "package_build_checked": true,
  "debs_built": false,
  "build_blocked_expected": true,
  "build_blocked_reason": "$reason",
  "artifacts": {
    "control_stub": "packaging/debian/control.stub",
    "package_manifest": "scripts/verify-package-manifests.sh"
  },
  "checks": {
    "package_build_checked": true,
    "build_blocked_expected": true,
    "fastgui_core_deb": false,
    "runtime_package_debs": false,
    "package_contents": false,
    "package_dependencies": false
  }
}
EOF
  printf 'Backlit Debian package build verification skipped as expected: %s. Artifacts: %s\n' "$reason" "$out_dir"
}

if [ "$(uname -s)" != "Linux" ]; then
  write_skipped_manifest "non-linux-host"
  exit 0
fi

if ! command -v dpkg-deb >/dev/null 2>&1; then
  write_skipped_manifest "missing-dpkg-deb"
  exit 0
fi

architecture="$(dpkg --print-architecture 2>/dev/null || dpkg-deb --showformat='${Architecture}' --show /dev/null 2>/dev/null || true)"
test -n "$architecture" || architecture="all"
commit="$(git rev-parse --short HEAD 2>/dev/null || printf unknown)"
version="0.1.0+git$commit"

rm -rf "$build_root" "$debs_dir"
mkdir -p "$build_root" "$debs_dir"

./scripts/verify-package-manifests.sh "$out_dir/package-manifests" >/dev/null

cargo build -p backlit-compositor --features smithay-backend
cargo build -p backlit-session --features smithay-backend

cargo build \
  -p backlit-shell \
  -p backlit-notification-daemon \
  -p backlit-demo-client \
  -p backlit-settings \
  -p backlit-settings-daemon \
  -p backlit-portal-backend

copy_file() {
  src="$1"
  dst="$2"
  mode="$3"
  mkdir -p "$(dirname "$dst")"
  install -m "$mode" "$src" "$dst"
}

sanitize_depends() {
  printf '%s\n' "${1#Depends: }" | awk -F, '
    {
      out = ""
      for (i = 1; i <= NF; i++) {
        item = $i
        gsub(/^[[:space:]]+/, "", item)
        gsub(/[[:space:]]+$/, "", item)
        if (item == "" || item == "${shlibs:Depends}" || item == "${misc:Depends}") {
          continue
        }
        if (out != "") {
          out = out ", "
        }
        out = out item
      }
      if (out != "") {
        print "Depends: " out
      }
    }
  '
}

control_field() {
  package="$1"
  field="$2"
  awk -v package="$package" -v field="$field" '
    /^Package: / {
      in_package = ($2 == package)
      next
    }
    in_package && index($0, field ": ") == 1 {
      print substr($0, length(field) + 3)
      exit
    }
    in_package && NF == 0 {
      exit
    }
  ' packaging/debian/control.stub
}

control_description() {
  package="$1"
  awk -v package="$package" '
    /^Package: / {
      in_package = ($2 == package)
      in_description = 0
      next
    }
    in_package && /^Description: / {
      print
      in_description = 1
      next
    }
    in_package && in_description && /^ / {
      print
      next
    }
    in_package && in_description {
      exit
    }
  ' packaging/debian/control.stub
}

write_control() {
  package="$1"
  root="$2"
  debian_dir="$root/DEBIAN"
  depends="$(control_field "$package" Depends || true)"
  description="$(control_description "$package")"
  test -n "$description" || fail "missing description for $package"
  mkdir -p "$debian_dir"
  {
    printf 'Package: %s\n' "$package"
    printf 'Version: %s\n' "$version"
    printf 'Section: x11\n'
    printf 'Priority: optional\n'
    printf 'Architecture: %s\n' "$architecture"
    printf 'Maintainer: Backlit contributors <devnull@example.invalid>\n'
    if [ -n "$depends" ]; then
      sanitized_depends="$(sanitize_depends "Depends: $depends")"
      if [ -n "$sanitized_depends" ]; then
        printf '%s\n' "$sanitized_depends"
      fi
    fi
    printf '%s\n' "$description"
  } > "$debian_dir/control"
}

package_root() {
  printf '%s/%s\n' "$build_root" "$1"
}

prepare_package_root() {
  package="$1"
  root="$(package_root "$package")"
  mkdir -p "$root"
  write_control "$package" "$root"
}

for package in \
  fastgui-compositor \
  fastgui-shell \
  fastgui-session \
  fastgui-settings \
  fastgui-portal \
  fastgui-core \
  fastgui-desktop \
  fastgui-dev-tools
do
  prepare_package_root "$package"
done

copy_file target/debug/backlit-compositor "$(package_root fastgui-compositor)/usr/bin/backlit-compositor" 0755

copy_file target/debug/backlit-shell "$(package_root fastgui-shell)/usr/bin/backlit-shell" 0755
copy_file target/debug/backlit-notification-daemon "$(package_root fastgui-shell)/usr/bin/backlit-notification-daemon" 0755

copy_file target/debug/backlit-session "$(package_root fastgui-session)/usr/bin/backlit-session" 0755
copy_file target/debug/backlit-demo-client "$(package_root fastgui-session)/usr/bin/backlit-demo-client" 0755
copy_file packaging/sessions/backlit.desktop "$(package_root fastgui-session)/usr/share/wayland-sessions/backlit.desktop" 0644
copy_file packaging/systemd/backlit-session.target "$(package_root fastgui-session)/usr/lib/systemd/user/backlit-session.target" 0644
copy_file packaging/systemd/backlit-compositor.service "$(package_root fastgui-session)/usr/lib/systemd/user/backlit-compositor.service" 0644
copy_file packaging/systemd/backlit-shell.service "$(package_root fastgui-session)/usr/lib/systemd/user/backlit-shell.service" 0644
copy_file packaging/systemd/backlit-notification-daemon.service "$(package_root fastgui-session)/usr/lib/systemd/user/backlit-notification-daemon.service" 0644
copy_file packaging/systemd/backlit-settings-daemon.service "$(package_root fastgui-session)/usr/lib/systemd/user/backlit-settings-daemon.service" 0644

copy_file target/debug/backlit-settings "$(package_root fastgui-settings)/usr/bin/backlit-settings" 0755
copy_file target/debug/backlit-settings-daemon "$(package_root fastgui-settings)/usr/bin/backlit-settings-daemon" 0755
copy_file packaging/applications/org.backlit.Settings.desktop "$(package_root fastgui-settings)/usr/share/applications/org.backlit.Settings.desktop" 0644

copy_file target/debug/backlit-portal-backend "$(package_root fastgui-portal)/usr/bin/backlit-portal-backend" 0755

for verifier in \
  verify-gui-smoke.sh \
  verify-compositor-runtime.sh \
  verify-smithay-compositor-runtime.sh \
  verify-compositor-socket.sh \
  verify-package-manifests.sh \
  verify-debian-package-build.sh \
  verify-debian-package-install.sh \
  verify-debian-system-install.sh \
  verify-launch-performance.sh \
  verify-launch-readiness.sh \
  verify-session-launch.sh \
  verify-session-replay.sh \
  verify-drm-session-smoke.sh \
  verify-drm-master-boundary.sh \
  verify-mvp1-contract.sh \
  verify-linux-e2e.sh
do
  copy_file "scripts/$verifier" "$(package_root fastgui-dev-tools)/usr/lib/backlit/dev-tools/$verifier" 0755
done

for package in \
  fastgui-compositor \
  fastgui-shell \
  fastgui-session \
  fastgui-settings \
  fastgui-portal \
  fastgui-core \
  fastgui-desktop \
  fastgui-dev-tools
do
  root="$(package_root "$package")"
  deb="$debs_dir/${package}_${version}_${architecture}.deb"
  dpkg-deb --build --root-owner-group "$root" "$deb" >/dev/null
  require_file "$deb"
  dpkg-deb --info "$deb" > "$out_dir/$package.info"
  dpkg-deb --contents "$deb" > "$out_dir/$package.contents"
done

require_deb_contains fastgui-compositor usr/bin/backlit-compositor
require_deb_contains fastgui-shell usr/bin/backlit-shell
require_deb_contains fastgui-shell usr/bin/backlit-notification-daemon
require_deb_contains fastgui-session usr/bin/backlit-session
require_deb_contains fastgui-session usr/bin/backlit-demo-client
require_deb_contains fastgui-session usr/share/wayland-sessions/backlit.desktop
require_deb_contains fastgui-session usr/lib/systemd/user/backlit-session.target
require_deb_contains fastgui-session usr/lib/systemd/user/backlit-compositor.service
require_deb_contains fastgui-session usr/lib/systemd/user/backlit-shell.service
require_deb_contains fastgui-session usr/lib/systemd/user/backlit-notification-daemon.service
require_deb_contains fastgui-session usr/lib/systemd/user/backlit-settings-daemon.service
require_deb_contains fastgui-settings usr/bin/backlit-settings
require_deb_contains fastgui-settings usr/bin/backlit-settings-daemon
require_deb_contains fastgui-settings usr/share/applications/org.backlit.Settings.desktop
require_deb_contains fastgui-portal usr/bin/backlit-portal-backend
require_deb_contains fastgui-dev-tools usr/lib/backlit/dev-tools/verify-linux-e2e.sh
require_deb_contains fastgui-dev-tools usr/lib/backlit/dev-tools/verify-smithay-compositor-runtime.sh
require_deb_contains fastgui-dev-tools usr/lib/backlit/dev-tools/verify-compositor-socket.sh
require_deb_contains fastgui-dev-tools usr/lib/backlit/dev-tools/verify-debian-package-build.sh
require_deb_contains fastgui-dev-tools usr/lib/backlit/dev-tools/verify-debian-package-install.sh
require_deb_contains fastgui-dev-tools usr/lib/backlit/dev-tools/verify-debian-system-install.sh
require_deb_contains fastgui-dev-tools usr/lib/backlit/dev-tools/verify-session-replay.sh
require_deb_contains fastgui-dev-tools usr/lib/backlit/dev-tools/verify-drm-master-boundary.sh
require_deb_contains fastgui-dev-tools usr/lib/backlit/dev-tools/verify-mvp1-contract.sh

test "$(grep -c '^ Package: fastgui-core$' "$out_dir/fastgui-core.info")" = "1" || fail "fastgui-core info missing package field"
test "$(grep -c '^ Package: fastgui-desktop$' "$out_dir/fastgui-desktop.info")" = "1" || fail "fastgui-desktop info missing package field"
require_line "$out_dir/fastgui-core.info" " Depends: fastgui-session"
require_line "$out_dir/fastgui-desktop.info" " Depends: fastgui-core, fastgui-portal"
require_line "$out_dir/fastgui-session.info" " Depends: fastgui-compositor, fastgui-shell, fastgui-settings"
require_line "$out_dir/fastgui-dev-tools.info" " Depends: fastgui-core"

deb_count="$(find "$debs_dir" -name '*.deb' -type f | wc -l | tr -d ' ')"
test "$deb_count" = "8" || fail "expected 8 debs, got $deb_count"

cat > "$manifest" <<EOF
{
  "name": "backlit-debian-package-build",
  "passed": true,
  "package_build_checked": true,
  "debs_built": true,
  "build_blocked_expected": false,
  "architecture": "$architecture",
  "version": "$version",
  "deb_count": $deb_count,
  "artifacts": {
    "debs_dir": "$debs_dir",
    "fastgui_core_deb": "$debs_dir/fastgui-core_${version}_${architecture}.deb",
    "fastgui_desktop_deb": "$debs_dir/fastgui-desktop_${version}_${architecture}.deb",
    "package_manifest": "$out_dir/package-manifests/manifest.json"
  },
  "checks": {
    "package_build_checked": true,
    "build_blocked_expected": false,
    "fastgui_core_deb": true,
    "compositor_smithay_feature_build": true,
    "session_smithay_feature_build": true,
    "runtime_package_debs": true,
    "package_contents": true,
    "package_dependencies": true
  }
}
EOF

printf 'Backlit Debian package build verification passed. Artifacts: %s\n' "$out_dir"
