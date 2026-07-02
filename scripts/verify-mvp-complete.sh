#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/mvp-complete}"
parallels_e2e_dir="${2:-target/linux-e2e-parallels}"
dedicated_e2e_dir="${3:-target/parallels-dedicated-drm-e2e}"
manifest="$out_dir/manifest.json"
parallels_manifest="$parallels_e2e_dir/manifest.json"
parallels_health_manifest="$parallels_e2e_dir/parallels-ubuntu-health/manifest.json"
dedicated_manifest="$dedicated_e2e_dir/manifest.json"
dedicated_health_manifest="$dedicated_e2e_dir/parallels-ubuntu-health/manifest.json"
dedicated_session_manifest="$dedicated_e2e_dir/dedicated-drm-session-manifest.json"
package_build_manifest="$dedicated_e2e_dir/package-build-manifest.json"
dedicated_dpkg_install_log="$dedicated_e2e_dir/system-dpkg-install.log"
dedicated_dpkg_purge_log="$dedicated_e2e_dir/system-dpkg-purge.log"
launch_performance_manifest="$parallels_e2e_dir/launch-performance-manifest.json"
resource_budget_manifest="$parallels_e2e_dir/resource-budget-manifest.json"
live_surface_snapshots_manifest="$parallels_e2e_dir/smithay-live-surface-snapshots-manifest.json"
real_shm_frame_manifest="$parallels_e2e_dir/smithay-real-shm-frame-manifest.json"
parallels_preview="$parallels_e2e_dir/gui-preview-backlit-session.png"
real_shm_frame_preview="$parallels_e2e_dir/smithay-real-shm-frame.png"
dedicated_preview="$dedicated_e2e_dir/dedicated-session.png"
mkdir -p "$out_dir"

commit="$(git rev-parse --short HEAD 2>/dev/null || printf unknown)"
branch="$(git branch --show-current 2>/dev/null || printf unknown)"
upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || printf '')"
upstream_commit="$(git rev-parse --short '@{u}' 2>/dev/null || printf unknown)"

design_scope=false
source_tree_ready=false
worktree_clean=false
pushed_commit=false
normal_parallels_e2e=false
package_installed_dedicated_drm=false
current_commit_evidence=false
gui_launch_verified=false
preview_evidence=false
semantic_gui_evidence=false
mvp1_acceptance=false
launch_performance_evidence=false
resource_budget_evidence=false
parallels_health_evidence=false
live_surface_snapshots_evidence=false
real_shm_frame_evidence=false

write_manifest() {
  passed="$1"
  reason="$2"
  cat > "$manifest" <<EOF
{
  "name": "backlit-mvp-complete",
  "passed": $passed,
  "reason": "$reason",
  "expected_commit": "$commit",
  "source": {
    "branch": "$branch",
    "upstream": "$upstream",
    "upstream_commit": "$upstream_commit",
    "worktree_clean": $worktree_clean,
    "pushed_commit": $pushed_commit
  },
  "artifacts": {
    "parallels_linux_e2e_manifest": "$parallels_manifest",
    "parallels_linux_health_manifest": "$parallels_health_manifest",
    "parallels_launch_performance_manifest": "$launch_performance_manifest",
    "parallels_resource_budget_manifest": "$resource_budget_manifest",
    "parallels_live_surface_snapshots_manifest": "$live_surface_snapshots_manifest",
    "parallels_real_shm_frame_manifest": "$real_shm_frame_manifest",
    "parallels_linux_gui_preview": "$parallels_preview",
    "parallels_real_shm_frame_preview": "$real_shm_frame_preview",
    "parallels_dedicated_drm_manifest": "$dedicated_manifest",
    "parallels_dedicated_health_manifest": "$dedicated_health_manifest",
    "parallels_dedicated_drm_session_manifest": "$dedicated_session_manifest",
    "parallels_dedicated_package_build_manifest": "$package_build_manifest",
    "parallels_dedicated_dpkg_install_log": "$dedicated_dpkg_install_log",
    "parallels_dedicated_dpkg_purge_log": "$dedicated_dpkg_purge_log",
    "parallels_dedicated_gui_preview": "$dedicated_preview"
  },
  "checks": {
    "design_scope": $design_scope,
    "source_tree_ready": $source_tree_ready,
    "normal_parallels_e2e": $normal_parallels_e2e,
    "package_installed_dedicated_drm": $package_installed_dedicated_drm,
    "current_commit_evidence": $current_commit_evidence,
    "gui_launch_verified": $gui_launch_verified,
    "preview_evidence": $preview_evidence,
    "semantic_gui_evidence": $semantic_gui_evidence,
    "mvp1_acceptance": $mvp1_acceptance,
    "launch_performance_evidence": $launch_performance_evidence,
    "resource_budget_evidence": $resource_budget_evidence,
    "parallels_health_evidence": $parallels_health_evidence,
    "live_surface_snapshots_evidence": $live_surface_snapshots_evidence,
    "real_shm_frame_evidence": $real_shm_frame_evidence
  }
}
EOF
}

fail() {
  reason="$1"
  shift
  write_manifest false "$reason"
  echo "MVP complete verification failed: $*" >&2
  echo "Manifest: $manifest" >&2
  exit 1
}

require_file() {
  file="$1"
  reason="$2"
  test -f "$file" || fail "$reason" "missing file $file"
}

require_nonempty_file() {
  file="$1"
  reason="$2"
  test -s "$file" || fail "$reason" "missing or empty file $file"
}

require_png_file() {
  file="$1"
  reason="$2"
  require_nonempty_file "$file" "$reason"
  signature="$(od -An -tx1 -N8 "$file" 2>/dev/null | tr -d ' \n')"
  test "$signature" = "89504e470d0a1a0a" || fail "$reason" "file is not a PNG image: $file"
}

require_contains() {
  file="$1"
  value="$2"
  reason="$3"
  grep -F -- "$value" "$file" >/dev/null || fail "$reason" "missing text in $file: $value"
}

require_file backlit-design.md missing-design
require_file docs/architecture/mvp-1.md missing-mvp1-doc
require_file docs/architecture/real-shm-client-pixels.md missing-real-shm-plan
require_file scripts/verify-linux-e2e.sh missing-linux-e2e
require_file scripts/verify-parallels-linux-e2e.sh missing-parallels-linux-e2e
require_file scripts/verify-parallels-dedicated-drm-e2e.sh missing-parallels-dedicated-drm-e2e
require_file scripts/verify-mvp1-contract.sh missing-mvp1-contract

require_contains backlit-design.md '### MVP 1' design-scope
require_contains backlit-design.md 'Bare graphical session' design-scope
require_contains backlit-design.md 'Ubuntu Server install plus `fastgui-core` package.' design-scope
require_contains backlit-design.md 'Launch a terminal.' design-scope
require_contains backlit-design.md 'Launch a Wayland app.' design-scope
require_contains backlit-design.md 'Move/resize windows smoothly.' design-scope
require_contains backlit-design.md 'Idle CPU and memory hit MVP budget.' design-scope
require_contains docs/architecture/mvp-1.md 'MVP 1 is the bare graphical session' design-scope
require_contains docs/architecture/mvp-1.md 'scripts/verify-parallels-dedicated-drm-e2e.sh' design-scope
require_contains docs/architecture/real-shm-client-pixels.md 'real `wl_shm` pixels in a Backlit-rendered frame' design-scope
require_contains docs/architecture/real-shm-client-pixels.md 'not full GPU texture compositing' design-scope
design_scope=true

if [ -n "$(git status --porcelain)" ]; then
  fail dirty-worktree "worktree has uncommitted changes"
fi
worktree_clean=true

if [ -z "$upstream" ]; then
  fail missing-upstream "current branch has no upstream"
fi
if [ "$commit" != "$upstream_commit" ]; then
  fail unpushed-commit "HEAD $commit does not match upstream $upstream at $upstream_commit"
fi
pushed_commit=true
source_tree_ready=true

require_file "$parallels_health_manifest" missing-parallels-linux-health-manifest
require_file "$dedicated_health_manifest" missing-parallels-dedicated-health-manifest

require_contains "$parallels_health_manifest" '"passed": true' parallels-health-evidence
require_contains "$parallels_health_manifest" '"e2e_ready": true' parallels-health-evidence
require_contains "$parallels_health_manifest" '"root_filesystem_writable": true' parallels-health-evidence
require_contains "$parallels_health_manifest" '"tmp_writable": true' parallels-health-evidence
require_contains "$dedicated_health_manifest" '"passed": true' parallels-health-evidence
require_contains "$dedicated_health_manifest" '"e2e_ready": true' parallels-health-evidence
require_contains "$dedicated_health_manifest" '"root_filesystem_writable": true' parallels-health-evidence
require_contains "$dedicated_health_manifest" '"tmp_writable": true' parallels-health-evidence
parallels_health_evidence=true

require_file "$parallels_manifest" missing-parallels-linux-e2e-manifest
require_file "$dedicated_manifest" missing-parallels-dedicated-drm-manifest
require_file "$launch_performance_manifest" missing-parallels-launch-performance-manifest
require_file "$resource_budget_manifest" missing-parallels-resource-budget-manifest
require_file "$live_surface_snapshots_manifest" missing-parallels-live-surface-snapshots-manifest
require_file "$real_shm_frame_manifest" missing-parallels-real-shm-frame-manifest
require_file "$dedicated_session_manifest" missing-parallels-dedicated-session-manifest
require_file "$package_build_manifest" missing-parallels-dedicated-package-build-manifest
require_file "$dedicated_dpkg_install_log" missing-parallels-dedicated-dpkg-install-log
require_file "$dedicated_dpkg_purge_log" missing-parallels-dedicated-dpkg-purge-log

require_contains "$parallels_manifest" '"passed": true' parallels-linux-e2e
require_contains "$parallels_manifest" "\"guest_commit\": \"$commit\"" current-commit-evidence
require_contains "$parallels_manifest" '"guest_e2e_passed": true' parallels-linux-e2e
require_contains "$parallels_manifest" '"actual_system_dpkg_install": true' parallels-linux-e2e
require_contains "$parallels_manifest" '"debian_system_install_replay": true' parallels-linux-e2e
require_contains "$parallels_manifest" '"nested_wayland": true' parallels-linux-e2e
require_contains "$parallels_manifest" '"drm_session_smoke": true' parallels-linux-e2e
require_contains "$parallels_manifest" '"smithay_live_surface_snapshots": true' parallels-linux-e2e
require_contains "$parallels_manifest" '"smithay_real_shm_frame": true' parallels-linux-e2e
require_contains "$parallels_manifest" '"real_shm_frame_pixels": true' parallels-linux-e2e
require_contains "$parallels_manifest" '"mvp1_contract": true' parallels-linux-e2e
require_contains "$parallels_manifest" '"gui_smoke_session_desktop_managed_window": true' semantic-gui-evidence
require_contains "$parallels_manifest" '"gui_smoke_demo_client_app_id": true' semantic-gui-evidence
require_contains "$parallels_manifest" '"drm_session_desktop_managed_window": true' semantic-gui-evidence
require_contains "$parallels_manifest" '"drm_session_demo_client_app_id": true' semantic-gui-evidence
require_contains "$parallels_manifest" '"debian_package_install_desktop_managed_window": true' semantic-gui-evidence
require_contains "$parallels_manifest" '"debian_package_install_demo_client_app_id": true' semantic-gui-evidence
require_contains "$parallels_manifest" '"debian_system_install_desktop_managed_window": true' semantic-gui-evidence
require_contains "$parallels_manifest" '"debian_system_install_demo_client_app_id": true' semantic-gui-evidence
require_contains "$parallels_manifest" '"nested_wayland_desktop_managed_window": true' semantic-gui-evidence
require_contains "$parallels_manifest" '"nested_wayland_demo_client_app_id": true' semantic-gui-evidence
require_contains "$parallels_manifest" '"launch_performance": true' parallels-linux-e2e
require_contains "$parallels_manifest" '"resource_budget": true' parallels-linux-e2e
require_contains "$parallels_manifest" '"png_written": true' preview-evidence
require_contains "$parallels_manifest" '"preview_format": "png"' preview-evidence
require_contains "$launch_performance_manifest" '"startup_budget": true' launch-performance-evidence
require_contains "$launch_performance_manifest" '"terminal_launch_budget": true' launch-performance-evidence
require_contains "$launch_performance_manifest" '"shell_ready_budget": true' launch-performance-evidence
require_contains "$resource_budget_manifest" '"resource_budget_checked": true' resource-budget-evidence
require_contains "$resource_budget_manifest" '"idle_cpu_budget": true' resource-budget-evidence
require_contains "$resource_budget_manifest" '"idle_rss_budget": true' resource-budget-evidence
require_contains "$live_surface_snapshots_manifest" '"smithay_live_surface_snapshots": true' live-surface-snapshots-evidence
require_contains "$live_surface_snapshots_manifest" '"real_wayland_client": true' live-surface-snapshots-evidence
require_contains "$live_surface_snapshots_manifest" '"live_snapshot_pipeline": true' live-surface-snapshots-evidence
require_contains "$live_surface_snapshots_manifest" '"live_snapshot_persisted": true' live-surface-snapshots-evidence
require_contains "$live_surface_snapshots_manifest" '"live_snapshot_metadata_preserved": true' live-surface-snapshots-evidence
require_contains "$live_surface_snapshots_manifest" '"live_snapshot_pixels_copied": true' live-surface-snapshots-evidence
require_contains "$live_surface_snapshots_manifest" '"live_snapshot_damage_recorded": true' live-surface-snapshots-evidence
require_contains "$live_surface_snapshots_manifest" '"live_snapshot_samples_verified": true' live-surface-snapshots-evidence
require_contains "$live_surface_snapshots_manifest" '"policy_window_from_live_snapshot": true' live-surface-snapshots-evidence
require_contains "$real_shm_frame_manifest" '"smithay_real_shm_frame": true' real-shm-frame-evidence
require_contains "$real_shm_frame_manifest" '"real_wayland_client": true' real-shm-frame-evidence
require_contains "$real_shm_frame_manifest" '"real_wayland_metadata": true' real-shm-frame-evidence
require_contains "$real_shm_frame_manifest" '"real_shm_pixels_captured": true' real-shm-frame-evidence
require_contains "$real_shm_frame_manifest" '"real_shm_pixels_composited": true' real-shm-frame-evidence
require_contains "$real_shm_frame_manifest" '"real_client_pixel_samples_verified": true' real-shm-frame-evidence
require_contains "$real_shm_frame_manifest" '"policy_window_from_real_surface": true' real-shm-frame-evidence
require_contains "$real_shm_frame_manifest" '"frame_ppm_written": true' real-shm-frame-evidence
require_png_file "$parallels_preview" missing-parallels-linux-preview
require_png_file "$real_shm_frame_preview" missing-real-shm-frame-preview
normal_parallels_e2e=true
launch_performance_evidence=true
resource_budget_evidence=true
live_surface_snapshots_evidence=true
real_shm_frame_evidence=true

require_contains "$dedicated_manifest" '"passed": true' package-installed-dedicated-drm
require_contains "$dedicated_manifest" "\"guest_commit\": \"$commit\"" current-commit-evidence
require_contains "$dedicated_manifest" '"system_package_dedicated_drm": true' package-installed-dedicated-drm
require_contains "$dedicated_manifest" '"system_session_binary": true' package-installed-dedicated-drm
require_contains "$dedicated_manifest" '"debs_built": true' package-installed-dedicated-drm
require_contains "$dedicated_manifest" '"dedicated_session_acceptance": true' package-installed-dedicated-drm
require_contains "$dedicated_manifest" '"drm_first_present_commit": true' package-installed-dedicated-drm
require_contains "$dedicated_manifest" '"drm_first_present_vblank": true' package-installed-dedicated-drm
require_contains "$dedicated_manifest" '"session_gui_verified": true' package-installed-dedicated-drm
require_contains "$dedicated_manifest" '"session_services": true' package-installed-dedicated-drm
require_contains "$dedicated_manifest" '"session_clean_exit": true' package-installed-dedicated-drm
require_contains "$dedicated_manifest" '"png_written": true' preview-evidence
require_contains "$dedicated_manifest" '"preview_format": "png"' preview-evidence
require_contains "$package_build_manifest" '"debs_built": true' package-installed-dedicated-drm
require_contains "$dedicated_dpkg_install_log" 'fastgui-core' package-installed-dedicated-drm
require_contains "$dedicated_dpkg_install_log" 'fastgui-session' package-installed-dedicated-drm
require_contains "$dedicated_dpkg_purge_log" 'fastgui-core' package-installed-dedicated-drm
require_contains "$dedicated_dpkg_purge_log" 'fastgui-session' package-installed-dedicated-drm
require_contains "$dedicated_session_manifest" '"expected_blocked": false' package-installed-dedicated-drm
require_contains "$dedicated_session_manifest" '"reason": "dedicated-drm-session-presented"' package-installed-dedicated-drm
require_contains "$dedicated_session_manifest" '"session_binary": "/usr/bin/backlit-session"' package-installed-dedicated-drm
require_contains "$dedicated_session_manifest" '"system_session_binary": true' package-installed-dedicated-drm
require_contains "$dedicated_session_manifest" '"session_desktop_launch": true' package-installed-dedicated-drm
require_contains "$dedicated_session_manifest" '"session_compositor_demo_client": true' package-installed-dedicated-drm
require_contains "$dedicated_session_manifest" '"session_gui_verified": true' semantic-gui-evidence
require_contains "$dedicated_session_manifest" '"session_services": true' semantic-gui-evidence
require_contains "$dedicated_session_manifest" '"session_clean_exit": true' package-installed-dedicated-drm
require_png_file "$dedicated_preview" missing-parallels-dedicated-preview
package_installed_dedicated_drm=true

current_commit_evidence=true
gui_launch_verified=true
preview_evidence=true
semantic_gui_evidence=true
mvp1_acceptance=true
write_manifest true complete

printf 'Backlit MVP complete evidence verification passed. Artifacts: %s\n' "$out_dir"
