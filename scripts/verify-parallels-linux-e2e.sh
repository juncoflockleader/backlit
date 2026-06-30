#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
credential_file="${BACKLIT_PARALLELS_CREDENTIAL_FILE:-$repo_root/.local/parallels-ubuntu.env}"
vm_name="${BACKLIT_PARALLELS_VM:-Ubuntu 22.04.2 ARM64}"
repo_url="${BACKLIT_E2E_REPO_URL:-https://github.com/juncoflockleader/backlit.git}"
branch="${BACKLIT_E2E_BRANCH:-main}"
e2e_out_dir="${BACKLIT_E2E_OUT_DIR:-target/linux-e2e-parallels}"

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
uploaded_ci_contract_verifier="/tmp/backlit-verify-ci-contract.sh"
uploaded_launch_readiness_verifier="/tmp/backlit-verify-launch-readiness.sh"
uploaded_session_launch_verifier="/tmp/backlit-verify-session-launch.sh"
uploaded_mvp0_contract_verifier="/tmp/backlit-verify-mvp0-contract.sh"
uploaded_packaging_verifier="/tmp/backlit-verify-packaging-contract.sh"
uploaded_staged_install_verifier="/tmp/backlit-verify-staged-session-install.sh"
uploaded_nested_verifier="/tmp/backlit-verify-nested-wayland-smoke.sh"

export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y git ca-certificates build-essential pkg-config curl python3

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
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_ci_contract_verifier" "\$repo_dir/scripts/verify-ci-contract.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_launch_readiness_verifier" "\$repo_dir/scripts/verify-launch-readiness.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_session_launch_verifier" "\$repo_dir/scripts/verify-session-launch.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_mvp0_contract_verifier" "\$repo_dir/scripts/verify-mvp0-contract.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_packaging_verifier" "\$repo_dir/scripts/verify-packaging-contract.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_staged_install_verifier" "\$repo_dir/scripts/verify-staged-session-install.sh"
install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_nested_verifier" "\$repo_dir/scripts/verify-nested-wayland-smoke.sh"

runuser -u "\$guest_user" -- bash -lc "
set -euo pipefail
source \"\\\$HOME/.cargo/env\"
cd \"\$repo_dir\"
scripts/verify-linux-e2e.sh \"\$e2e_out_dir\"
"
EOF
chmod 700 "$root_runner"

printf 'Using Parallels VM: %s\n' "$vm_name"
"$prlctl_bin" list --all | grep -F "$vm_name" >/dev/null

upload_script "$repo_root/scripts/verify-linux-e2e.sh" "/tmp/backlit-verify-linux-e2e.sh"
upload_script "$repo_root/scripts/verify-gui-smoke.sh" "/tmp/backlit-verify-gui-smoke.sh"
upload_script "$repo_root/scripts/render-gui-preview.sh" "/tmp/backlit-render-gui-preview.sh"
upload_script "$repo_root/scripts/verify-ci-contract.sh" "/tmp/backlit-verify-ci-contract.sh"
upload_script "$repo_root/scripts/verify-launch-readiness.sh" "/tmp/backlit-verify-launch-readiness.sh"
upload_script "$repo_root/scripts/verify-session-launch.sh" "/tmp/backlit-verify-session-launch.sh"
upload_script "$repo_root/scripts/verify-mvp0-contract.sh" "/tmp/backlit-verify-mvp0-contract.sh"
upload_script "$repo_root/scripts/verify-packaging-contract.sh" "/tmp/backlit-verify-packaging-contract.sh"
upload_script "$repo_root/scripts/verify-staged-session-install.sh" "/tmp/backlit-verify-staged-session-install.sh"
upload_script "$repo_root/scripts/verify-nested-wayland-smoke.sh" "/tmp/backlit-verify-nested-wayland-smoke.sh"
upload_script "$root_runner" "/tmp/backlit-parallels-root-runner.sh"

"$prlctl_bin" exec "$vm_name" --user root /tmp/backlit-parallels-root-runner.sh
