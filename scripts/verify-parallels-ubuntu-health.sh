#!/usr/bin/env bash
set -u

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
credential_file="${BACKLIT_PARALLELS_CREDENTIAL_FILE:-$repo_root/.local/parallels-ubuntu.env}"
vm_name="${BACKLIT_PARALLELS_VM:-Ubuntu 22.04.2 ARM64}"
host_out_dir="${1:-${BACKLIT_PARALLELS_HEALTH_OUT_DIR:-target/parallels-ubuntu-health}}"
manifest="$host_out_dir/manifest.json"
probe_log="$host_out_dir/probe.log"
mounts_file="$host_out_dir/proc-mounts.txt"
list_file="$host_out_dir/prlctl-list.txt"
status_file="$host_out_dir/prlctl-status.txt"

mkdir -p "$host_out_dir"
: > "$probe_log"

prlctl_available=false
credential_file_present=false
guest_user_defined=false
vm_registered=false
vm_running=false
guest_root_exec=false
guest_user_exec=false
root_mount_detected=false
root_filesystem_rw_flag=false
root_filesystem_writable=false
tmp_writable=false
passed=false
reason="not-run"
guest_user="${BACKLIT_PARALLELS_UBUNTU_USER:-}"
root_mount=""
root_mount_options=""

json_string() {
  printf '"%s"' "$(printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g')"
}

json_bool() {
  if [ "$1" = true ]; then
    printf 'true'
  else
    printf 'false'
  fi
}

write_manifest() {
  cat > "$manifest" <<EOF
{
  "name": "backlit-parallels-ubuntu-health",
  "passed": $(json_bool "$passed"),
  "reason": $(json_string "$reason"),
  "vm": $(json_string "$vm_name"),
  "guest_user": $(json_string "$guest_user"),
  "credential_file": $(json_string "$credential_file"),
  "root_mount": $(json_string "$root_mount"),
  "root_mount_options": $(json_string "$root_mount_options"),
  "artifacts": {
    "probe_log": $(json_string "$probe_log"),
    "prlctl_list": $(json_string "$list_file"),
    "prlctl_status": $(json_string "$status_file"),
    "proc_mounts": $(json_string "$mounts_file")
  },
  "checks": {
    "prlctl_available": $(json_bool "$prlctl_available"),
    "credential_file_present": $(json_bool "$credential_file_present"),
    "guest_user_defined": $(json_bool "$guest_user_defined"),
    "vm_registered": $(json_bool "$vm_registered"),
    "vm_running": $(json_bool "$vm_running"),
    "guest_root_exec": $(json_bool "$guest_root_exec"),
    "guest_user_exec": $(json_bool "$guest_user_exec"),
    "root_mount_detected": $(json_bool "$root_mount_detected"),
    "root_filesystem_rw_flag": $(json_bool "$root_filesystem_rw_flag"),
    "root_filesystem_writable": $(json_bool "$root_filesystem_writable"),
    "tmp_writable": $(json_bool "$tmp_writable"),
    "e2e_ready": $(json_bool "$passed")
  }
}
EOF
}

run_capture() {
  output_file="$1"
  shift
  printf '$' >> "$probe_log"
  for arg in "$@"; do
    printf ' %s' "$arg" >> "$probe_log"
  done
  printf '\n' >> "$probe_log"

  "$@" > "$output_file" 2>> "$probe_log"
  status="$?"
  if [ -s "$output_file" ]; then
    cat "$output_file" >> "$probe_log"
    printf '\n' >> "$probe_log"
  fi
  printf '[exit %s]\n' "$status" >> "$probe_log"
  return "$status"
}

run_guest_write_probe() {
  probe_name="$1"
  probe_path="$2"
  touch_log="$host_out_dir/${probe_name}-write-touch.txt"
  cleanup_log="$host_out_dir/${probe_name}-write-cleanup.txt"

  if ! run_capture "$touch_log" "$prlctl_bin" exec "$vm_name" --user root touch "$probe_path"; then
    return 1
  fi

  run_capture "$cleanup_log" "$prlctl_bin" exec "$vm_name" --user root rm -f "$probe_path" || true
  return 0
}

prlctl_bin="${PRLCTL:-}"
if [ -z "$prlctl_bin" ]; then
  if command -v prlctl >/dev/null 2>&1; then
    prlctl_bin="$(command -v prlctl)"
  elif [ -x /usr/local/bin/prlctl ]; then
    prlctl_bin="/usr/local/bin/prlctl"
  else
    reason="missing-prlctl"
    write_manifest
    echo "Parallels Ubuntu health check failed: prlctl not found." >&2
    echo "Manifest: $manifest" >&2
    exit 2
  fi
fi
prlctl_available=true

if [ -r "$credential_file" ]; then
  credential_file_present=true
  set -a
  # shellcheck disable=SC1090
  source "$credential_file"
  set +a
  guest_user="${BACKLIT_PARALLELS_UBUNTU_USER:-}"
fi

if [ -n "$guest_user" ]; then
  guest_user_defined=true
fi

if run_capture "$list_file" "$prlctl_bin" list --all --output name,status; then
  if grep -F "$vm_name" "$list_file" >/dev/null 2>&1; then
    vm_registered=true
    if grep -F "$vm_name" "$list_file" | grep -F "running" >/dev/null 2>&1; then
      vm_running=true
    fi
  fi
else
  reason="prlctl-list-failed"
  write_manifest
  echo "Parallels Ubuntu health check failed: unable to list VMs." >&2
  echo "Manifest: $manifest" >&2
  exit 2
fi

run_capture "$status_file" "$prlctl_bin" status "$vm_name" || true

if [ "$vm_registered" != true ]; then
  reason="vm-not-registered"
  write_manifest
  echo "Parallels Ubuntu health check failed: VM not registered: $vm_name" >&2
  echo "Manifest: $manifest" >&2
  exit 2
fi

if [ "$vm_running" != true ]; then
  reason="vm-not-running"
  write_manifest
  echo "Parallels Ubuntu health check failed: VM is not running: $vm_name" >&2
  echo "Manifest: $manifest" >&2
  exit 2
fi

if [ "$credential_file_present" != true ]; then
  reason="missing-credential-file"
  write_manifest
  echo "Parallels Ubuntu health check failed: missing credential file: $credential_file" >&2
  echo "Manifest: $manifest" >&2
  exit 2
fi

if [ "$guest_user_defined" != true ]; then
  reason="missing-guest-user"
  write_manifest
  echo "Parallels Ubuntu health check failed: BACKLIT_PARALLELS_UBUNTU_USER is not set." >&2
  echo "Manifest: $manifest" >&2
  exit 2
fi

if run_capture "$host_out_dir/root-id.txt" "$prlctl_bin" exec "$vm_name" --user root id -u; then
  guest_root_exec=true
fi

if run_capture "$host_out_dir/guest-id.txt" "$prlctl_bin" exec "$vm_name" --user "$guest_user" id -u; then
  guest_user_exec=true
fi

if [ "$guest_root_exec" != true ]; then
  reason="guest-root-exec-failed"
  write_manifest
  echo "Parallels Ubuntu health check failed: cannot execute as root in guest." >&2
  echo "Manifest: $manifest" >&2
  exit 2
fi

if [ "$guest_user_exec" != true ]; then
  reason="guest-user-exec-failed"
  write_manifest
  echo "Parallels Ubuntu health check failed: cannot execute as $guest_user in guest." >&2
  echo "Manifest: $manifest" >&2
  exit 2
fi

if run_capture "$mounts_file" "$prlctl_bin" exec "$vm_name" --user root cat /proc/mounts; then
  root_mount="$(awk '$2 == "/" { print; exit }' "$mounts_file")"
  root_mount_options="$(awk '$2 == "/" { print $4; exit }' "$mounts_file")"
  if [ -n "$root_mount" ]; then
    root_mount_detected=true
    case ",$root_mount_options," in
      *,rw,*) root_filesystem_rw_flag=true ;;
    esac
  fi
fi

root_probe="/root/.backlit-ubuntu-health-write-check-$$"
tmp_probe="/tmp/backlit-ubuntu-health-write-check-$$"

if run_guest_write_probe "root" "$root_probe"; then
  root_filesystem_writable=true
fi

if run_guest_write_probe "tmp" "$tmp_probe"; then
  tmp_writable=true
fi

if [ "$root_mount_detected" != true ]; then
  reason="guest-root-mount-not-detected"
elif [ "$root_filesystem_rw_flag" != true ]; then
  reason="guest-root-read-only"
elif [ "$root_filesystem_writable" != true ]; then
  reason="guest-root-not-writable"
elif [ "$tmp_writable" != true ]; then
  reason="guest-tmp-not-writable"
else
  passed=true
  reason="ok"
fi

write_manifest

if [ "$passed" = true ]; then
  printf 'Parallels Ubuntu health check passed. Manifest: %s\n' "$manifest"
  exit 0
fi

cat >&2 <<EOF
Parallels Ubuntu health check failed: $reason

VM: $vm_name
Root mount: ${root_mount:-unknown}

Restart or repair the Ubuntu VM so its root filesystem mounts read-write before running:
  ./scripts/verify-parallels-linux-e2e.sh
  ./scripts/verify-parallels-dedicated-drm-e2e.sh

Runbook: docs/runbooks/parallels-ubuntu-readonly.md
Manifest: $manifest
EOF
exit 2
