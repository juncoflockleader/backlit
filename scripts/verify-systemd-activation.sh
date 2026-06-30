#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/systemd-activation}"
mkdir -p "$out_dir"

session_log="$out_dir/session.jsonl"
session_err="$out_dir/session.stderr"
fake_systemctl="$out_dir/fake-systemctl"
fake_systemctl_log="$out_dir/fake-systemctl.log"

fail() {
  echo "Systemd activation verification failed: $*" >&2
  exit 1
}

: > "$fake_systemctl_log"
cat > "$fake_systemctl" <<'EOF'
#!/usr/bin/env sh
set -eu

: "${BACKLIT_FAKE_SYSTEMCTL_LOG:?BACKLIT_FAKE_SYSTEMCTL_LOG is required}"
printf '%s\n' "$*" >> "$BACKLIT_FAKE_SYSTEMCTL_LOG"
EOF
chmod 755 "$fake_systemctl"

BACKLIT_FAKE_SYSTEMCTL_LOG="$fake_systemctl_log" cargo run -p backlit-session -- \
  --backend=headless \
  --verify-systemd-units \
  --systemd-unit-dir packaging/systemd \
  --verify-systemd-activation \
  --systemctl-program "$fake_systemctl" > "$session_log" 2> "$session_err"

grep '"event":"session.systemd_units_verified"' "$session_log" >/dev/null
grep '"event":"session.systemd_launch_plan"' "$session_log" >/dev/null
grep '"event":"session.systemd_activation"' "$session_log" >/dev/null
grep '"event":"session.exit"' "$session_log" >/dev/null
grep '"passed":true' "$session_log" >/dev/null
grep '"launch_plan_ready":true' "$session_log" >/dev/null
grep '"stop_after_start":true' "$session_log" >/dev/null
grep '"import_environment_run":true' "$session_log" >/dev/null
grep '"import_environment_exit_success":true' "$session_log" >/dev/null
grep '"start_target_run":true' "$session_log" >/dev/null
grep '"start_target_exit_success":true' "$session_log" >/dev/null
grep '"stop_target_run":true' "$session_log" >/dev/null
grep '"stop_target_exit_success":true' "$session_log" >/dev/null
grep "\"systemctl_program\":\"$fake_systemctl\"" "$session_log" >/dev/null

grep -Fx -- "--user import-environment XDG_RUNTIME_DIR XDG_SESSION_ID XDG_SEAT XDG_SESSION_TYPE WAYLAND_DISPLAY XDG_CURRENT_DESKTOP DESKTOP_SESSION" "$fake_systemctl_log" >/dev/null \
  || fail "missing import-environment command"
grep -Fx -- "--user start backlit-session.target" "$fake_systemctl_log" >/dev/null \
  || fail "missing target start command"
grep -Fx -- "--user stop backlit-session.target" "$fake_systemctl_log" >/dev/null \
  || fail "missing target stop command"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-systemd-activation",
  "passed": true,
  "artifacts": {
    "session_log": "$session_log",
    "session_stderr": "$session_err",
    "fake_systemctl": "$fake_systemctl",
    "fake_systemctl_log": "$fake_systemctl_log"
  },
  "checks": {
    "systemd_units_verified": true,
    "systemd_launch_plan": true,
    "systemd_activation": true,
    "systemd_import_environment": true,
    "systemd_start_target": true,
    "systemd_stop_target": true,
    "activation_smoke_exited": true
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit systemd activation verification passed. Artifacts: %s\n' "$out_dir"
