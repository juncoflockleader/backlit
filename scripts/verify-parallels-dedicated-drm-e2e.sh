#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
credential_file="${BACKLIT_PARALLELS_CREDENTIAL_FILE:-$repo_root/.local/parallels-ubuntu.env}"
vm_name="${BACKLIT_PARALLELS_VM:-Ubuntu 22.04.2 ARM64}"
repo_url="${BACKLIT_E2E_REPO_URL:-https://github.com/juncoflockleader/backlit.git}"
branch="${BACKLIT_E2E_BRANCH:-main}"
host_out_dir="${1:-${BACKLIT_PARALLELS_DEDICATED_DRM_HOST_OUT_DIR:-target/parallels-dedicated-drm-e2e}}"
guest_out_dir="${BACKLIT_PARALLELS_DEDICATED_DRM_GUEST_OUT_DIR:-target/parallels-dedicated-drm-e2e}"
dedicated_tty="${BACKLIT_PARALLELS_DEDICATED_TTY:-/dev/tty3}"

prlctl_bin="${PRLCTL:-}"
if [ -z "$prlctl_bin" ]; then
  if command -v prlctl >/dev/null 2>&1; then
    prlctl_bin="$(command -v prlctl)"
  elif [ -x /usr/local/bin/prlctl ]; then
    prlctl_bin="/usr/local/bin/prlctl"
  else
    echo "prlctl not found. Install or initialize Parallels Desktop first." >&2
    exit 2
  fi
fi

if [ ! -r "$credential_file" ]; then
  cat >&2 <<EOF
Missing credential file: $credential_file

Create it locally, keep it out of Git, and set at least:
  BACKLIT_PARALLELS_UBUNTU_USER=<guest-admin-user>
  BACKLIT_PARALLELS_UBUNTU_PASSWORD=<guest-password>
EOF
  exit 2
fi

set -a
source "$credential_file"
set +a

guest_user="${BACKLIT_PARALLELS_UBUNTU_USER:?BACKLIT_PARALLELS_UBUNTU_USER is required}"
repo_dir="${BACKLIT_E2E_REPO_DIR:-/home/$guest_user/backlit-e2e}"

quote_shell() {
  local value="$1"
  printf "'%s'" "${value//\'/\'\\\'\'}"
}

base64_one_line() {
  local path="$1"
  if base64 --help 2>&1 | grep -q -- '-w'; then
    base64 -w 0 "$path"
  else
    base64 -i "$path" | tr -d '\n'
  fi
}

upload_script() {
  local local_path="$1"
  local remote_path="$2"
  local payload_file chunk_file chunk remote_payload_path chunk_prefix
  payload_file="$tmp_dir/$(basename "$local_path").b64"
  remote_payload_path="$remote_path.b64"
  chunk_prefix="$tmp_dir/$(basename "$local_path").chunk."
  base64_one_line "$local_path" > "$payload_file"
  split -b 3000 "$payload_file" "$chunk_prefix"

  "$prlctl_bin" exec "$vm_name" --user root python3 -c \
    "\"import pathlib; pathlib.Path(\\\"$remote_payload_path\\\").write_text(\\\"\\\")\""

  for chunk_file in "$chunk_prefix"*; do
    chunk="$(cat "$chunk_file")"
    "$prlctl_bin" exec "$vm_name" --user root python3 -c \
      "\"import pathlib; pathlib.Path(\\\"$remote_payload_path\\\").open(\\\"a\\\").write(\\\"$chunk\\\")\""
  done

  "$prlctl_bin" exec "$vm_name" --user root python3 -c \
    "\"import base64,os,pathlib; src=pathlib.Path(\\\"$remote_payload_path\\\"); dst=pathlib.Path(\\\"$remote_path\\\"); data=base64.b64decode(src.read_text()); assert data, \\\"empty upload\\\"; dst.write_bytes(data); os.chmod(dst,0o755); src.unlink()\""
}

download_file() {
  local remote_path="$1"
  local local_path="$2"
  mkdir -p "$(dirname "$local_path")"
  "$prlctl_bin" exec "$vm_name" --user "$guest_user" cat "$remote_path" > "$local_path"
}

require_contains() {
  local file="$1"
  local value="$2"
  grep -F -- "$value" "$file" >/dev/null || {
    echo "Parallels dedicated DRM E2E export verification failed: missing text in $file: $value" >&2
    exit 1
  }
}

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/backlit-parallels-dedicated-drm.XXXXXX")"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

tty_number="${dedicated_tty##*/tty}"
if [ -z "$tty_number" ] || [ "$tty_number" = "$dedicated_tty" ]; then
  echo "BACKLIT_PARALLELS_DEDICATED_TTY must look like /dev/tty3, got: $dedicated_tty" >&2
  exit 2
fi

root_runner="$tmp_dir/backlit-parallels-dedicated-drm-root-runner.sh"
cat > "$root_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail

guest_user=$(quote_shell "$guest_user")
repo_url=$(quote_shell "$repo_url")
repo_dir=$(quote_shell "$repo_dir")
branch=$(quote_shell "$branch")
guest_out_dir=$(quote_shell "$guest_out_dir")
dedicated_tty=$(quote_shell "$dedicated_tty")
tty_number=$(quote_shell "$tty_number")
unit_name="backlit-dedicated-drm-e2e-\$\$"
unit_log="/tmp/backlit-dedicated-drm-e2e.log"
inner_runner="/tmp/backlit-dedicated-drm-e2e-inner.sh"

export DEBIAN_FRONTEND=noninteractive

retry_guest_command() {
  local attempt=1
  local max_attempts=5
  local delay=2

  while true; do
    if "\$@"; then
      return 0
    fi

    local status="\$?"
    if [ "\$attempt" -ge "\$max_attempts" ]; then
      return "\$status"
    fi

    printf 'Guest command failed (attempt %s/%s); retrying in %ss: %s\n' \
      "\$attempt" "\$max_attempts" "\$delay" "\$*" >&2
    sleep "\$delay"
    attempt="\$((attempt + 1))"
    delay="\$((delay * 2))"
  done
}

apt-get update
apt-get install -y git ca-certificates build-essential pkg-config curl python3

if [ -e "\$repo_dir" ] && [ ! -d "\$repo_dir/.git" ]; then
  echo "Refusing to use non-Git path: \$repo_dir" >&2
  exit 2
fi

mkdir -p "\$(dirname "\$repo_dir")"
chown "\$guest_user:\$guest_user" "\$(dirname "\$repo_dir")"

if [ -d "\$repo_dir/.git" ]; then
  retry_guest_command runuser -u "\$guest_user" -- git -C "\$repo_dir" fetch origin "\$branch"
  runuser -u "\$guest_user" -- git -C "\$repo_dir" checkout "\$branch"
  runuser -u "\$guest_user" -- git -C "\$repo_dir" reset --hard "origin/\$branch"
else
  retry_guest_command runuser -u "\$guest_user" -- git clone --branch "\$branch" "\$repo_url" "\$repo_dir"
fi

DEBIAN_FRONTEND=noninteractive "\$repo_dir/scripts/bootstrap-ubuntu.sh"

runuser -u "\$guest_user" -- bash -lc '
set -euo pipefail
if [ ! -x "\$HOME/.cargo/bin/rustup" ]; then
  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable
fi
source "\$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
'

retry_guest_command runuser -u "\$guest_user" -- git -C "\$repo_dir" fetch origin "\$branch"
runuser -u "\$guest_user" -- git -C "\$repo_dir" checkout "\$branch"
runuser -u "\$guest_user" -- git -C "\$repo_dir" reset --hard "origin/\$branch"

package_build_dir="\$guest_out_dir/package-build"
debs_dir="\$repo_dir/\$package_build_dir/debs"
dpkg_install_log="\$repo_dir/\$guest_out_dir/system-dpkg-install.log"
dpkg_purge_log="\$repo_dir/\$guest_out_dir/system-dpkg-purge.log"
packages="fastgui-compositor fastgui-shell fastgui-settings fastgui-session fastgui-core"
purge_packages="fastgui-dev-tools fastgui-desktop fastgui-core fastgui-portal fastgui-session fastgui-settings fastgui-shell fastgui-compositor"
installed_packages=false

cleanup_packages() {
  if [ "\$installed_packages" = true ]; then
    dpkg --purge \$purge_packages >> "\$dpkg_purge_log" 2>&1 || true
  fi
}

mkdir -p "\$repo_dir/\$guest_out_dir"
chown -R "\$guest_user:\$guest_user" "\$repo_dir/\$guest_out_dir"

runuser -u "\$guest_user" -- bash -lc "
set -euo pipefail
source \"\\\$HOME/.cargo/env\"
cd \"\$repo_dir\"
scripts/verify-debian-package-build.sh \"\$package_build_dir\"
"

: > "\$dpkg_install_log"
: > "\$dpkg_purge_log"
dpkg --purge \$purge_packages >> "\$dpkg_purge_log" 2>&1 || true

install_debs=""
for package in \$packages; do
  deb="\$(find "\$debs_dir" -maxdepth 1 -name "\${package}_*.deb" -type f | sort | sed -n '1p')"
  if [ -z "\$deb" ]; then
    echo "Missing deb for \$package in \$debs_dir" >&2
    exit 1
  fi
  install_debs="\$install_debs \$deb"
done

installed_packages=true
dpkg --install \$install_debs > "\$dpkg_install_log" 2>&1

for package in \$packages; do
  status="\$(dpkg-query -W -f='\${Status}' "\$package" 2>/dev/null || true)"
  if [ "\$status" != "install ok installed" ]; then
    echo "Package not installed: \$package (\$status)" >&2
    exit 1
  fi
done

cat > "\$inner_runner" <<INNER
#!/usr/bin/env bash
set -euo pipefail

source "\\\$HOME/.cargo/env"
cd "\$repo_dir"
printf 'session env: id=%s runtime=%s type=%s seat=%s tty=' "\\\${XDG_SESSION_ID-}" "\\\${XDG_RUNTIME_DIR-}" "\\\${XDG_SESSION_TYPE-}" "\\\${XDG_SEAT-}"
tty || true
if [ -n "\\\${XDG_SESSION_ID-}" ]; then
  loginctl show-session "\\\$XDG_SESSION_ID" -p Name -p User -p Seat -p TTY -p Type -p State -p Remote -p Class --no-pager || true
fi

BACKLIT_DEDICATED_DRM_SESSION_BIN=/usr/bin/backlit-session \\
BACKLIT_REQUIRE_DEDICATED_DRM_SESSION=1 \\
BACKLIT_REQUIRE_DRM_MASTER_PRESENT=1 \\
  ./scripts/verify-dedicated-drm-session.sh "\$guest_out_dir"
INNER
chmod 0755 "\$inner_runner"
chown "\$guest_user:\$guest_user" "\$inner_runner"

rm -f "\$unit_log"
before_tty="\$(fgconsole 2>/dev/null || printf 2)"
restore_tty() {
  chvt "\$before_tty" 2>/dev/null || chvt 2 2>/dev/null || true
}

cleanup_run() {
  restore_tty
  cleanup_packages
}
trap cleanup_run EXIT

chvt "\$tty_number" 2>/dev/null || true

set +e
systemd-run --unit="\$unit_name" --wait --collect \
  --property=PAMName=login \
  --property=TTYPath="\$dedicated_tty" \
  --property=StandardInput=tty-force \
  --property=StandardOutput=file:"\$unit_log" \
  --property=StandardError=append:"\$unit_log" \
  --property=RuntimeMaxSec=300 \
  --uid="\$guest_user" \
  "\$inner_runner"
status="\$?"
set -e

restore_tty
cleanup_packages
installed_packages=false
trap - EXIT
chown "\$guest_user:\$guest_user" "\$unit_log" "\$dpkg_install_log" "\$dpkg_purge_log" 2>/dev/null || true
if [ -d "\$repo_dir/\$guest_out_dir" ]; then
  chown -R "\$guest_user:\$guest_user" "\$repo_dir/\$guest_out_dir"
fi
cat "\$unit_log" 2>/dev/null || true
exit "\$status"
EOF
chmod 700 "$root_runner"

printf 'Using Parallels VM: %s\n' "$vm_name"
"$prlctl_bin" list --all | grep -F "$vm_name" >/dev/null

upload_script "$root_runner" "/tmp/backlit-parallels-dedicated-drm-root-runner.sh"
"$prlctl_bin" exec "$vm_name" --user root /tmp/backlit-parallels-dedicated-drm-root-runner.sh

guest_commit="$("$prlctl_bin" exec "$vm_name" --user "$guest_user" git -C "$repo_dir" rev-parse --short HEAD | tr -d '\r')"
guest_dedicated_dir="$repo_dir/$guest_out_dir"

host_manifest="$host_out_dir/dedicated-drm-session-manifest.json"
host_boundary_manifest="$host_out_dir/drm-master-boundary-manifest.json"
host_session_log="$host_out_dir/session.jsonl"
host_session_stderr="$host_out_dir/session.stderr"
host_compositor_log="$host_out_dir/compositor-service.jsonl"
host_compositor_stderr="$host_out_dir/compositor-service.stderr"
host_runner_log="$host_out_dir/runner.log"
host_package_build_manifest="$host_out_dir/package-build-manifest.json"
host_dpkg_install_log="$host_out_dir/system-dpkg-install.log"
host_dpkg_purge_log="$host_out_dir/system-dpkg-purge.log"
host_ppm="$host_out_dir/dedicated-session.ppm"
host_png="$host_out_dir/dedicated-session.png"

mkdir -p "$host_out_dir"
rm -f \
  "$host_manifest" \
  "$host_boundary_manifest" \
  "$host_session_log" \
  "$host_session_stderr" \
  "$host_compositor_log" \
  "$host_compositor_stderr" \
  "$host_runner_log" \
  "$host_package_build_manifest" \
  "$host_dpkg_install_log" \
  "$host_dpkg_purge_log" \
  "$host_ppm" \
  "$host_png" \
  "$host_out_dir/manifest.json"

download_file "$guest_dedicated_dir/manifest.json" "$host_manifest"
download_file "$guest_dedicated_dir/drm-master-boundary/manifest.json" "$host_boundary_manifest"
download_file "$guest_dedicated_dir/session.jsonl" "$host_session_log"
download_file "$guest_dedicated_dir/session.stderr" "$host_session_stderr"
download_file "$guest_dedicated_dir/session-services/compositor.jsonl" "$host_compositor_log"
download_file "$guest_dedicated_dir/session-services/compositor.stderr" "$host_compositor_stderr"
download_file "/tmp/backlit-dedicated-drm-e2e.log" "$host_runner_log"
download_file "$guest_dedicated_dir/package-build/manifest.json" "$host_package_build_manifest"
download_file "$guest_dedicated_dir/system-dpkg-install.log" "$host_dpkg_install_log"
download_file "$guest_dedicated_dir/system-dpkg-purge.log" "$host_dpkg_purge_log"
download_file "$guest_dedicated_dir/dedicated-session.ppm" "$host_ppm"

preview_image="$host_ppm"
preview_format="ppm"
png_written=false
converter="none"

if command -v sips >/dev/null 2>&1; then
  if sips -s format png "$host_ppm" --out "$host_png" >/dev/null 2>&1; then
    preview_image="$host_png"
    preview_format="png"
    png_written=true
    converter="sips"
  fi
elif command -v magick >/dev/null 2>&1; then
  if magick "$host_ppm" "$host_png" >/dev/null 2>&1; then
    preview_image="$host_png"
    preview_format="png"
    png_written=true
    converter="magick"
  fi
elif command -v convert >/dev/null 2>&1; then
  if convert "$host_ppm" "$host_png" >/dev/null 2>&1; then
    preview_image="$host_png"
    preview_format="png"
    png_written=true
    converter="convert"
  fi
fi

require_contains "$host_manifest" '"passed": true'
require_contains "$host_manifest" '"expected_blocked": false'
require_contains "$host_manifest" '"reason": "dedicated-drm-session-presented"'
require_contains "$host_manifest" '"dedicated_session_acceptance": true'
require_contains "$host_manifest" '"current_session_can_present": true'
require_contains "$host_manifest" '"first_present_commit_succeeded": true'
require_contains "$host_manifest" '"first_present_vblank_event_received": true'
require_contains "$host_manifest" '"session_drm_first_present_probe": true'
require_contains "$host_manifest" '"session_binary": "/usr/bin/backlit-session"'
require_contains "$host_manifest" '"system_session_binary": true'
require_contains "$host_manifest" '"session_gui_verified": true'
require_contains "$host_manifest" '"session_services": true'
require_contains "$host_manifest" '"session_desktop_launch": true'
require_contains "$host_manifest" '"session_compositor_demo_client": true'
require_contains "$host_manifest" '"session_clean_exit": true'
require_contains "$host_package_build_manifest" '"debs_built": true'
require_contains "$host_session_log" '"implementation":"smithay-compositor-runtime"'
require_contains "$host_session_log" '"kms_first_present_commit_succeeded":true'
require_contains "$host_session_log" '"kms_first_present_vblank_event_received":true'
require_contains "$host_session_log" '"compositor_smithay_runtime":true'
require_contains "$host_session_log" '"compositor_smithay_protocol_globals":true'
require_contains "$host_session_log" '"children_exited_cleanly":true'

ppm_bytes="$(wc -c < "$host_ppm" | tr -d ' ')"

cat > "$host_out_dir/manifest.json" <<EOF
{
  "name": "backlit-parallels-dedicated-drm-e2e-export",
  "passed": true,
  "vm": "$vm_name",
  "guest_commit": "$guest_commit",
  "guest_repo": "$repo_dir",
  "guest_dedicated_dir": "$guest_out_dir",
  "dedicated_tty": "$dedicated_tty",
  "artifacts": {
    "dedicated_drm_session_manifest": "$host_manifest",
    "drm_master_boundary_manifest": "$host_boundary_manifest",
    "session_log": "$host_session_log",
    "session_stderr": "$host_session_stderr",
    "compositor_service_log": "$host_compositor_log",
    "compositor_service_stderr": "$host_compositor_stderr",
    "runner_log": "$host_runner_log",
    "package_build_manifest": "$host_package_build_manifest",
    "dpkg_install_log": "$host_dpkg_install_log",
    "dpkg_purge_log": "$host_dpkg_purge_log",
    "gui_preview_ppm": "$host_ppm",
    "gui_preview_image": "$preview_image"
  },
  "checks": {
    "system_package_dedicated_drm": true,
    "system_session_binary": true,
    "debs_built": true,
    "dedicated_session_acceptance": true,
    "drm_first_present_commit": true,
    "drm_first_present_vblank": true,
    "session_gui_verified": true,
    "session_services": true,
    "session_clean_exit": true,
    "ppm_bytes": $ppm_bytes,
    "png_written": $png_written,
    "preview_format": "$preview_format",
    "converter": "$converter"
  }
}
EOF

printf 'Backlit Parallels dedicated DRM E2E exported: %s\n' "$host_out_dir"
printf 'Manifest: %s\n' "$host_out_dir/manifest.json"
if [ "$png_written" = true ]; then
  printf 'To view dedicated DRM preview on macOS: open %s\n' "$host_png"
else
  printf 'To view dedicated DRM preview on macOS: open %s\n' "$host_ppm"
fi
