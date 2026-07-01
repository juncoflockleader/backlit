#!/usr/bin/env bash
set -u

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

out_dir="${1:-${BACKLIT_PARALLELS_MVP_E2E_OUT_DIR:-target/parallels-mvp-e2e}}"
normal_e2e_dir="${BACKLIT_PARALLELS_E2E_HOST_OUT_DIR:-target/linux-e2e-parallels}"
dedicated_e2e_dir="${BACKLIT_PARALLELS_DEDICATED_DRM_HOST_OUT_DIR:-target/parallels-dedicated-drm-e2e}"
mvp_complete_dir="${BACKLIT_MVP_COMPLETE_OUT_DIR:-target/mvp-complete}"
normal_health_dir="$normal_e2e_dir/parallels-ubuntu-health"
dedicated_health_dir="$dedicated_e2e_dir/parallels-ubuntu-health"
manifest="$out_dir/manifest.json"

mkdir -p "$out_dir"

branch="$(git branch --show-current 2>/dev/null || printf unknown)"
head_commit="$(git rev-parse --short HEAD 2>/dev/null || printf unknown)"
upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || printf '')"
upstream_commit="$(git rev-parse --short '@{u}' 2>/dev/null || printf unknown)"
worktree_clean=false
pushed_commit=false
source_tree_ready=false
normal_health_status=-1
dedicated_health_status=-1
normal_e2e_status=-1
dedicated_e2e_status=-1
mvp_complete_status=-1
passed=false
reason="not-run"

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

status_json() {
  status="$1"
  if [ "$status" -lt 0 ]; then
    printf 'null'
  else
    printf '%s' "$status"
  fi
}

run_step() {
  step="$1"
  shift
  log_file="$out_dir/$step.log"
  printf 'Running %s...\n' "$step"
  "$@" > "$log_file" 2>&1
  status="$?"
  cat "$log_file"
  printf '%s exited with %s. Log: %s\n' "$step" "$status" "$log_file"
  return "$status"
}

write_manifest() {
  cat > "$manifest" <<EOF
{
  "name": "backlit-parallels-mvp-e2e",
  "passed": $(json_bool "$passed"),
  "reason": $(json_string "$reason"),
  "source": {
    "branch": $(json_string "$branch"),
    "upstream": $(json_string "$upstream"),
    "head_commit": $(json_string "$head_commit"),
    "upstream_commit": $(json_string "$upstream_commit"),
    "worktree_clean": $(json_bool "$worktree_clean"),
    "pushed_commit": $(json_bool "$pushed_commit")
  },
  "artifacts": {
    "normal_health_manifest": $(json_string "$normal_health_dir/manifest.json"),
    "dedicated_health_manifest": $(json_string "$dedicated_health_dir/manifest.json"),
    "normal_e2e_manifest": $(json_string "$normal_e2e_dir/manifest.json"),
    "dedicated_e2e_manifest": $(json_string "$dedicated_e2e_dir/manifest.json"),
    "mvp_complete_manifest": $(json_string "$mvp_complete_dir/manifest.json"),
    "normal_health_log": $(json_string "$out_dir/normal-health.log"),
    "dedicated_health_log": $(json_string "$out_dir/dedicated-health.log"),
    "normal_e2e_log": $(json_string "$out_dir/normal-e2e.log"),
    "dedicated_e2e_log": $(json_string "$out_dir/dedicated-e2e.log"),
    "mvp_complete_log": $(json_string "$out_dir/mvp-complete.log")
  },
  "checks": {
    "source_tree_ready": $(json_bool "$source_tree_ready"),
    "normal_health_status": $(status_json "$normal_health_status"),
    "dedicated_health_status": $(status_json "$dedicated_health_status"),
    "normal_e2e_status": $(status_json "$normal_e2e_status"),
    "dedicated_e2e_status": $(status_json "$dedicated_e2e_status"),
    "mvp_complete_status": $(status_json "$mvp_complete_status"),
    "health_preflight": $(json_bool "$health_preflight"),
    "normal_e2e": $(json_bool "$normal_e2e_passed"),
    "dedicated_e2e": $(json_bool "$dedicated_e2e_passed"),
    "mvp_complete": $(json_bool "$mvp_complete_passed")
  }
}
EOF
}

health_preflight=false
normal_e2e_passed=false
dedicated_e2e_passed=false
mvp_complete_passed=false

if [ -n "$upstream" ] && [ "$head_commit" = "$upstream_commit" ]; then
  pushed_commit=true
fi

if [ -n "$(git status --porcelain)" ]; then
  reason="dirty-worktree"
  write_manifest
  cat >&2 <<EOF
Parallels MVP E2E stopped before VM work because the source tree has uncommitted changes.

Commit and push the milestone first, then rerun:
  ./scripts/verify-parallels-mvp-e2e.sh

Manifest: $manifest
EOF
  exit 2
fi
worktree_clean=true

if [ -z "$upstream" ]; then
  reason="missing-upstream"
  write_manifest
  cat >&2 <<EOF
Parallels MVP E2E stopped before VM work because the current branch has no upstream.

Set an upstream and push the branch before running true E2E.
Manifest: $manifest
EOF
  exit 2
fi

if [ "$head_commit" != "$upstream_commit" ]; then
  reason="unpushed-commit"
  write_manifest
  cat >&2 <<EOF
Parallels MVP E2E stopped before VM work because HEAD does not match its upstream.

HEAD: $head_commit
Upstream $upstream: $upstream_commit

Push the current commit before running true E2E.
Manifest: $manifest
EOF
  exit 2
fi
source_tree_ready=true

if run_step normal-health ./scripts/verify-parallels-ubuntu-health.sh "$normal_health_dir"; then
  normal_health_status=0
else
  normal_health_status="$?"
fi

if run_step dedicated-health ./scripts/verify-parallels-ubuntu-health.sh "$dedicated_health_dir"; then
  dedicated_health_status=0
else
  dedicated_health_status="$?"
fi

if [ "$normal_health_status" -ne 0 ] || [ "$dedicated_health_status" -ne 0 ]; then
  reason="parallels-health-failed"
  write_manifest
  cat >&2 <<EOF
Parallels MVP E2E stopped before guest mutation because health preflight failed.

Runbook: docs/runbooks/parallels-ubuntu-readonly.md
Manifest: $manifest
EOF
  exit 2
fi

health_preflight=true

if run_step normal-e2e ./scripts/verify-parallels-linux-e2e.sh "$normal_e2e_dir"; then
  normal_e2e_status=0
  normal_e2e_passed=true
else
  normal_e2e_status="$?"
  reason="normal-parallels-e2e-failed"
  write_manifest
  echo "Parallels MVP E2E failed during normal Linux E2E. Manifest: $manifest" >&2
  exit "$normal_e2e_status"
fi

if run_step dedicated-e2e ./scripts/verify-parallels-dedicated-drm-e2e.sh "$dedicated_e2e_dir"; then
  dedicated_e2e_status=0
  dedicated_e2e_passed=true
else
  dedicated_e2e_status="$?"
  reason="dedicated-parallels-e2e-failed"
  write_manifest
  echo "Parallels MVP E2E failed during dedicated DRM E2E. Manifest: $manifest" >&2
  exit "$dedicated_e2e_status"
fi

if run_step mvp-complete ./scripts/verify-mvp-complete.sh "$mvp_complete_dir" "$normal_e2e_dir" "$dedicated_e2e_dir"; then
  mvp_complete_status=0
  mvp_complete_passed=true
else
  mvp_complete_status="$?"
  reason="mvp-complete-failed"
  write_manifest
  echo "Parallels MVP E2E failed during final MVP complete audit. Manifest: $manifest" >&2
  exit "$mvp_complete_status"
fi

passed=true
reason="complete"
write_manifest

printf 'Backlit Parallels MVP E2E passed. Manifest: %s\n' "$manifest"
