#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
credential_file="${BACKLIT_PARALLELS_CREDENTIAL_FILE:-$repo_root/.local/parallels-ubuntu.env}"
vm_name="${BACKLIT_PARALLELS_VM:-Ubuntu 22.04.2 ARM64}"
repo_url="${BACKLIT_E2E_REPO_URL:-https://github.com/juncoflockleader/backlit.git}"
branch="${BACKLIT_E2E_BRANCH:-main}"
host_out_dir="${1:-${BACKLIT_PARALLELS_E2E_HOST_OUT_DIR:-target/linux-e2e-parallels}}"
e2e_out_dir="${BACKLIT_E2E_OUT_DIR:-$host_out_dir}"
health_out_dir="$host_out_dir/parallels-ubuntu-health"
health_manifest="$health_out_dir/manifest.json"

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

require_matches() {
  local file="$1"
  local value="$2"
  grep -E "$value" "$file" >/dev/null || {
    echo "Parallels E2E export verification failed: missing pattern in $file: $value" >&2
    exit 1
  }
}

check_guest_writable() {
  local probe_path="/tmp/backlit-linux-e2e-write-check-$$"
  local probe_log="$tmp_dir/guest-writeability.log"

  if "$prlctl_bin" exec "$vm_name" --user root sh -lc \
    "mount | grep ' / ' || true; if touch '$probe_path'; then rm -f '$probe_path'; echo write_probe=ok; else echo write_probe=failed; exit 1; fi" \
    > "$probe_log" 2>&1; then
    return 0
  fi

  cat >&2 <<EOF
Parallels Linux E2E cannot start because the Ubuntu guest is not writable.

VM: $vm_name
Probe output:
EOF
  cat "$probe_log" >&2
  cat >&2 <<EOF

Restart or repair the Ubuntu VM so its root filesystem mounts read-write, then rerun:
  $0 $host_out_dir
EOF
  exit 2
}

run_health_preflight() {
  if "$repo_root/scripts/verify-parallels-ubuntu-health.sh" "$health_out_dir"; then
    return 0
  else
    local status="$?"
    cat >&2 <<EOF
Parallels Linux E2E cannot start because the Ubuntu health preflight failed.

Health manifest: $health_manifest
EOF
    exit "$status"
  fi
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
uploaded_smithay_compositor_runtime_verifier="/tmp/backlit-verify-smithay-compositor-runtime.sh"
uploaded_smithay_live_surface_snapshots_verifier="/tmp/backlit-verify-smithay-live-surface-snapshots.sh"
uploaded_smithay_real_app_e2e_verifier="/tmp/backlit-verify-smithay-real-app-e2e.sh"
uploaded_smithay_real_shm_frame_verifier="/tmp/backlit-verify-smithay-real-shm-frame.sh"
uploaded_compositor_socket_verifier="/tmp/backlit-verify-compositor-socket.sh"
uploaded_launch_performance_verifier="/tmp/backlit-verify-launch-performance.sh"
uploaded_resource_budget_verifier="/tmp/backlit-verify-resource-budget.sh"
uploaded_ci_contract_verifier="/tmp/backlit-verify-ci-contract.sh"
uploaded_launch_readiness_verifier="/tmp/backlit-verify-launch-readiness.sh"
uploaded_session_launch_verifier="/tmp/backlit-verify-session-launch.sh"
uploaded_session_replay_verifier="/tmp/backlit-verify-session-replay.sh"
uploaded_drm_session_smoke_verifier="/tmp/backlit-verify-drm-session-smoke.sh"
uploaded_drm_master_boundary_verifier="/tmp/backlit-verify-drm-master-boundary.sh"
uploaded_dedicated_drm_session_verifier="/tmp/backlit-verify-dedicated-drm-session.sh"
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

install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_verifier" "\$repo_dir/scripts/verify-linux-e2e.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_gui_smoke_verifier" "\$repo_dir/scripts/verify-gui-smoke.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_gui_preview_renderer" "\$repo_dir/scripts/render-gui-preview.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_compositor_runtime_verifier" "\$repo_dir/scripts/verify-compositor-runtime.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_smithay_compositor_runtime_verifier" "\$repo_dir/scripts/verify-smithay-compositor-runtime.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_smithay_live_surface_snapshots_verifier" "\$repo_dir/scripts/verify-smithay-live-surface-snapshots.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_smithay_real_app_e2e_verifier" "\$repo_dir/scripts/verify-smithay-real-app-e2e.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_smithay_real_shm_frame_verifier" "\$repo_dir/scripts/verify-smithay-real-shm-frame.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_compositor_socket_verifier" "\$repo_dir/scripts/verify-compositor-socket.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_launch_performance_verifier" "\$repo_dir/scripts/verify-launch-performance.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_resource_budget_verifier" "\$repo_dir/scripts/verify-resource-budget.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_ci_contract_verifier" "\$repo_dir/scripts/verify-ci-contract.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_launch_readiness_verifier" "\$repo_dir/scripts/verify-launch-readiness.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_session_launch_verifier" "\$repo_dir/scripts/verify-session-launch.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_session_replay_verifier" "\$repo_dir/scripts/verify-session-replay.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_drm_session_smoke_verifier" "\$repo_dir/scripts/verify-drm-session-smoke.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_drm_master_boundary_verifier" "\$repo_dir/scripts/verify-drm-master-boundary.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_dedicated_drm_session_verifier" "\$repo_dir/scripts/verify-dedicated-drm-session.sh"
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
run_health_preflight
"$prlctl_bin" list --all | grep -F "$vm_name" >/dev/null
check_guest_writable

upload_script "$repo_root/scripts/verify-linux-e2e.sh" "/tmp/backlit-verify-linux-e2e.sh"
upload_script "$repo_root/scripts/verify-gui-smoke.sh" "/tmp/backlit-verify-gui-smoke.sh"
upload_script "$repo_root/scripts/render-gui-preview.sh" "/tmp/backlit-render-gui-preview.sh"
upload_script "$repo_root/scripts/verify-compositor-runtime.sh" "/tmp/backlit-verify-compositor-runtime.sh"
upload_script "$repo_root/scripts/verify-smithay-compositor-runtime.sh" "/tmp/backlit-verify-smithay-compositor-runtime.sh"
upload_script "$repo_root/scripts/verify-smithay-live-surface-snapshots.sh" "/tmp/backlit-verify-smithay-live-surface-snapshots.sh"
upload_script "$repo_root/scripts/verify-smithay-real-app-e2e.sh" "/tmp/backlit-verify-smithay-real-app-e2e.sh"
upload_script "$repo_root/scripts/verify-smithay-real-shm-frame.sh" "/tmp/backlit-verify-smithay-real-shm-frame.sh"
upload_script "$repo_root/scripts/verify-compositor-socket.sh" "/tmp/backlit-verify-compositor-socket.sh"
upload_script "$repo_root/scripts/verify-launch-performance.sh" "/tmp/backlit-verify-launch-performance.sh"
upload_script "$repo_root/scripts/verify-resource-budget.sh" "/tmp/backlit-verify-resource-budget.sh"
upload_script "$repo_root/scripts/verify-ci-contract.sh" "/tmp/backlit-verify-ci-contract.sh"
upload_script "$repo_root/scripts/verify-launch-readiness.sh" "/tmp/backlit-verify-launch-readiness.sh"
upload_script "$repo_root/scripts/verify-session-launch.sh" "/tmp/backlit-verify-session-launch.sh"
upload_script "$repo_root/scripts/verify-session-replay.sh" "/tmp/backlit-verify-session-replay.sh"
upload_script "$repo_root/scripts/verify-drm-session-smoke.sh" "/tmp/backlit-verify-drm-session-smoke.sh"
upload_script "$repo_root/scripts/verify-drm-master-boundary.sh" "/tmp/backlit-verify-drm-master-boundary.sh"
upload_script "$repo_root/scripts/verify-dedicated-drm-session.sh" "/tmp/backlit-verify-dedicated-drm-session.sh"
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
host_smithay_compositor_runtime_manifest="$host_out_dir/smithay-compositor-runtime-manifest.json"
host_smithay_live_surface_snapshots_manifest="$host_out_dir/smithay-live-surface-snapshots-manifest.json"
host_smithay_real_app_e2e_manifest="$host_out_dir/smithay-real-app-e2e-manifest.json"
host_smithay_real_shm_frame_manifest="$host_out_dir/smithay-real-shm-frame-manifest.json"
host_compositor_socket_manifest="$host_out_dir/compositor-socket-manifest.json"
host_smithay_runtime_probe_manifest="$host_out_dir/smithay-runtime-probe-manifest.json"
host_launch_performance_manifest="$host_out_dir/launch-performance-manifest.json"
host_resource_budget_manifest="$host_out_dir/resource-budget-manifest.json"
host_launch_readiness_manifest="$host_out_dir/launch-readiness-manifest.json"
host_session_launch_manifest="$host_out_dir/session-launch-manifest.json"
host_session_replay_manifest="$host_out_dir/session-replay-manifest.json"
host_drm_session_smoke_manifest="$host_out_dir/drm-session-smoke-manifest.json"
host_drm_master_boundary_manifest="$host_out_dir/drm-master-boundary-manifest.json"
host_dedicated_drm_session_manifest="$host_out_dir/dedicated-drm-session-manifest.json"
host_dedicated_drm_handoff="$host_out_dir/dedicated-drm-handoff.sh"
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
host_smithay_real_app_e2e_ppm="$host_out_dir/smithay-real-app-e2e.ppm"
host_smithay_real_app_e2e_png="$host_out_dir/smithay-real-app-e2e.png"
host_smithay_real_shm_frame_ppm="$host_out_dir/smithay-real-shm-frame.ppm"
host_smithay_real_shm_frame_png="$host_out_dir/smithay-real-shm-frame.png"

mkdir -p "$host_out_dir"
rm -f \
  "$host_guest_manifest" \
  "$host_gui_smoke_manifest" \
  "$host_gui_preview_manifest" \
  "$host_compositor_runtime_manifest" \
  "$host_smithay_compositor_runtime_manifest" \
  "$host_smithay_live_surface_snapshots_manifest" \
  "$host_smithay_real_app_e2e_manifest" \
  "$host_smithay_real_shm_frame_manifest" \
  "$host_compositor_socket_manifest" \
  "$host_smithay_runtime_probe_manifest" \
  "$host_launch_performance_manifest" \
  "$host_resource_budget_manifest" \
  "$host_launch_readiness_manifest" \
  "$host_session_launch_manifest" \
  "$host_session_replay_manifest" \
  "$host_drm_session_smoke_manifest" \
  "$host_drm_master_boundary_manifest" \
  "$host_dedicated_drm_session_manifest" \
  "$host_dedicated_drm_handoff" \
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
  "$host_smithay_real_app_e2e_ppm" \
  "$host_smithay_real_app_e2e_png" \
  "$host_smithay_real_shm_frame_ppm" \
  "$host_smithay_real_shm_frame_png" \
  "$host_out_dir/manifest.json"

download_file "$guest_e2e_dir/manifest.json" "$host_guest_manifest"
download_file "$guest_e2e_dir/gui-smoke/manifest.json" "$host_gui_smoke_manifest"
download_file "$guest_e2e_dir/gui-preview/manifest.json" "$host_gui_preview_manifest"
download_file "$guest_e2e_dir/compositor-runtime/manifest.json" "$host_compositor_runtime_manifest"
download_file "$guest_e2e_dir/smithay-compositor-runtime/manifest.json" "$host_smithay_compositor_runtime_manifest"
download_file "$guest_e2e_dir/smithay-live-surface-snapshots/manifest.json" "$host_smithay_live_surface_snapshots_manifest"
download_file "$guest_e2e_dir/smithay-real-app-e2e/manifest.json" "$host_smithay_real_app_e2e_manifest"
download_file "$guest_e2e_dir/smithay-real-shm-frame/manifest.json" "$host_smithay_real_shm_frame_manifest"
download_file "$guest_e2e_dir/compositor-socket/manifest.json" "$host_compositor_socket_manifest"
download_file "$guest_e2e_dir/smithay-runtime-probe/manifest.json" "$host_smithay_runtime_probe_manifest"
download_file "$guest_e2e_dir/launch-performance/manifest.json" "$host_launch_performance_manifest"
download_file "$guest_e2e_dir/resource-budget/manifest.json" "$host_resource_budget_manifest"
download_file "$guest_e2e_dir/launch-readiness/manifest.json" "$host_launch_readiness_manifest"
download_file "$guest_e2e_dir/session-launch/manifest.json" "$host_session_launch_manifest"
download_file "$guest_e2e_dir/session-replay/manifest.json" "$host_session_replay_manifest"
download_file "$guest_e2e_dir/drm-session-smoke/manifest.json" "$host_drm_session_smoke_manifest"
download_file "$guest_e2e_dir/drm-master-boundary/manifest.json" "$host_drm_master_boundary_manifest"
download_file "$guest_e2e_dir/dedicated-drm-session/manifest.json" "$host_dedicated_drm_session_manifest"
download_file "$guest_e2e_dir/dedicated-drm-session/dedicated-drm-handoff.sh" "$host_dedicated_drm_handoff"
chmod +x "$host_dedicated_drm_handoff"
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
download_file "$guest_e2e_dir/smithay-real-app-e2e/backlit-real-app-frame.ppm" "$host_smithay_real_app_e2e_ppm"
download_file "$guest_e2e_dir/smithay-real-shm-frame/backlit-real-shm-frame.ppm" "$host_smithay_real_shm_frame_ppm"

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

real_app_e2e_image="$host_smithay_real_app_e2e_ppm"
real_app_e2e_format="ppm"
real_app_e2e_png_written=false
real_app_e2e_converter="none"

if command -v sips >/dev/null 2>&1; then
  if sips -s format png "$host_smithay_real_app_e2e_ppm" --out "$host_smithay_real_app_e2e_png" >/dev/null 2>&1; then
    real_app_e2e_image="$host_smithay_real_app_e2e_png"
    real_app_e2e_format="png"
    real_app_e2e_png_written=true
    real_app_e2e_converter="sips"
  fi
elif command -v magick >/dev/null 2>&1; then
  if magick "$host_smithay_real_app_e2e_ppm" "$host_smithay_real_app_e2e_png" >/dev/null 2>&1; then
    real_app_e2e_image="$host_smithay_real_app_e2e_png"
    real_app_e2e_format="png"
    real_app_e2e_png_written=true
    real_app_e2e_converter="magick"
  fi
elif command -v convert >/dev/null 2>&1; then
  if convert "$host_smithay_real_app_e2e_ppm" "$host_smithay_real_app_e2e_png" >/dev/null 2>&1; then
    real_app_e2e_image="$host_smithay_real_app_e2e_png"
    real_app_e2e_format="png"
    real_app_e2e_png_written=true
    real_app_e2e_converter="convert"
  fi
elif command -v pnmtopng >/dev/null 2>&1; then
  if pnmtopng "$host_smithay_real_app_e2e_ppm" > "$host_smithay_real_app_e2e_png"; then
    real_app_e2e_image="$host_smithay_real_app_e2e_png"
    real_app_e2e_format="png"
    real_app_e2e_png_written=true
    real_app_e2e_converter="pnmtopng"
  fi
fi

real_app_e2e_ppm_bytes="$(wc -c < "$host_smithay_real_app_e2e_ppm" | tr -d ' ')"
test "$real_app_e2e_ppm_bytes" -gt 10000

real_shm_frame_image="$host_smithay_real_shm_frame_ppm"
real_shm_frame_format="ppm"
real_shm_frame_png_written=false
real_shm_frame_converter="none"

if command -v sips >/dev/null 2>&1; then
  if sips -s format png "$host_smithay_real_shm_frame_ppm" --out "$host_smithay_real_shm_frame_png" >/dev/null 2>&1; then
    real_shm_frame_image="$host_smithay_real_shm_frame_png"
    real_shm_frame_format="png"
    real_shm_frame_png_written=true
    real_shm_frame_converter="sips"
  fi
elif command -v magick >/dev/null 2>&1; then
  if magick "$host_smithay_real_shm_frame_ppm" "$host_smithay_real_shm_frame_png" >/dev/null 2>&1; then
    real_shm_frame_image="$host_smithay_real_shm_frame_png"
    real_shm_frame_format="png"
    real_shm_frame_png_written=true
    real_shm_frame_converter="magick"
  fi
elif command -v convert >/dev/null 2>&1; then
  if convert "$host_smithay_real_shm_frame_ppm" "$host_smithay_real_shm_frame_png" >/dev/null 2>&1; then
    real_shm_frame_image="$host_smithay_real_shm_frame_png"
    real_shm_frame_format="png"
    real_shm_frame_png_written=true
    real_shm_frame_converter="convert"
  fi
elif command -v pnmtopng >/dev/null 2>&1; then
  if pnmtopng "$host_smithay_real_shm_frame_ppm" > "$host_smithay_real_shm_frame_png"; then
    real_shm_frame_image="$host_smithay_real_shm_frame_png"
    real_shm_frame_format="png"
    real_shm_frame_png_written=true
    real_shm_frame_converter="pnmtopng"
  fi
fi

real_shm_frame_ppm_bytes="$(wc -c < "$host_smithay_real_shm_frame_ppm" | tr -d ' ')"
test "$real_shm_frame_ppm_bytes" -gt 10000

require_contains "$host_guest_manifest" '"passed": true'
require_contains "$host_guest_manifest" "\"commit\": \"$guest_commit\""
require_contains "$host_guest_manifest" '"debian_package_build": true'
require_contains "$host_guest_manifest" '"debian_package_install": true'
require_contains "$host_guest_manifest" '"launch_readiness": true'
require_contains "$host_guest_manifest" '"smithay_compositor_runtime": true'
require_contains "$host_guest_manifest" '"smithay_live_surface_snapshots": true'
require_contains "$host_guest_manifest" '"smithay_real_app_e2e": true'
require_contains "$host_guest_manifest" '"smithay_real_shm_frame": true'
require_contains "$host_guest_manifest" '"drm_master_boundary": true'
require_contains "$host_guest_manifest" '"drm_session_smoke": true'
require_contains "$host_guest_manifest" '"dedicated_drm_session": true'
require_contains "$host_guest_manifest" '"dedicated_drm_handoff": true'
require_contains "$host_guest_manifest" '"launch_performance": true'
require_contains "$host_guest_manifest" '"resource_budget": true'
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
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_compositor_runtime": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_runtime_trait": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_runtime_launch_plan": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_scripted_client": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_core_protocol_globals": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_mvp_protocol_globals": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_seat_global": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_keyboard_pointer_capabilities": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_input_sources": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_input_event_loop": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_input_seat_handles": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_input_seat_dispatch": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_input_event_classification": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_real_wayland_client": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_real_wayland_metadata": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_real_shm_buffer": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_real_wayland_policy_window": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_event_loop_runtime": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_drm_first_present_probe": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_service_ready": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_service_socket": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_service_socket_runtime_trait": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_event_loop_service_socket": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"smithay_demo_client_socket_lifecycle": true'
require_contains "$host_smithay_compositor_runtime_manifest" '"drm_launch_ready": true'
require_contains "$host_smithay_live_surface_snapshots_manifest" '"smithay_live_surface_snapshots": true'
require_contains "$host_smithay_live_surface_snapshots_manifest" '"real_wayland_client": true'
require_contains "$host_smithay_live_surface_snapshots_manifest" '"live_snapshot_pipeline": true'
require_contains "$host_smithay_live_surface_snapshots_manifest" '"live_snapshot_persisted": true'
require_contains "$host_smithay_live_surface_snapshots_manifest" '"live_snapshot_metadata_preserved": true'
require_contains "$host_smithay_live_surface_snapshots_manifest" '"live_snapshot_pixels_copied": true'
require_contains "$host_smithay_live_surface_snapshots_manifest" '"live_snapshot_damage_recorded": true'
require_contains "$host_smithay_live_surface_snapshots_manifest" '"live_snapshot_samples_verified": true'
require_contains "$host_smithay_live_surface_snapshots_manifest" '"policy_window_from_live_snapshot": true'
require_contains "$host_smithay_live_surface_snapshots_manifest" '"drm_launch_ready": true'
require_contains "$host_smithay_real_app_e2e_manifest" '"smithay_real_app_e2e": true'
require_contains "$host_smithay_real_app_e2e_manifest" '"real_installed_app": true'
require_contains "$host_smithay_real_app_e2e_manifest" '"real_app_wayland_client_connected": true'
require_contains "$host_smithay_real_app_e2e_manifest" '"real_app_metadata_observed": true'
require_contains "$host_smithay_real_app_e2e_manifest" '"real_app_shm_pixels_captured": true'
require_contains "$host_smithay_real_app_e2e_manifest" '"real_app_pixels_composited": true'
require_contains "$host_smithay_real_app_e2e_manifest" '"real_app_frame_samples_verified": true'
require_contains "$host_smithay_real_app_e2e_manifest" '"policy_window_from_real_app": true'
require_contains "$host_smithay_real_app_e2e_manifest" '"frame_ppm_written": true'
require_contains "$host_smithay_real_app_e2e_manifest" '"drm_launch_ready": true'
require_contains "$host_smithay_real_shm_frame_manifest" '"smithay_real_shm_frame": true'
require_contains "$host_smithay_real_shm_frame_manifest" '"real_wayland_client": true'
require_contains "$host_smithay_real_shm_frame_manifest" '"real_wayland_metadata": true'
require_contains "$host_smithay_real_shm_frame_manifest" '"real_shm_pixels_captured": true'
require_contains "$host_smithay_real_shm_frame_manifest" '"real_shm_pixels_composited": true'
require_contains "$host_smithay_real_shm_frame_manifest" '"real_client_pixel_samples_verified": true'
require_contains "$host_smithay_real_shm_frame_manifest" '"policy_window_from_real_surface": true'
require_contains "$host_smithay_real_shm_frame_manifest" '"frame_ppm_written": true'
require_contains "$host_smithay_real_shm_frame_manifest" '"drm_launch_ready": true'
require_contains "$host_compositor_socket_manifest" '"session_socket_bound": true'
require_contains "$host_compositor_socket_manifest" '"socket_accepts_client_connection": true'
require_contains "$host_compositor_socket_manifest" '"demo_client_socket_launch": true'
require_contains "$host_compositor_socket_manifest" '"demo_client_surface_mapped": true'
require_contains "$host_compositor_socket_manifest" '"session_socket_cleanup": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_dependency_compiled": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_backend_feature": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_drm_component": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_gbm_allocator_component": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_egl_display_component": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_gles_renderer_component": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_drm_node_resolved": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_card_opened": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_device_created": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_event_source_inserted": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_event_loop_dispatched": true'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_crtc_count": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_connector_count": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_connected_connector_count": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_mode_count": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_primary_plane_count": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_cursor_plane_count": [0-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_overlay_plane_count": [0-9][0-9]*'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_scanout_plan_ready": true'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_scanout_connector_id": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_scanout_crtc_id": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_scanout_primary_plane_id": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_scanout_mode_width": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_scanout_mode_height": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_scanout_mode_refresh_hz": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_scanout_mode_preferred": (true|false)'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_surface_created": true'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_surface_legacy": (true|false)'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_surface_crtc_matches_plan": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_surface_primary_plane_matches_plan": true'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_surface_pending_connector_count": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_surface_current_connector_count": [0-9][0-9]*'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_surface_pending_mode_matches_plan": true'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_surface_commit_pending": (true|false)'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_surface_dropped_after_pause": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_framebuffer_created": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_framebuffer_added": true'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_framebuffer_test_state_succeeded": (true|false)'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_framebuffer_test_state_permission_denied": (true|false)'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_framebuffer_(test_state_succeeded|test_state_permission_denied)": true'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_framebuffer_test_allow_modeset": (true|false)'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_framebuffer_primary_plane_matches_surface": true'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_framebuffer_width": [1-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_framebuffer_height": [1-9][0-9]*'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_framebuffer_released_before_surface_drop": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_first_present_framebuffer_filled": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_first_present_plane_state_ready": true'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_first_present_commit_attempted": (true|false)'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_first_present_commit_succeeded": (true|false)'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_first_present_vblank_event_received": (true|false)'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_first_present_blocked_by_drm_master": (true|false)'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_kms_first_present_(commit_succeeded|blocked_by_drm_master)": true'
if grep -F '"smithay_kms_first_present_commit_succeeded": true' "$host_smithay_runtime_probe_manifest" >/dev/null; then
  require_contains "$host_smithay_runtime_probe_manifest" '"smithay_kms_first_present_vblank_event_received": true'
fi
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_renderer_node_selected": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_renderer_node_opened": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_gbm_device_created": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_gbm_allocator_created": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_egl_display_created": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_egl_context_created": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_gles_renderer_created": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_offscreen_buffer_created": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_offscreen_frame_rendered": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_offscreen_frame_copied": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_offscreen_pixel_verified": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_offscreen_render_width": 16'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_offscreen_render_height": 16'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_offscreen_render_pixels": 256'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_offscreen_sample_red": 255'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_offscreen_sample_green": 0'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_offscreen_sample_blue": 0'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_offscreen_sample_alpha": 255'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libseat_session_created": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libseat_event_source_inserted": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libseat_event_loop_dispatched": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libinput_context_created": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libinput_seat_assigned": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libinput_backend_created": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libinput_event_source_inserted": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libinput_event_loop_dispatched": true'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_libinput_event_count": [0-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_libinput_keyboard_event_count": [0-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_libinput_pointer_event_count": [0-9][0-9]*'
require_matches "$host_smithay_runtime_probe_manifest" '"smithay_libinput_special_event_count": [0-9][0-9]*'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libinput_component": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_libseat_session_component": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_calloop_component": true'
require_contains "$host_smithay_runtime_probe_manifest" '"drm_launch_ready": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_runtime_probe": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_runtime_bootstrap": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_wayland_display_bootstrap": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_wayland_socket_bootstrap": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_wayland_client_inserted": true'
require_contains "$host_smithay_runtime_probe_manifest" '"smithay_calloop_dispatch_bootstrap": true'
require_contains "$host_launch_performance_manifest" '"startup_budget": true'
require_contains "$host_launch_performance_manifest" '"terminal_launch_budget": true'
require_contains "$host_launch_performance_manifest" '"shell_ready_budget": true'
require_contains "$host_resource_budget_manifest" '"name": "backlit-resource-budget"'
require_contains "$host_resource_budget_manifest" '"resource_budget_checked": true'
require_contains "$host_resource_budget_manifest" '"idle_cpu_budget": true'
require_contains "$host_resource_budget_manifest" '"idle_rss_budget": true'
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
require_contains "$host_session_launch_manifest" '"drm_backend_launch_plan": true'
require_contains "$host_session_launch_manifest" '"drm_smithay_launch_plan": true'
require_contains "$host_drm_master_boundary_manifest" '"name": "backlit-drm-master-boundary"'
require_contains "$host_drm_master_boundary_manifest" '"session_entry_drm": true'
require_contains "$host_drm_master_boundary_manifest" '"compositor_service_drm": true'
require_contains "$host_drm_master_boundary_manifest" '"compositor_service_smithay_runtime": true'
require_contains "$host_drm_master_boundary_manifest" '"drm_master_boundary_checked": true'
require_contains "$host_drm_master_boundary_manifest" '"drm_launch_ready": true'
require_contains "$host_drm_master_boundary_manifest" '"first_present_framebuffer_filled": true'
require_contains "$host_drm_master_boundary_manifest" '"first_present_plane_state_ready": true'
require_contains "$host_drm_master_boundary_manifest" '"first_present_blocked_by_drm_master": true'
require_contains "$host_drm_master_boundary_manifest" '"drm_master_boundary_observed": true'
require_contains "$host_drm_master_boundary_manifest" '"dedicated_session_required": true'
require_contains "$host_drm_master_boundary_manifest" '"current_session_can_present": false'
require_contains "$host_drm_master_boundary_manifest" '"mutating_handoff_attempted": false'
require_contains "$host_dedicated_drm_session_manifest" '"name": "backlit-dedicated-drm-session"'
require_contains "$host_dedicated_drm_session_manifest" '"dedicated_handoff_plan": true'
require_contains "$host_dedicated_drm_session_manifest" '"dedicated_handoff_script":'
require_contains "$host_dedicated_drm_session_manifest" '"seat_owner_required": true'
require_contains "$host_dedicated_drm_session_manifest" '"drm_master_present_required": true'
require_contains "$host_dedicated_drm_session_manifest" '"acceptance_checks": "first-present-commit-vblank-gui-services-launch-clean-exit"'
require_contains "$host_dedicated_drm_session_manifest" '"dedicated_handoff_script_checked": true'
require_contains "$host_dedicated_drm_session_manifest" '"dedicated_handoff_seat_owner_required": true'
require_contains "$host_dedicated_drm_session_manifest" '"dedicated_handoff_drm_master_present_required": true'
require_contains "$host_dedicated_drm_session_manifest" '"dedicated_handoff_acceptance_checks": true'
require_contains "$host_dedicated_drm_session_manifest" '"mutating_handoff_attempted": false'
test -x "$host_dedicated_drm_handoff" || {
  echo "Parallels E2E export verification failed: missing executable $host_dedicated_drm_handoff" >&2
  exit 1
}
require_contains "$host_dedicated_drm_session_manifest" '"expected_blocked": true'
require_contains "$host_dedicated_drm_session_manifest" '"reason": "drm-master-unavailable"'
require_contains "$host_dedicated_drm_session_manifest" '"dedicated_session_acceptance": false'
require_contains "$host_dedicated_drm_session_manifest" '"current_session_can_present": false'
require_contains "$host_dedicated_drm_session_manifest" '"dedicated_session_required": true'
require_contains "$host_dedicated_drm_session_manifest" '"first_present_blocked_by_drm_master": true'
require_contains "$host_drm_session_smoke_manifest" '"drm_session_smoke_ready": true'
require_contains "$host_drm_session_smoke_manifest" '"drm_session_clean_exit": true'
require_contains "$host_drm_session_smoke_manifest" '"drm_backend_launch_plan": true'
require_contains "$host_drm_session_smoke_manifest" '"drm_smithay_launch_plan": true'
require_contains "$host_drm_session_smoke_manifest" '"drm_device_selected": true'
require_contains "$host_drm_session_smoke_manifest" '"drm_input_selected": true'
require_contains "$host_drm_session_smoke_manifest" '"session_drm_first_present_probe": true'
require_matches "$host_drm_session_smoke_manifest" '"session_first_present_(commit_succeeded|blocked_by_drm_master)": true'
require_contains "$host_drm_session_smoke_manifest" '"session_compositor_smithay_runtime": true'
require_contains "$host_drm_session_smoke_manifest" '"session_compositor_smithay_protocol_globals": true'
require_contains "$host_drm_session_smoke_manifest" '"session_compositor_smithay_input_sources": true'
require_contains "$host_drm_session_smoke_manifest" '"session_compositor_smithay_input_event_loop": true'
require_contains "$host_drm_session_smoke_manifest" '"session_compositor_smithay_input_seat_handles": true'
require_contains "$host_drm_session_smoke_manifest" '"session_compositor_smithay_input_seat_dispatch": true'
require_contains "$host_drm_session_smoke_manifest" '"session_compositor_demo_client": true'
require_contains "$host_drm_session_smoke_manifest" '"session_compositor_demo_app_id_preserved": true'
require_contains "$host_drm_session_smoke_manifest" '"session_desktop_launch": true'
require_contains "$host_drm_session_smoke_manifest" '"session_desktop_managed_window": true'
require_contains "$host_drm_session_smoke_manifest" '"settings_service": true'
require_contains "$host_drm_session_smoke_manifest" '"notification_service": true'
require_contains "$host_debian_package_build_manifest" '"debs_built": true'
require_contains "$host_debian_package_build_manifest" '"fastgui_core_deb": true'
require_contains "$host_debian_package_build_manifest" '"compositor_smithay_feature_build": true'
require_contains "$host_debian_package_build_manifest" '"session_smithay_feature_build": true'
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
require_contains "$host_nested_wayland_manifest" '"session_wayland_desktop_launch": true'
require_contains "$host_nested_wayland_manifest" '"session_wayland_desktop_managed_window": true'
require_contains "$host_nested_wayland_manifest" '"session_wayland_demo_client": true'
require_contains "$host_nested_wayland_manifest" '"session_wayland_demo_app_id_preserved": true'
require_contains "$host_nested_wayland_manifest" '"session_wayland_clean_exit": true'
require_contains "$host_mvp0_contract_manifest" '"artifact_manifests_checked": true'
require_contains "$host_mvp0_contract_manifest" '"drm_master_boundary": true'
require_contains "$host_mvp1_contract_manifest" '"artifact_manifests_checked": true'
require_contains "$host_mvp1_contract_manifest" '"drm_launch_ready_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"drm_master_boundary_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"dedicated_drm_session_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"drm_session_smoke_ready_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"smithay_compositor_runtime_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"smithay_live_surface_snapshots_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"smithay_real_app_e2e_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"smithay_real_shm_frame_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"resource_budget_contract": true'
require_contains "$host_mvp1_contract_manifest" '"debian_package_install_replay_artifact": true'
require_contains "$host_mvp1_contract_manifest" '"debian_system_install_replay_artifact": true'
require_contains "$health_manifest" '"passed": true'
require_contains "$health_manifest" '"e2e_ready": true'

cat > "$host_out_dir/manifest.json" <<EOF
{
  "name": "backlit-parallels-linux-e2e-export",
  "passed": true,
  "vm": "$vm_name",
  "guest_commit": "$guest_commit",
  "guest_repo": "$repo_dir",
  "guest_e2e_dir": "$e2e_out_dir",
  "artifacts": {
    "parallels_ubuntu_health_manifest": "$health_manifest",
    "guest_manifest": "$host_guest_manifest",
    "gui_smoke_manifest": "$host_gui_smoke_manifest",
    "gui_preview_manifest": "$host_gui_preview_manifest",
    "compositor_runtime_manifest": "$host_compositor_runtime_manifest",
    "smithay_compositor_runtime_manifest": "$host_smithay_compositor_runtime_manifest",
    "smithay_live_surface_snapshots_manifest": "$host_smithay_live_surface_snapshots_manifest",
    "smithay_real_app_e2e_manifest": "$host_smithay_real_app_e2e_manifest",
    "smithay_real_shm_frame_manifest": "$host_smithay_real_shm_frame_manifest",
    "compositor_socket_manifest": "$host_compositor_socket_manifest",
    "smithay_runtime_probe_manifest": "$host_smithay_runtime_probe_manifest",
    "launch_performance_manifest": "$host_launch_performance_manifest",
    "resource_budget_manifest": "$host_resource_budget_manifest",
    "launch_readiness_manifest": "$host_launch_readiness_manifest",
    "session_launch_manifest": "$host_session_launch_manifest",
    "session_replay_manifest": "$host_session_replay_manifest",
    "drm_session_smoke_manifest": "$host_drm_session_smoke_manifest",
    "drm_master_boundary_manifest": "$host_drm_master_boundary_manifest",
    "dedicated_drm_session_manifest": "$host_dedicated_drm_session_manifest",
    "dedicated_drm_handoff": "$host_dedicated_drm_handoff",
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
    "compositor_runtime_preview_image": "$compositor_preview_image",
    "smithay_real_app_e2e_ppm": "$host_smithay_real_app_e2e_ppm",
    "smithay_real_app_e2e_image": "$real_app_e2e_image",
    "smithay_real_shm_frame_ppm": "$host_smithay_real_shm_frame_ppm",
    "smithay_real_shm_frame_image": "$real_shm_frame_image"
  },
  "checks": {
    "parallels_ubuntu_health": true,
    "guest_root_filesystem_writable": true,
    "guest_e2e_passed": true,
    "guest_commit_matches_manifest": true,
    "guest_artifacts_exported": true,
	    "gui_smoke": true,
	    "gui_smoke_session_desktop_managed_window": true,
	    "gui_smoke_demo_client_app_id": true,
	    "gui_preview": true,
    "compositor_runtime": true,
    "compositor_runtime_policy_preview": true,
    "smithay_compositor_runtime": true,
    "smithay_live_surface_snapshots": true,
    "smithay_real_app_e2e": true,
    "real_app_e2e_pixels": true,
    "smithay_real_shm_frame": true,
    "real_shm_frame_pixels": true,
    "compositor_socket": true,
    "smithay_runtime_probe": true,
    "launch_performance": true,
    "resource_budget": true,
    "launch_readiness": true,
    "drm_launch_plan": true,
    "drm_master_boundary": true,
    "dedicated_drm_session": true,
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
    "nested_wayland_desktop_managed_window": true,
    "nested_wayland_demo_client_app_id": true,
    "mvp0_contract": true,
    "mvp1_contract": true,
    "ppm_bytes": $ppm_bytes,
    "png_written": $png_written,
    "preview_format": "$preview_format",
    "converter": "$converter",
    "compositor_preview_ppm_bytes": $compositor_preview_ppm_bytes,
    "compositor_png_written": $compositor_png_written,
    "compositor_preview_format": "$compositor_preview_format",
    "compositor_converter": "$compositor_converter",
    "real_app_e2e_ppm_bytes": $real_app_e2e_ppm_bytes,
    "real_app_e2e_png_written": $real_app_e2e_png_written,
    "real_app_e2e_format": "$real_app_e2e_format",
    "real_app_e2e_converter": "$real_app_e2e_converter",
    "real_shm_frame_ppm_bytes": $real_shm_frame_ppm_bytes,
    "real_shm_frame_png_written": $real_shm_frame_png_written,
    "real_shm_frame_format": "$real_shm_frame_format",
    "real_shm_frame_converter": "$real_shm_frame_converter"
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
if [ "$real_app_e2e_format" = "png" ]; then
  printf 'To view real app E2E preview on macOS: open %s\n' "$real_app_e2e_image"
fi
if [ "$real_shm_frame_format" = "png" ]; then
  printf 'To view real SHM frame preview on macOS: open %s\n' "$real_shm_frame_image"
fi
