#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
credential_file="${BACKLIT_PARALLELS_CREDENTIAL_FILE:-$repo_root/.local/parallels-ubuntu.env}"
vm_name="${BACKLIT_PARALLELS_VM:-Ubuntu 22.04.2 ARM64}"
repo_url="${BACKLIT_E2E_REPO_URL:-https://github.com/juncoflockleader/backlit.git}"
branch="${BACKLIT_E2E_BRANCH:-main}"
host_out_dir="${1:-target/gui-preview-parallels}"
guest_out_dir="${BACKLIT_PARALLELS_GUI_PREVIEW_OUT_DIR:-target/gui-preview-parallels}"

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
  "$prlctl_bin" exec "$vm_name" --user "$guest_user" cat "$remote_path" > "$local_path"
}

remote_file_exists() {
  local remote_path="$1"
  "$prlctl_bin" exec "$vm_name" --user "$guest_user" test -f "$remote_path" >/dev/null 2>&1
}

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/backlit-parallels-preview.XXXXXX")"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

root_runner="$tmp_dir/backlit-parallels-preview-root-runner.sh"
cat > "$root_runner" <<EOF
#!/usr/bin/env bash
set -euo pipefail

guest_user=$(quote_shell "$guest_user")
repo_url=$(quote_shell "$repo_url")
repo_dir=$(quote_shell "$repo_dir")
branch=$(quote_shell "$branch")
guest_out_dir=$(quote_shell "$guest_out_dir")
uploaded_gui_preview_renderer="/tmp/backlit-render-gui-preview.sh"

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

runuser -u "\$guest_user" -- bash -lc '
set -euo pipefail
if [ ! -x "\$HOME/.cargo/bin/rustup" ]; then
  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable
fi
source "\$HOME/.cargo/env"
rustup default stable
'

install -m 0755 -o "\$guest_user" -g "\$guest_user" "\$uploaded_gui_preview_renderer" "\$repo_dir/scripts/render-gui-preview.sh"

runuser -u "\$guest_user" -- bash -lc "
set -euo pipefail
source \"\\\$HOME/.cargo/env\"
cd \"\$repo_dir\"
scripts/render-gui-preview.sh \"\$guest_out_dir\"
"
EOF
chmod 700 "$root_runner"

mkdir -p "$host_out_dir"

printf 'Using Parallels VM: %s\n' "$vm_name"
"$prlctl_bin" list --all | grep -F "$vm_name" >/dev/null

upload_script "$repo_root/scripts/render-gui-preview.sh" "/tmp/backlit-render-gui-preview.sh"
upload_script "$root_runner" "/tmp/backlit-parallels-preview-root-runner.sh"

"$prlctl_bin" exec "$vm_name" --user root /tmp/backlit-parallels-preview-root-runner.sh

guest_commit="$("$prlctl_bin" exec "$vm_name" --user "$guest_user" git -C "$repo_dir" rev-parse --short HEAD | tr -d '\r')"
guest_manifest="$repo_dir/$guest_out_dir/manifest.json"
guest_ppm="$repo_dir/$guest_out_dir/backlit-session.ppm"
guest_png="$repo_dir/$guest_out_dir/backlit-session.png"
guest_session_log="$repo_dir/$guest_out_dir/session.jsonl"

host_guest_manifest="$host_out_dir/guest-manifest.json"
host_session_log="$host_out_dir/session.jsonl"
host_ppm="$host_out_dir/backlit-session.ppm"
host_png="$host_out_dir/backlit-session.png"

download_file "$guest_manifest" "$host_guest_manifest"
download_file "$guest_session_log" "$host_session_log"
download_file "$guest_ppm" "$host_ppm"
if remote_file_exists "$guest_png"; then
  download_file "$guest_png" "$host_png"
fi

preview_image="$host_ppm"
preview_format="ppm"
png_written=false
converter="none"

if [ -s "$host_png" ]; then
  preview_image="$host_png"
  preview_format="png"
  png_written=true
  converter="guest"
elif command -v sips >/dev/null 2>&1; then
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
grep '"passed": true' "$host_guest_manifest" >/dev/null
grep '"session_verified": true' "$host_guest_manifest" >/dev/null
grep '"session_services": true' "$host_guest_manifest" >/dev/null

cat > "$host_out_dir/manifest.json" <<EOF
{
  "name": "backlit-parallels-gui-preview-export",
  "passed": true,
  "vm": "$vm_name",
  "guest_commit": "$guest_commit",
  "guest_repo": "$repo_dir",
  "guest_preview_dir": "$guest_out_dir",
  "artifacts": {
    "guest_manifest": "$host_guest_manifest",
    "session_log": "$host_session_log",
    "session_screenshot_ppm": "$host_ppm",
    "preview_image": "$preview_image"
  },
  "checks": {
    "guest_preview_passed": true,
    "ppm_bytes": $ppm_bytes,
    "png_written": $png_written,
    "preview_format": "$preview_format",
    "converter": "$converter"
  }
}
EOF

printf 'Backlit Parallels GUI preview exported: %s\n' "$preview_image"
printf 'Manifest: %s\n' "$host_out_dir/manifest.json"
if [ "$preview_format" = "png" ]; then
  printf 'To view on macOS: open %s\n' "$preview_image"
else
  printf 'No PNG converter found; view the PPM directly or install ImageMagick/netpbm.\n'
fi
