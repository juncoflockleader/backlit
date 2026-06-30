# MVP 1 Architecture

MVP 1 is the bare graphical session: installable session entry, real launch path, DRM/KMS backend, libinput keyboard and pointer support, Wayland app windows, focus, movement, resize, maximize, fullscreen, terminal hotkey, app switcher, and clean exit.

The first slice is launch readiness. It does not claim the real DRM compositor loop is complete; it makes the runtime contract executable so development and VM testing can prove whether the host is capable of starting that backend.

Current launch-readiness checks:

- Headless backend preflight still succeeds everywhere.
- DRM/KMS preflight requires Linux.
- DRM/KMS preflight requires `XDG_RUNTIME_DIR`.
- DRM/KMS preflight requires `XDG_RUNTIME_DIR` to be owned by the launching user, so VM runners cannot accidentally inherit root's runtime directory.
- DRM/KMS preflight requires at least one `/dev/dri/card*` node with read/write access for mode setting.
- DRM/KMS preflight requires `/dev/input/event*` devices for the future libinput path.
- DRM/KMS preflight requires input readiness through direct input-event access or logind/libseat/libinput brokering.
- DRM/KMS preflight requires `XDG_SESSION_ID` so logind/libseat authorization can be added behind the same contract.
- DRM/KMS preflight requires logind to verify that the session is active, local, seated, and has a concrete session type such as `tty` or `wayland`.
- `scripts/verify-launch-readiness.sh` records whether the host is DRM launch-ready or expected-blocked.
- `backlit-session --preflight-only` verifies backend launch prerequisites through the actual session entrypoint before rendering starts.
- `scripts/verify-session-launch.sh` verifies the desktop session entry launches `backlit-session --backend=drm --activate-systemd`, the headless development launch still works, `backlit-session.target`, dry-run `systemctl --user` launch plan, user systemd unit contract, and DRM session launch preflight. The dry-run import plan includes `XDG_RUNTIME_DIR`, `XDG_SESSION_ID`, `XDG_SEAT`, `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`, `XDG_CURRENT_DESKTOP`, and `DESKTOP_SESSION` so the compositor service receives the same runtime/logind context the DRM preflight checks.
- `scripts/verify-systemd-activation.sh` executes the session launcher's systemd import/start/stop sequence against a fake `systemctl`, proving command order and arguments without mutating the host user manager.
- `backlit-session --verify-launch-spawn` verifies that the `Super+Enter` terminal route can spawn a process with `WAYLAND_DISPLAY` set.
- `scripts/verify-nested-wayland-smoke.sh` verifies that the real terminal target (`foot`) spawns against a parent Weston Wayland socket with `WAYLAND_DISPLAY` set, recording Weston-headless's expected no-seat exit when the terminal cannot become interactive.
- `scripts/verify-launcher-desktop-discovery.sh` verifies fixture `.desktop` parsing and host XDG application discovery for launching installed apps.
- `backlit-input --verify` verifies the policy-level input contract that future libinput events must satisfy: terminal hotkey routing, app-switcher focus changes, pointer focus, title-bar move, resize-grip resize, and clean pointer grab release.
- `backlit-session --verify` verifies workspace switching and left/right window snapping through the same window-policy layer used for focus and movement.
- `backlit-session --verify` renders the preview from `WindowPolicy`, requiring policy windows, visible workspace windows, focused-window title styling, and workspace indicator checks in the generated `session.verified` metrics.
- `backlit-session --scripted-replay-dir` writes verified replay frames for app switching, terminal launch, pointer move, pointer resize, snap, hiding a window on another workspace, and switching to that workspace.
- `backlit-shell --component=all --verify` verifies the current shell chrome contract: wallpaper, panel clock/battery/network/volume indicators, NetworkManager and PipeWire control plans, panel power menu, workspace indicator, launcher targets, app switcher entries, and lock-screen readiness.
- `backlit-surface --verify` verifies the policy-level xdg-shell toplevel and popup contract: create, configure, ack, map into window policy, keep popup focus on the parent, constrain popup geometry, maximize, fullscreen, close request, and clean removal. `backlit-compositor -- --smoke-test` also maps configured xdg toplevel and popup surfaces into the headless compositor backend frame so the compositor entrypoint consumes the same window lifecycle.
- `scripts/verify-compositor-runtime.sh` runs `backlit-compositor --scripted-client --scripted-client-preview ... --serve --serve-for-ms=25`, proving the service runtime can map app-like surfaces into managed policy windows, write a verified compositor-runtime GUI preview frame, avoid idle redraws, process targeted damage, damage the output on surface close, and cleanly remove client surfaces on disconnect.
- `scripts/verify-compositor-socket.sh` runs `backlit-compositor --socket=... --serve --serve-for-ms=500` with a private `XDG_RUNTIME_DIR`, proving the compositor publishes the configured Unix socket, accepts a client connection, and removes the socket on bounded service exit.
- `scripts/verify-package-manifests.sh` verifies the `fastgui-core` bare-session package contract and Debian `.install` file ownership for runtime binaries, session units, desktop entries, portal backend, and development verifiers.
- `scripts/verify-debian-package-build.sh` builds and inspects real `fastgui-*` `.deb` artifacts on Linux so `fastgui-core` is no longer only a control-file promise.
- `scripts/verify-debian-package-install.sh` installs the `fastgui-core` package closure into a disposable dpkg root and runs `backlit-session` plus the scripted session replay from the installed `/usr/bin` tree, proving the package output still launches, verifies the GUI/session path, and writes the launcher/app-switcher replay frames.
- `scripts/verify-debian-system-install.sh` is root and opt-in only; in the Parallels guest it installs the `fastgui-core` package closure into the real dpkg database, runs `/usr/bin/backlit-session --backend=headless --verify --verify-services --verify-clean-exit`, runs the scripted session replay from `/usr/bin/backlit-session`, verifies settings from `/usr/bin`, and purges the packages afterward.
- `backlit-perf --verify` verifies a deterministic 60-frame drag pacing smoke with zero dropped frames and pointer-to-frame p99 under 16 ms.
- `backlit-compositor -- --smoke-test` verifies direct-scanout eligibility policy for opaque fullscreen dmabuf surfaces, including overlay and SHM blockers.
- `backlit-notification-daemon --verify` verifies notification service behavior that the session launches with shell services.
- `backlit-settings --verify` verifies that the launcher settings target resolves to a real app binary and that display, input, and power settings panels apply through settings-daemon policy.
- `backlit-settings-daemon --verify` verifies display, input, power policy state, and dry-run logind/systemd command plans for lock, logout, suspend, reboot, and shutdown.
- `backlit-portal-backend --verify` verifies that direct screenshot, screencast, and remote-desktop capture are denied and only consented portal-mediated flows are allowed.
- `backlit-session-supervisor --verify` and `scripts/verify-crash-logs.sh` verify crash isolation plus user-journal crash records for compositor and shell failures.
- `scripts/verify-launch-performance.sh` verifies MVP launch budgets from built binaries: GUI ready under 500 ms, terminal hotkey spawn under 300 ms, and shell-ready service probes under 2 seconds.
- `scripts/verify-resource-budget.sh` verifies Linux idle resource budgets from bounded compositor and shell probes: the non-smoke compositor path accepts a bootstrap client and presents a bootstrap surface, compositor idle CPU stays under 0.5%, and combined compositor+shell RSS stays under 250 MB.
- `scripts/verify-drm-session-smoke.sh` runs the full DRM session path with GUI verification, launch spawn, compositor/shell/settings service probes, and clean shutdown on launch-ready Linux hosts.
- `scripts/verify-mvp1-contract.sh` collects the MVP 1 readiness evidence into one gate. On non-launch-ready hosts it requires expected-blocked DRM artifacts; inside the Parallels Ubuntu GUI guest it requires launch-ready DRM preflight, DRM session smoke, package-installed replay, system-installed replay, nested Wayland, launch-performance, resource-budget, compositor-runtime, and compositor-socket evidence.
- The Linux E2E manifest includes the launch-readiness manifest.
- The Linux E2E manifest includes the session launch manifest.
- The Linux E2E manifest includes the launch-performance manifest.
- The Linux E2E manifest includes the resource-budget manifest.
- The Linux E2E manifest includes the notification-daemon manifest.
- The Linux E2E manifest includes the settings-daemon manifest.
- The Linux E2E manifest includes the DRM session smoke manifest.
- The Linux E2E manifest includes the MVP 1 contract manifest.
- Parallels Ubuntu E2E maps the active `parallels` logind session before running the guest verifier, runs the opt-in root system-install verifier, exports the guest manifests and GUI preview back to the host, and is expected to report `xdg_runtime_dir_owned_by_user: true`, `session_local: true`, `drm_card_access_ready: true`, `input_broker_ready: true`, `drm_expected_ready: true`, `drm_ready: true`, `drm_session_smoke_ready: true`, `dpkg_root_install: true`, and `actual_system_dpkg_install: true`.
