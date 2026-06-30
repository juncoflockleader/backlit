# MVP 1 Architecture

MVP 1 is the bare graphical session: installable session entry, real launch path, DRM/KMS backend, libinput keyboard and pointer support, Wayland app windows, focus, movement, resize, maximize, fullscreen, terminal hotkey, app switcher, and clean exit.

The first slice is launch readiness. It does not claim the real DRM compositor loop is complete; it makes the runtime contract executable so development and VM testing can prove whether the host is capable of starting that backend.

Current launch-readiness checks:

- Headless backend preflight still succeeds everywhere.
- DRM/KMS preflight requires Linux.
- DRM/KMS preflight requires `XDG_RUNTIME_DIR`.
- DRM/KMS preflight requires at least one `/dev/dri/card*` or `/dev/dri/renderD*` node.
- DRM/KMS preflight requires `/dev/input/event*` devices for the future libinput path.
- DRM/KMS preflight requires `XDG_SESSION_ID` so logind/libseat authorization can be added behind the same contract.
- `scripts/verify-launch-readiness.sh` records whether the host is DRM launch-ready or expected-blocked.
- `backlit-session --preflight-only` verifies backend launch prerequisites through the actual session entrypoint before rendering starts.
- `scripts/verify-session-launch.sh` verifies the desktop session entry, headless session launch, and DRM session launch preflight.
- `backlit-session --verify-launch-spawn` verifies that the `Super+Enter` terminal route can spawn a process with `WAYLAND_DISPLAY` set.
- `scripts/verify-launch-performance.sh` verifies MVP launch budgets from built binaries: GUI ready under 500 ms, terminal hotkey spawn under 300 ms, and shell-ready service probes under 2 seconds.
- `scripts/verify-drm-session-smoke.sh` runs the full DRM session path with GUI verification, launch spawn, and compositor/shell service probes on launch-ready Linux hosts.
- The Linux E2E manifest includes the launch-readiness manifest.
- The Linux E2E manifest includes the session launch manifest.
- The Linux E2E manifest includes the launch-performance manifest.
- The Linux E2E manifest includes the DRM session smoke manifest.
- Parallels Ubuntu E2E is expected to report `drm_expected_ready: true`, `drm_ready: true`, `drm_session_expected_ready: true`, `drm_session_ready: true`, and `drm_session_smoke_ready: true`.
