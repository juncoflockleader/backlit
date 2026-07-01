#!/usr/bin/env bash
set -u

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

out_dir="${1:-${BACKLIT_PARALLELS_POST_REPAIR_OUT_DIR:-target/parallels-post-repair-readiness}}"
normal_e2e_dir="${BACKLIT_PARALLELS_E2E_HOST_OUT_DIR:-target/linux-e2e-parallels}"
dedicated_e2e_dir="${BACKLIT_PARALLELS_DEDICATED_DRM_HOST_OUT_DIR:-target/parallels-dedicated-drm-e2e}"
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
normal_health_reason="not-run"
dedicated_health_reason="not-run"
normal_health_root_mount=""
dedicated_health_root_mount=""
normal_health_root_mount_options=""
dedicated_health_root_mount_options=""
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

json_string_field() {
  file="$1"
  key="$2"
  if [ ! -f "$file" ]; then
    printf 'missing'
    return 0
  fi
  value="$(sed -n "s/^[[:space:]]*\"$key\": \"\\(.*\\)\"[,]*$/\\1/p" "$file" | sed -n '1p')"
  if [ -n "$value" ]; then
    printf '%s' "$value"
  else
    printf 'unknown'
  fi
}

refresh_health_summary() {
  normal_health_reason="$(json_string_field "$normal_health_dir/manifest.json" reason)"
  dedicated_health_reason="$(json_string_field "$dedicated_health_dir/manifest.json" reason)"
  normal_health_root_mount="$(json_string_field "$normal_health_dir/manifest.json" root_mount)"
  dedicated_health_root_mount="$(json_string_field "$dedicated_health_dir/manifest.json" root_mount)"
  normal_health_root_mount_options="$(json_string_field "$normal_health_dir/manifest.json" root_mount_options)"
  dedicated_health_root_mount_options="$(json_string_field "$dedicated_health_dir/manifest.json" root_mount_options)"
}

write_manifest() {
  cat > "$manifest" <<EOF
{
  "name": "backlit-parallels-post-repair-readiness",
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
  "health": {
    "normal_reason": $(json_string "$normal_health_reason"),
    "dedicated_reason": $(json_string "$dedicated_health_reason"),
    "normal_root_mount": $(json_string "$normal_health_root_mount"),
    "dedicated_root_mount": $(json_string "$dedicated_health_root_mount"),
    "normal_root_mount_options": $(json_string "$normal_health_root_mount_options"),
    "dedicated_root_mount_options": $(json_string "$dedicated_health_root_mount_options")
  },
  "artifacts": {
    "normal_health_manifest": $(json_string "$normal_health_dir/manifest.json"),
    "dedicated_health_manifest": $(json_string "$dedicated_health_dir/manifest.json"),
    "normal_health_log": $(json_string "$out_dir/normal-health.log"),
    "dedicated_health_log": $(json_string "$out_dir/dedicated-health.log")
  },
  "checks": {
    "source_tree_ready": $(json_bool "$source_tree_ready"),
    "normal_health_status": $(status_json "$normal_health_status"),
    "dedicated_health_status": $(status_json "$dedicated_health_status"),
    "ready_for_parallels_mvp_e2e": $(json_bool "$passed")
  }
}
EOF
}

run_health() {
  step="$1"
  dir="$2"
  log_file="$out_dir/$step.log"
  printf 'Running %s health probe...\n' "$step"
  ./scripts/verify-parallels-ubuntu-health.sh "$dir" > "$log_file" 2>&1
  status="$?"
  cat "$log_file"
  printf '%s health exited with %s. Log: %s\n' "$step" "$status" "$log_file"
  return "$status"
}

if [ -n "$upstream" ] && [ "$head_commit" = "$upstream_commit" ]; then
  pushed_commit=true
fi

if [ -n "$(git status --porcelain)" ]; then
  reason="dirty-worktree"
  write_manifest
  echo "Parallels post-repair readiness stopped because the source tree has uncommitted changes." >&2
  echo "Manifest: $manifest" >&2
  exit 2
fi
worktree_clean=true

if [ -z "$upstream" ]; then
  reason="missing-upstream"
  write_manifest
  echo "Parallels post-repair readiness stopped because the current branch has no upstream." >&2
  echo "Manifest: $manifest" >&2
  exit 2
fi

if [ "$head_commit" != "$upstream_commit" ]; then
  reason="unpushed-commit"
  write_manifest
  echo "Parallels post-repair readiness stopped because HEAD does not match its upstream." >&2
  echo "Manifest: $manifest" >&2
  exit 2
fi
source_tree_ready=true

if run_health normal "$normal_health_dir"; then
  normal_health_status=0
else
  normal_health_status="$?"
fi

if run_health dedicated "$dedicated_health_dir"; then
  dedicated_health_status=0
else
  dedicated_health_status="$?"
fi
refresh_health_summary

if [ "$normal_health_status" -ne 0 ] || [ "$dedicated_health_status" -ne 0 ]; then
  reason="parallels-health-failed"
  write_manifest
  cat >&2 <<EOF
Parallels post-repair readiness is not ready for true E2E yet.

Runbook: docs/runbooks/parallels-ubuntu-readonly.md
Manifest: $manifest
EOF
  exit 2
fi

passed=true
reason="ready"
write_manifest

cat <<EOF
Parallels post-repair readiness passed.

Next command:
  ./scripts/verify-parallels-mvp-e2e.sh

Manifest: $manifest
EOF
