#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
credential_file="${BACKLIT_PARALLELS_CREDENTIAL_FILE:-$repo_root/.local/parallels-ubuntu.env}"
vm_name="${BACKLIT_PARALLELS_VM:-Ubuntu 22.04.2 ARM64}"
repo_url="${BACKLIT_E2E_REPO_URL:-https://github.com/juncoflockleader/backlit.git}"
branch="${BACKLIT_E2E_BRANCH:-main}"
host_out_dir="${1:-${BACKLIT_PARALLELS_E2E_HOST_OUT_DIR:-target/linux-e2e-parallels}}"
e2e_out_dir="${BACKLIT_E2E_OUT_DIR:-$host_out_dir}"

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
    echo "Parallels E2E export verification failed: missing text in $file: $value" >&2
    exit 1
  }
}

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/backlit-parallels-e2e.XXXXXX")"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

root_runner="$tmp_dir/backlit-parallels-root-runner.sh"
cat > "$root_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail

guest_user=$(quote_shell "$guest_user")
repo_url=$(quote_shell "$repo_url")
repo_dir=$(quote_shell "$repo_dir")
branch=$(quote_shell "$branch")
e2e_out_dir=$(quote_shell "$e2e_out_dir")
uploaded_verifier="/tmp/backlit-verify-linux-e2e.sh"
uploaded_gui_smoke_verifier="/tmp/backlit-verify-gui-smoke.sh"
uploaded_gui_preview_renderer="/tmp/backlit-render-gui-preview.sh"
uploaded_compositor_runtime_verifier="/tmp/backlit-verify-compositor-runtime.sh"
uploaded_compositor_socket_verifier="/tmp/backlit-verify-compositor-socket.sh"
uploaded_launch_performance_verifier="/tmp/backlit-verify-launch-performance.sh"
uploaded_ci_contract_verifier="/tmp/backlit-verify-ci-contract.sh"
uploaded_launch_readiness_verifier="/tmp/backlit-verify-launch-readiness.sh"
uploaded_session_launch_verifier="/tmp/backlit-verify-session-launch.sh"
uploaded_session_replay_verifier="/tmp/backlit-verify-session-replay.sh"
uploaded_drm_session_smoke_verifier="/tmp/backlit-verify-drm-session-smoke.sh"
uploaded_service_lifecycle_verifier="/tmp/backlit-verify-service-lifecycle.sh"
uploaded_mvp0_contract_verifier="/tmp/backlit-verify-mvp0-contract.sh"
uploaded_mvp1_contract_verifier="/tmp/backlit-verify-mvp1-contract.sh"
uploaded_packaging_verifier="/tmp/backlit-verify-packaging-contract.sh"
uploaded_package_manifests_verifier="/tmp/backlit-verify-package-manifests.sh"
uploaded_debian_package_build_verifier="/tmp/backlit-verify-debian-package-build.sh"
uploaded_debian_package_install_verifier="/tmp/backlit-verify-debian-package-install.sh"
uploaded_debian_system_install_verifier="/tmp/backlit-verify-debian-system-install.sh"
uploaded_staged_install_verifier="/tmp/backlit-verify-staged-session-install.sh"
uploaded_systemd_activation_verifier="/tmp/backlit-verify-systemd-activation.sh"
uploaded_nested_verifier="/tmp/backlit-verify-nested-wayland-smoke.sh"

export DEBIAN_FRONTEND=noninteractive

guest_uid="\$(id -u "\$guest_user")"
guest_runtime_dir="/run/user/\$guest_uid"
guest_session_id="\$(loginctl list-sessions --no-legend 2>/dev/null | awk -v user="\$guest_user" '\$3 == user { print \$1; exit }' || true)"
guest_seat=""
guest_session_type=""
if [ -n "\$guest_session_id" ]; then
  guest_seat="\$(loginctl show-session "\$guest_session_id" -p Seat --value 2>/dev/null || true)"
  guest_session_type="\$(loginctl show-session "\$guest_session_id" -p Type --value 2>/dev/null || true)"
fi

apt-get update
apt-get install -y git ca-certificates build-essential pkg-config curl python3

if [ -d "\$guest_runtime_dir" ]; then
  export XDG_RUNTIME_DIR="\$guest_runtime_dir"
else
  unset XDG_RUNTIME_DIR
fi
if [ -n "\$guest_session_id" ]; then
  export XDG_SESSION_ID="\$guest_session_id"
else
  unset XDG_SESSION_ID
fi
if [ -n "\$guest_seat" ]; then
  export XDG_SEAT="\$guest_seat"
else
  unset XDG_SEAT
fi
if [ -n "\$guest_session_type" ]; then
  export XDG_SESSION_TYPE="\$guest_session_type"
else
  unset XDG_SESSION_TYPE
fi

printf 'Using guest session: user=%s uid=%s session=%s seat=%s type=%s runtime=%s\n' \
  "\$guest_user" "\$guest_uid" "\${guest_session_id:-none}" "\${guest_seat:-none}" \
  "\${guest_session_type:-none}" "\${XDG_RUNTIME_DIR:-none}"

if [ -e "\$repo_dir" ] && [ ! -d "\$repo_dir/.git" ]; then
  echo "Refusing to use non-Git path: \$repo_dir" >&2
  exit 2
fi

mkdir -p "\$(dirname "\$repo_dir")"
chown "\$guest_user:\$guest_user" "\$(dirname "\$repo_dir")"

if [ -d "\$repo_dir/.git" ]; then
  runuser -u "\$guest_user" -- git -C "\$repo_dir" fetch origin "\$branch"
  runuser -u "\$guest_user" -- git -C "\$repo_dir" checkout "\$branch"
  runuser -u "\$guest_user" -- git -C "\$repo_dir" reset --hard "origin/\$branch"
else
  runuser -u "\$guest_user" -- git clone --branch "\$branch" "\$repo_url" "\$repo_dir"
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

runuser -u "\$guest_user" -- bash -lc "
set -euo pipefail
source \"\\\$HOME/.cargo/env\"
cd \"\$repo_dir\"
git fetch origin \"\$branch\"
git checkout \"\$branch\"
git reset --hard \"origin/\$branch\"
"

install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_verifier" "\$repo_dir/scripts/verify-linux-e2e.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_gui_smoke_verifier" "\$repo_dir/scripts/verify-gui-smoke.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_gui_preview_renderer" "\$repo_dir/scripts/render-gui-preview.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_compositor_runtime_verifier" "\$repo_dir/scripts/verify-compositor-runtime.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_compositor_socket_verifier" "\$repo_dir/scripts/verify-compositor-socket.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_launch_performance_verifier" "\$repo_dir/scripts/verify-launch-performance.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_ci_contract_verifier" "\$repo_dir/scripts/verify-ci-contract.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_launch_readiness_verifier" "\$repo_dir/scripts/verify-launch-readiness.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_session_launch_verifier" "\$repo_dir/scripts/verify-session-launch.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_session_replay_verifier" "\$repo_dir/scripts/verify-session-replay.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_drm_session_smoke_verifier" "\$repo_dir/scripts/verify-drm-session-smoke.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_service_lifecycle_verifier" "\$repo_dir/scripts/verify-service-lifecycle.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_mvp0_contract_verifier" "\$repo_dir/scripts/verify-mvp0-contract.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_mvp1_contract_verifier" "\$repo_dir/scripts/verify-mvp1-contract.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_packaging_verifier" "\$repo_dir/scripts/verify-packaging-contract.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_package_manifests_verifier" "\$repo_dir/scripts/verify-package-manifests.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_debian_package_build_verifier" "\$repo_dir/scripts/verify-debian-package-build.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_debian_package_install_verifier" "\$repo_dir/scripts/verify-debian-package-install.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_debian_system_install_verifier" "\$repo_dir/scripts/verify-debian-system-install.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_staged_install_verifier" "\$repo_dir/scripts/verify-staged-session-install.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_systemd_activation_verifier" "\$repo_dir/scripts/verify-systemd-activation.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_nested_verifier" "\$repo_dir/scripts/verify-nested-wayland-smoke.sh"

runuser -u "\$guest_user" -- bash -lc "
set -euo pipefail
source \"\\\$HOME/.cargo/env\"
cd \"\$repo_dir\"
scripts/verify-linux-e2e.sh \"\$e2e_out_dir\"
"

cd "\$repo_dir"
BACKLIT_ALLOW_SYSTEM_PACKAGE_INSTALL=1 scripts/verify-debian-system-install.sh \
  "\$e2e_out_dir/debian-system-install" \
  "\$e2e_out_dir/debian-package-build"
chown -R "\$guest_user:\$guest_user" "\$repo_dir/\$e2e_out_dir/debian-system-install"
runuser -u "\$guest_user" -- bash -lc "
set -euo pipefail
source \"\\\$HOME/.cargo/env\"
cd \"\$repo_dir\"
scripts/verify-mvp1-contract.sh \"\$e2e_out_dir/mvp1-contract\" \"\$e2e_out_dir\"
"
EOF
chmod 700 "$root_runner"

printf 'Using Parallels VM: %s\n' "$vm_name"
"$prlctl_bin" list --all | grep -F "$vm_name" >/dev/null

upload_script "$repo_root/scripts/verify-linux-e2e.sh" "/tmp/backlit-verify-linux-e2e.sh"
upload_script "$repo_root/scripts/verify-gui-smoke.sh" "/tmp/backlit-verify-gui-smoke.sh"
upload_script "$repo_root/scripts/render-gui-preview.sh" "/tmp/backlit-render-gui-preview.sh"
upload_script "$repo_root/scripts/verify-compositor-runtime.sh" "/tmp/backlit-verify-compositor-runtime.sh"
upload_script "$repo_root/scripts/verify-compositor-socket.sh" "/tmp/backlit-verify-compositor-socket.sh"
upload_script "$repo_root/scripts/verify-launch-performance.sh" "/tmp/backlit-verify-launch-performance.sh"
upload_script "$repo_root/scripts/verify-ci-contract.sh" "/tmp/backlit-verify-ci-contract.sh"
upload_script "$repo_root/scripts/verify-launch-readiness.sh" "/tmp/backlit-verify-launch-readiness.sh"
upload_script "$repo_root/scripts/verify-session-launch.sh" "/tmp/backlit-verify-session-launch.sh"
upload_script "$repo_root/scripts/verify-session-replay.sh" "/tmp/backlit-verify-session-replay.sh"
upload_script "$repo_root/scripts/verify-drm-session-smoke.sh" "/tmp/backlit-verify-drm-session-smoke.sh"
upload_script "$repo_root/scripts/verify-service-lifecycle.sh" "/tmp/backlit-verify-service-lifecycle.sh"
upload_script "$repo_root/scripts/verify-mvp0-contract.sh" "/tmp/backlit-verify-mvp0-contract.sh"
upload_script "$repo_root/scripts/verify-mvp1-contract.sh" "/tmp/backlit-verify-mvp1-contract.sh"
upload_script "$repo_root/scripts/verify-packaging-contract.sh" "/tmp/backlit-verify-packaging-contract.sh"
upload_script "$repo_root/scripts/verify-package-manifests.sh" "/tmp/backlit-verify-package-manifests.sh"
upload_script "$repo_root/scripts/verify-debian-package-build.sh" "/tmp/backlit-verify-debian-package-build.sh"
upload_script "$repo_root/scripts/verify-debian-package-install.sh" "/tmp/backlit-verify-debian-package-install.sh"
upload_script "$repo_root/scripts/verify-debian-system-install.sh" "/tmp/backlit-verify-debian-system-install.sh"
upload_script "$repo_root/scripts/verify-staged-session-install.sh" "/tmp/backlit-verify-staged-session-install.sh"
upload_script "$repo_root/scripts/verify-systemd-activation.sh" "/tmp/backlit-verify-systemd-activation.sh"
upload_script "$repo_root/scripts/verify-nested-wayland-smoke.sh" "/tmp/backlit-verify-nested-wayland-smoke.sh"
upload_script "$root_runner" "/tmp/backlit-parallels-root-runner.sh"

"$prlctl_bin" exec "$vm_name" --user root /tmp/backlit-parallels-root-runner.sh

guest_commit="$("$prlctl_bin" exec "$vm_name" --user "$guest_user" git -C "$repo_dir" rev-parse --short HEAD | tr -d '\r')"
guest_e2e_dir="$repo_dir/$e2e_out_dir"

host_guest_manifest="$host_out_dir/guest-manifest.json"
host_gui_smoke_manifest="$host_out_dir/gui-smoke-manifest.json"
host_gui_preview_manifest="$host_out_dir/gui-preview-manifest.json"
host_compositor_runtime_manifest="$host_out_dir/compositor-runtime-manifest.json"
host_compositor_socket_manifest="$host_out_dir/compositor-socket-manifest.json"
host_smithay_runtime_probe_manifest="$host_out_dir/smithay-runtime-probe-manifest.json"
host_launch_readiness_manifest="$host_out_dir/launch-readiness-manifest.json"
host_session_replay_manifest="$host_out_dir/session-replay-manifest.json"
host_drm_session_smoke_manifest="$host_out_dir/drm-session-smoke-manifest.json"
host_debian_package_build_manifest="$host_out_dir/debian-package-build-manifest.json"
host_debian_package_install_manifest="$host_out_dir/debian-package-install-manifest.json"
host_debian_package_install_replay_manifest="$host_out_dir/debian-package-install-session-replay-manifest.json"
host_debian_system_install_manifest="$host_out_dir/debian-system-install-manifest.json"
host_debian_system_install_replay_manifest="$host_out_dir/debian-system-install-session-replay-manifest.json"
host_nested_wayland_manifest="$host_out_dir/nested-wayland-manifest.json"
host_mvp0_contract_manifest="$host_out_dir/mvp0-contract-manifest.json"
host_mvp1_contract_manifest="$host_out_dir/mvp1-contract-manifest.json"
host_ppm="$host_out_dir/gui-preview-backlit-session.ppm"
host_png="$host_out_dir/gui-preview-backlit-session.png"
host_compositor_preview_ppm="$host_out_dir/compositor-runtime-scripted-client-policy-preview.ppm"
host_compositor_preview_png="$host_out_dir/compositor-runtime-scripted-client-policy-preview.png"

mkdir -p "$host_out_dir"
rm -f \
  "$host_guest_manifest" \
  "$host_gui_smoke_manifest" \
  "$host_gui_preview_manifest" \
  "$host_compositor_runtime_manifest" \
  "$host_compositor_socket_manifest" \
  "$host_smithay_runtime_probe_manifest" \
  "$host_launch_readiness_manifest" \
  "$host_session_replay_manifest" \
  "$host_drm_session_smoke_manifest" \
  "$host_debian_package_build_manifest" \
  "$host_debian_package_install_manifest" \
  "$host_debian_package_install_replay_manifest" \
  "$host_debian_system_install_manifest" \
  "$host_debian_system_install_replay_manifest" \
  "$host_nested_wayland_manifest" \
  "$host_mvp0_contract_manifest" \
  "$host_mvp1_contract_manifest" \
  "$host_ppm" \
  "$host_png" \
  "$host_compositor_preview_ppm" \
  "$host_compositor_preview_png" \
  "$host_out_dir/manifest.json"

download_file "$guest_e2e_dir/manifest.json" "$host_guest_manifest"
download_file "$guest_e2e_dir/gui-smoke/manifest.json" "$host_gui_smoke_manifest"
download_file "$guest_e2e_dir/gui-preview/manifest.json" "$host_gui_preview_manifest"
download_file "$guest_e2e_dir/compositor-runtime/manifest.json" "$host_compositor_runtime_manifest"
download_file "$guest_e2e_dir/compositor-socket/manifest.json" "$host_compositor_socket_manifest"
download_file "$guest_e2e_dir/smithay-runtime-probe/manifest.json" "$host_smithay_runtime_probe_manifest"
download_file "$guest_e2e_dir/launch-readiness/manifest.json" "$host_launch_readiness_manifest"
download_file "$guest_e2e_dir/session-replay/manifest.json" "$host_session_replay_manifest"
download_file "$guest_e2e_dir/drm-session-smoke/manifest.json" "$host_drm_session_smoke_manifest"
download_file "$guest_e2e_dir/debian-package-build/manifest.json" "$host_debian_package_build_manifest"
download_file "$guest_e2e_dir/debian-package-install/manifest.json" "$host_debian_package_install_manifest"
download_file "$guest_e2e_dir/debian-package-install/session-replay/manifest.json" "$host_debian_package_install_replay_manifest"
download_file "$guest_e2e_dir/debian-system-install/manifest.json" "$host_debian_system_install_manifest"
download_file "$guest_e2e_dir/debian-system-install/session-replay/manifest.json" "$host_debian_system_install_replay_manifest"
download_file "$guest_e2e_dir/nested-wayland/manifest.json" "$host_nested_wayland_manifest"
download_file "$guest_e2e_dir/mvp0-contract/manifest.json" "$host_mvp0_contract_manifest"
download_file "$guest_e2e_dir/mvp1-contract/manifest.json" "$host_mvp1_contract_manifest"
download_file "$guest_e2e_dir/gui-preview/backlit-session.ppm" "$host_ppm"
download_file "$guest_e2e_dir/compositor-runtime/scripted-client-policy-preview.ppm" "$host_compositor_preview_ppm"

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
elif command -v pnmtopng >/dev/null 2>&1; then
  if pnmtopng "$host_ppm" > "$host_png"; then
    preview_image="$host_png"
    preview_format="png"
    png_written=true
    converter="pnmtopng"
  fi
fi

ppm_bytes="$(wc -c < "$host_ppm" | tr -d ' ')"
test "$ppm_bytes" = "1248015"

compositor_preview_image="$host_compositor_preview_ppm"
compositor_preview_format="ppm"
compositor_png_written=false
compositor_converter="none"

if command -v sips >/dev/null 2>&1; then
  if sips -s format png "$host_compositor_preview_ppm" --out "$host_compositor_preview_png" >/dev/null 2>&1; then
    compositor_preview_image="$host_compositor_preview_png"
    compositor_preview_format="png"
    compositor_png_written=true
    compositor_converter="sips"
  fi
elif command -v magick >/dev/null 2>&1; then
  if magick "$host_compositor_preview_ppm" "$host_compositor_preview_png" >/dev/null 2>&1; then
    compositor_preview_image="$host_compositor_preview_png"
    compositor_preview_format="png"
    compositor_png_written=true
    compositor_converter="magick"
  fi
elif command -v convert >/dev/null 2>&1; then
  if convert "$host_compositor_preview_ppm" "$host_compositor_preview_png" >/dev/null 2>&1; then
    compositor_preview_image="$host_compositor_preview_png"
    compositor_preview_format="png"
    compositor_png_written=true
    compositor_converter="convert"
  fi
elif command -v pnmtopng >/dev/null 2>&1; then
  if pnmtopng "$host_compositor_preview_ppm" > "$host_compositor_preview_png"; then
    compositor_preview_image="$host_compositor_preview_png"
    compositor_preview_format="png"
    compositor_png_written=true
    compositor_converter="pnmtopng"
  fi
fi

compositor_preview_ppm_bytes="$(wc -c < "$host_compositor_preview_ppm" | tr -d ' ')"
test "$compositor_preview_ppm_bytes" -gt 10000

require_contains "$host_guest_manifest" '"passed": true'
require_contains "$host_guest_manifest" "\"commit\": \"$guest_commit\""
require_contains "$host_guest_manifest" '"debian_package_build": true'
require_contains "$host_guest_manifest" '"debian_package_install": true'
require_contains "$host_guest_manifest" '"launch_readiness": true'
require_contains "$host_guest_manifest" '"drm_session_smoke": true'
require_contains "$host_guest_manifest" '"nested_wayland": true'
require_contains "$host_guest_manifest" '"mvp1_contract": true'
require_contains "$host_gui_smoke_manifest" '"golden_checksum": true'
require_contains "$host_gui_smoke_manifest" '"session_compositor_demo_client": true'
require_contains "$host_gui_smoke_manifest" '"session_compositor_demo_app_id_preserved": true'
require_contains "$host_gui_smoke_manifest" '"session_desktop_launch": true'
require_contains "$host_gui_smoke_manifest" '"session_desktop_managed_window": true'
require_contains "$host_gui_preview_manifest" '"session_verified": true'
require_contains "$host_gui_preview_manifest" '"session_services": true'
require_contains "$host_compositor_runtime_manifest" '"scripted_client_runtime": true'
require_contains "$host_compositor_runtime_manifest" '"surface_policy_preview": true'
require_contains "$host_compositor_runtime_manifest" '"policy_preview_ppm_bytes":'
require_contains "$host_compositor_runtime_manifest" '"targeted_surface_damage": true'
require_contains "$host_compositor_runtime_manifest" '"client_disconnect_cleanup": true'
require_contains "$host_compositor_socket_manifest" '"session_socket_bound": true'
require_contains "$host_compositor_socket_manifest" '"socket_accepts_client_connection": true'
require_contains "$host_compositor_socket_manifest" '"demo_client_socket_launch": true'
require_contains "$host_compositor_socket_manifest" '"demo_client_surface_mapped": true'
require_contains "$host_compositor_socket_manifest" '"session_socket_cleanup": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_dependency_compiled": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_backend_feature": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_drm_component": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libinput_component": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libseat_session_component": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_calloop_component": true'
require_contains "$host_smithay_runtime_probe_manifest" '"drm_launch_ready": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_runtime_probe": true'
require_contains "$host_session_replay_manifest" '"session_replay_event": true'
require_contains "$host_session_replay_manifest" '"frame_count": 9'
require_contains "$host_session_replay_manifest" '"launcher_overlay_frame": true'
require_contains "$host_session_replay_manifest" '"app_switcher_overlay_frame": true'
require_contains "$host_session_replay_manifest" '"workspace_switch": true'
require_contains "$host_launch_readiness_manifest" '"drm_expected_ready": true'
require_contains "$host_launch_readiness_manifest" '"drm_ready": true'
require_contains "$host_launch_readiness_manifest" '"xdg_runtime_dir_owned_by_user": true'
require_contains "$host_launch_readiness_manifest" '"session_local": true'
require_contains "$host_launch_readiness_manifest" '"drm_card_access_ready": true'
require_contains "$host_launch_readiness_manifest" '"input_broker_ready": true'
require_contains "$host_launch_readiness_manifest" '"drm_launch_plan": true'
require_contains "$host_launch_readiness_manifest" '"drm_device_selected": true'
require_contains "$host_launch_readiness_manifest" '"drm_input_selected": true'
require_contains "$host_drm_session_smoke_manifest" '"drm_session_smoke_ready": true'
require_contains "$host_drm_session_smoke_manifest" '"drm_session_clean_exit": true'
require_contains "$host_drm_session_smoke_manifest" '"drm_backend_launch_plan": true'
require_contains "$host_drm_session_smoke_manifest" '"drm_device_selected": true'
require_contains "$host_drm_session_smoke_manifest" '"drm_input_selected": true'
require_contains "$host_drm_session_smoke_manifest" '"session_compositor_demo_client": true'
require_contains "$host_drm_session_smoke_manifest" '"session_compositor_demo_app_id_preserved": true'
require_contains "$host_drm_session_smoke_manifest" '"session_desktop_launch": true'
require_contains "$host_drm_session_smoke_manifest" '"session_desktop_managed_window": true'
require_contains "$host_drm_session_smoke_manifest" '"settings_service": true'
require_contains "$host_drm_session_smoke_manifest" '"notification_service": true'
require_contains "$host_debian_package_build_manifest" '"debs_built": true'
require_contains "$host_debian_package_build_manifest" '"fastgui_core_deb": true'
require_contains "$host_debian_package_install_manifest" '"debs_installed": true'
require_contains "$host_debian_package_install_manifest" '"dpkg_root_install": true'
require_contains "$host_debian_package_install_manifest" '"session_gui_from_extracted_debs": true'
require_contains "$host_debian_package_install_manifest" '"session_services_from_extracted_debs": true'
require_contains "$host_debian_package_install_manifest" '"session_compositor_demo_client_from_extracted_debs": true'
require_contains "$host_debian_package_install_manifest" '"session_compositor_demo_app_id_from_extracted_debs": true'
require_contains "$host_debian_package_install_manifest" '"session_desktop_launch_from_extracted_debs": true'
require_contains "$host_debian_package_install_manifest" '"session_desktop_managed_window_from_extracted_debs": true'
require_contains "$host_debian_package_install_manifest" '"session_replay_from_extracted_debs": true'
require_contains "$host_debian_package_install_manifest" '"session_clean_exit_from_extracted_debs": true'
require_contains "$host_debian_package_install_replay_manifest" '"frame_count": 9'
require_contains "$host_debian_package_install_replay_manifest" '"launcher_overlay_frame": true'
require_contains "$host_debian_package_install_replay_manifest" '"app_switcher_overlay_frame": true'
require_contains "$host_debian_system_install_manifest" '"system_install_performed": true'
require_contains "$host_debian_system_install_manifest" '"actual_system_dpkg_install": true'
require_contains "$host_debian_system_install_manifest" '"usr_bin_session_launch": true'
require_contains "$host_debian_system_install_manifest" '"session_services_from_system_install": true'
require_contains "$host_debian_system_install_manifest" '"session_compositor_demo_client_from_system_install": true'
require_contains "$host_debian_system_install_manifest" '"session_compositor_demo_app_id_from_system_install": true'
require_contains "$host_debian_system_install_manifest" '"session_desktop_launch_from_system_install": true'
require_contains "$host_debian_system_install_manifest" '"session_desktop_managed_window_from_system_install": true'
require_contains "$host_debian_system_install_manifest" '"session_replay_from_system_install": true'
require_contains "$host_debian_system_install_manifest" '"packages_purged_after_verification": true'
require_contains "$host_debian_system_install_replay_manifest" '"frame_count": 9'
require_contains "$host_debian_system_install_replay_manifest" '"launcher_overlay_frame": true'
require_contains "$host_debian_system_install_replay_manifest" '"app_switcher_overlay_frame": true'
require_contains "$host_nested_wayland_manifest" '"session_wayland_clean_exit": true'
require_contains "$host_mvp0_contract_manifest" '"artifact_manifests_checked": true'
require_contains "$host_mvp1_contract_manifest" '"artifact_manifests_checked": true'
require_contains "$host_mvp1_contract_manifest" '"drm_launch_ready_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"drm_session_smoke_ready_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"debian_package_install_replay_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"debian_system_install_replay_artifact": true'

cat > "$host_out_dir/manifest.json" <<EOF
{
  "name": "backlit-parallels-linux-e2e-export",
  "passed": true,
  "vm": "$vm_name",
  "guest_commit": "$guest_commit",
  "guest_repo": "$repo_dir",
  "guest_e2e_dir": "$e2e_out_dir",
  "artifacts": {
    "guest_manifest": "$host_guest_manifest",
    "gui_smoke_manifest": "$host_gui_smoke_manifest",
    "gui_preview_manifest": "$host_gui_preview_manifest",
    "compositor_runtime_manifest": "$host_compositor_runtime_manifest",
    "compositor_socket_manifest": "$host_compositor_socket_manifest",
    "smithay_runtime_probe_manifest": "$host_smithay_runtime_probe_manifest",
    "launch_readiness_manifest": "$host_launch_readiness_manifest",
    "session_replay_manifest": "$host_session_replay_manifest",
    "drm_session_smoke_manifest": "$host_drm_session_smoke_manifest",
    "debian_package_build_manifest": "$host_debian_package_build_manifest",
    "debian_package_install_manifest": "$host_debian_package_install_manifest",
    "debian_package_install_replay_manifest": "$host_debian_package_install_replay_manifest",
    "debian_system_install_manifest": "$host_debian_system_install_manifest",
    "debian_system_install_replay_manifest": "$host_debian_system_install_replay_manifest",
    "nested_wayland_manifest": "$host_nested_wayland_manifest",
    "mvp0_contract_manifest": "$host_mvp0_contract_manifest",
    "mvp1_contract_manifest": "$host_mvp1_contract_manifest",
    "gui_preview_ppm": "$host_ppm",
    "gui_preview_image": "$preview_image",
    "compositor_runtime_preview_ppm": "$host_compositor_preview_ppm",
    "compositor_runtime_preview_image": "$compositor_preview_image"
  },
  "checks": {
    "guest_e2e_passed": true,
    "guest_commit_matches_manifest": true,
    "guest_artifacts_exported": true,
	    "gui_smoke": true,
	    "gui_smoke_session_desktop_managed_window": true,
	    "gui_smoke_demo_client_app_id": true,
	    "gui_preview": true,
    "compositor_runtime": true,
    "compositor_runtime_policy_preview": true,
    "compositor_socket": true,
    "smithay_runtime_probe": true,
    "launch_readiness": true,
    "drm_launch_plan": true,
    "session_replay": true,
    "parallels_drm_launch_ready": true,
	    "drm_session_smoke": true,
	    "drm_session_clean_exit": true,
	    "drm_session_desktop_managed_window": true,
	    "drm_session_demo_client_app_id": true,
	    "debian_package_build": true,
	    "debian_package_install": true,
	    "debian_package_install_desktop_managed_window": true,
	    "debian_package_install_demo_client_app_id": true,
	    "debian_package_install_replay": true,
    "dpkg_root_install": true,
	    "debian_system_install": true,
	    "debian_system_install_desktop_managed_window": true,
	    "debian_system_install_demo_client_app_id": true,
	    "debian_system_install_replay": true,
    "actual_system_dpkg_install": true,
    "nested_wayland": true,
    "mvp0_contract": true,
    "mvp1_contract": true,
    "ppm_bytes": $ppm_bytes,
    "png_written": $png_written,
    "preview_format": "$preview_format",
    "converter": "$converter",
    "compositor_preview_ppm_bytes": $compositor_preview_ppm_bytes,
    "compositor_png_written": $compositor_png_written,
    "compositor_preview_format": "$compositor_preview_format",
    "compositor_converter": "$compositor_converter"
  }
}
EOF

printf 'Backlit Parallels Linux E2E exported: %s\n' "$host_out_dir"
printf 'Manifest: %s\n' "$host_out_dir/manifest.json"
if [ "$preview_format" = "png" ]; then
  printf 'To view E2E preview on macOS: open %s\n' "$preview_image"
else
  printf 'No PNG converter found; view the PPM directly or install ImageMagick/netpbm.\n'
fi
if [ "$compositor_preview_format" = "png" ]; then
  printf 'To view compositor-runtime preview on macOS: open %s\n' "$compositor_preview_image"
fi
