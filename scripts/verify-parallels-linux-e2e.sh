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
  local payload
  payload="$(base64_one_line "$local_path")"

  "$prlctl_bin" exec "$vm_name" --user root python3 -c \
    "\"import base64,os,pathlib; p=pathlib.Path(\\\"$remote_path\\\"); p.write_bytes(base64.b64decode(\\\"$payload\\\")); os.chmod(p,0o755)\""
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
upload_script "$root_runner" "/tmp/backlit-parallels-root-runner.sh"

"$prlctl_bin" exec "$vm_name" --user root /tmp/backlit-parallels-root-runner.sh
