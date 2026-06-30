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
- The Linux E2E manifest includes the launch-readiness manifest.
- Parallels Ubuntu E2E is expected to report `drm_expected_ready: true` and `drm_ready: true`.
