#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/launcher-desktop-discovery}"
mkdir -p "$out_dir"

fixture_log="$out_dir/fixture-launcher.jsonl"
host_log="$out_dir/host-launcher.jsonl"

fail() {
  echo "Launcher desktop discovery verification failed: $*" >&2
  exit 1
}

count_default_desktop_files() {
  count=0

  if [ -n "${XDG_DATA_HOME:-}" ]; then
    user_app_dir="$XDG_DATA_HOME/applications"
  elif [ -n "${HOME:-}" ]; then
    user_app_dir="$HOME/.local/share/applications"
  else
    user_app_dir=""
  fi

  if [ -n "$user_app_dir" ] && [ -d "$user_app_dir" ]; then
    count=$((count + $(find "$user_app_dir" -maxdepth 1 -name '*.desktop' -print 2>/dev/null | wc -l | tr -d ' ')))
  fi

  data_dirs="${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
  old_ifs="$IFS"
  IFS=:
  for data_dir in $data_dirs; do
    app_dir="$data_dir/applications"
    if [ -d "$app_dir" ]; then
      count=$((count + $(find "$app_dir" -maxdepth 1 -name '*.desktop' -print 2>/dev/null | wc -l | tr -d ' ')))
    fi
  done
  IFS="$old_ifs"

  printf '%s' "$count"
}

desktop_entry_events() {
  grep -c '"event":"launcher.desktop_entry"' "$1" | tr -d ' '
}

cargo run -p backlit-launcher -- \
  --verify \
  --list \
  --desktop-dir=crates/launcher/fixtures \
  --require-desktop-entries > "$fixture_log"

grep '"event":"launcher.desktop_discovery"' "$fixture_log" >/dev/null
grep '"default_dirs":false' "$fixture_log" >/dev/null
grep '"required":true' "$fixture_log" >/dev/null
grep '"desktop_entries":3' "$fixture_log" >/dev/null
grep '"desktop_dirs":1' "$fixture_log" >/dev/null
fixture_desktop_entries="$(desktop_entry_events "$fixture_log")"
test "$fixture_desktop_entries" -eq 3 || fail "fixture desktop discovery expected 3 entries"

raw_default_desktop_files="$(count_default_desktop_files)"
host_desktop_entries_required=false
if [ "$(uname -s)" = "Linux" ] && [ "$raw_default_desktop_files" -gt 0 ]; then
  host_desktop_entries_required=true
fi

if [ "$host_desktop_entries_required" = true ]; then
  cargo run -p backlit-launcher -- --verify --list --require-desktop-entries > "$host_log"
else
  cargo run -p backlit-launcher -- --verify --list > "$host_log"
fi

grep '"event":"launcher.desktop_discovery"' "$host_log" >/dev/null
grep '"default_dirs":true' "$host_log" >/dev/null
grep '"host_desktop_discovery":true' "$host_log" >/dev/null
host_desktop_entries="$(desktop_entry_events "$host_log")"
if [ "$host_desktop_entries_required" = true ] && [ "$host_desktop_entries" -eq 0 ]; then
  cat "$host_log" >&2
  fail "host has desktop files but launcher discovered no visible desktop entries"
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-launcher-desktop-discovery",
  "passed": true,
  "target_os": "$(uname -s)",
  "artifacts": {
    "fixture_log": "$fixture_log",
    "host_log": "$host_log"
  },
  "checks": {
    "fixture_desktop_discovery": true,
    "fixture_desktop_entries": $fixture_desktop_entries,
    "host_default_desktop_discovery": true,
    "host_raw_desktop_files": $raw_default_desktop_files,
    "host_desktop_entries": $host_desktop_entries,
    "host_desktop_entries_required": $host_desktop_entries_required
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit launcher desktop discovery verification passed. Artifacts: %s\n' "$out_dir"
