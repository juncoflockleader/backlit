# Backlit

Backlit is an early-stage fast Linux GUI project: a small Wayland-native desktop session designed to start from Ubuntu Server/headless Ubuntu and grow from a measurable compositor plus shell into a complete desktop environment.

The initial implementation follows the design in [backlit-design.md](backlit-design.md):

- Rust-first compositor work, with Smithay intended for the real Wayland compositor layer.
- A small shell composed of separate clients where practical.
- MVP 0 first: build system, headless/nested development harness, smoke tests, metrics, CI, and packaging skeleton.
- Pure Rust window policy logic that can be tested on macOS without a Linux graphics stack.

## Quick Start

```bash
cargo test --workspace
cargo run -p backlit-compositor -- --backend=headless --smoke-test
cargo run -p backlit-compositor-backend -- --backend=headless --verify
cargo run -p backlit-protocols -- --verify --list
cargo run -p backlit-perf -- --verify
cargo run -p backlit-launcher -- --verify --list --target=terminal --desktop-dir=crates/launcher/fixtures
cargo run -p backlit-launcher -- --verify --target=terminal --spawn-smoke --spawn-program=true --wayland-display=backlit-0
cargo run -p backlit-shortcuts -- --verify --list --resolve=Super+Enter
cargo run -p backlit-input -- --verify
cargo run -p backlit-surface -- --verify
cargo run -p backlit-session-supervisor -- --verify
cargo run -p backlit-clipboard -- --verify
cargo run -p backlit-notification-daemon -- --verify
cargo run -p backlit-settings-daemon -- --verify
cargo run -p backlit-portal-backend -- --verify
cargo run -p backlit-shell -- --component=all --verify
cargo run -p backlit-session -- --backend=headless --screenshot target/backlit-session.ppm --verify --verify-services --verify-clean-exit
./scripts/render-gui-preview.sh
./scripts/render-parallels-gui-preview.sh
./scripts/verify-gui-smoke.sh
./scripts/verify-compositor-runtime.sh
./scripts/verify-compositor-socket.sh
./scripts/verify-launch-performance.sh
./scripts/verify-launcher-desktop-discovery.sh
./scripts/verify-resource-budget.sh
./scripts/verify-notification-daemon.sh
./scripts/verify-settings-daemon.sh
./scripts/verify-portal-security.sh
./scripts/verify-crash-logs.sh
./scripts/verify-ci-contract.sh
./scripts/verify-launch-readiness.sh
./scripts/verify-session-launch.sh
./scripts/verify-session-replay.sh
./scripts/verify-session-clean-exit.sh
./scripts/verify-drm-session-smoke.sh
./scripts/verify-mvp0-contract.sh
./scripts/verify-packaging-contract.sh
./scripts/verify-package-manifests.sh
./scripts/verify-debian-package-build.sh
./scripts/verify-debian-package-install.sh
./scripts/verify-debian-system-install.sh
./scripts/verify-staged-session-install.sh
./scripts/verify-systemd-activation.sh
./scripts/verify-service-lifecycle.sh
./scripts/verify-mvp1-contract.sh
./scripts/verify-linux-e2e.sh
```

The preview renderer writes `target/gui-preview/backlit-session.ppm` and, on macOS, a PNG you can open. The session preview is rendered from `WindowPolicy`, so the smoke path now checks visible workspace windows, focused-window title styling, and the workspace indicator instead of only a static picture. The session replay verifier writes nine interaction frames for app switching, launcher overlay, terminal launch, move, resize, snap, hidden workspace, and switched workspace states. The Parallels preview renderer runs the same path in Ubuntu and copies the artifact back to `target/gui-preview-parallels/`. The smoke verifier writes a top-level artifact manifest to `target/gui-smoke/manifest.json`; its session service probe starts the compositor in bounded service mode and launches `backlit-demo-client` into the compositor socket when the host permits Unix sockets, preserving the demo client's app id in managed window policy. The shell verifier checks wallpaper, panel status indicators, NetworkManager/PipeWire status and control plans, workspace indicator, launcher targets, and app switcher entries; the notification verifier checks D-Bus-style notify, replace, action, persistence, and close-reason behavior; the perf smoke checks render/present time, idle no-redraw behavior, targeted surface damage, drag-frame pacing, and headless direct-scanout eligibility; the compositor-runtime verifier maps scripted app surfaces through the `CompositorRuntime` backend trait, verifies they become managed policy windows, writes a compositor-runtime GUI preview frame, verifies targeted damage and idle no-redraw behavior, closes a surface, and disconnects the client inside bounded service mode; the Smithay compositor runtime verifier runs `backlit-compositor --backend=drm --runtime=smithay --scripted-client`, `--smithay-client-smoke`, plus a bounded `--runtime=smithay --serve` service socket with the optional Smithay backend feature on launch-ready Linux and requires the scripted lifecycle, real Wayland registry/bind/surface/xdg-toplevel configure/ack/commit lifecycle, service-ready bootstrap, demo-client socket lifecycle, core Wayland protocol globals, Wayland display dispatch, and calloop event-loop ticks to pass through the same runtime trait; the compositor-socket verifier proves bounded service mode publishes a Unix socket in `XDG_RUNTIME_DIR`, accepts multiple `backlit-demo-client` windows, maps their announced surfaces and app ids into window policy, focuses the newest app, moves and resizes it through the socket, verifies maximize work-area geometry and fullscreen output geometry, processes a damage event, closes that surface, removes its managed policy window, falls focus back to the older app window, disconnects the closed client, exits cleanly, and removes the socket; the launch-performance verifier enforces MVP startup, service-ready, and terminal-hotkey budgets from built binaries; the launcher discovery verifier checks fixture `.desktop` parsing, quoted `Exec=` argument handling, discovered-entry spawning with `WAYLAND_DISPLAY`, and host freedesktop app discovery from XDG application directories; the session verifier now also resolves and spawns a discovered desktop entry with the session `WAYLAND_DISPLAY`, maps it into managed policy with app id and focus preserved, and packaged install checks prove the installed Settings desktop entry follows that path from the installed `backlit-session`; the nested Wayland verifier launches the real terminal target (`foot`) against a parent Weston socket; the resource-budget verifier checks Linux idle CPU, compositor+shell RSS, and compositor service readiness from bounded idle probes; the service-lifecycle verifier proves compositor/shell/notification/settings services enter `--serve` mode and exit cleanly under `--serve-for-ms`; the settings-daemon verifier checks display, input, and power policy validation; the portal-security verifier checks that direct screenshot, screencast, and remote-desktop capture are denied while consented portal-mediated requests are allowed; the crash-log verifier checks structured supervisor crash records plus systemd user-journal routing for session services; the Smithay runtime probe compiles the optional Smithay backend feature on Linux and verifies DRM/libinput/libseat/calloop linkage plus a Wayland display, listening socket, inserted client, and calloop dispatch bootstrap before launch-ready guests require it; the launch-readiness verifier records whether the current host has the user-owned runtime, active local logind session, accessible DRM card, direct or brokered input state, and backend launch-plan evidence needed for the future DRM backend; the session launch verifier checks that the installed desktop entry runs `backlit-session --backend=drm --activate-systemd`, `backlit-session --preflight-only`, the `backlit-session.target` user target, and the dry-run `systemctl --user` launch plan for compositor/shell/notification/settings services; the systemd activation verifier executes that import/start/stop path against a fake `systemctl`; the package-manifest verifier checks the `fastgui-core` package split and Debian `.install` file ownership map; the Debian package-build verifier builds and inspects real `.deb` artifacts on Linux; the Debian package-install verifier installs the `fastgui-core` package closure into a disposable dpkg root and launches the session plus scripted replay from that installed tree; the Debian system-install verifier is root and opt-in only, installs that closure into the actual Ubuntu dpkg database, launches `/usr/bin/backlit-session`, verifies services, scripted replay, and clean exit, then purges the packages; the session clean-exit verifier checks that requested shutdown closes managed windows and clears focus; the DRM session smoke verifier runs the full DRM session path on launch-ready Linux hosts, verifies the selected DRM/input launch plan, launches the demo client through the session-started compositor socket with app id preserved, launches a discovered desktop entry through the session verifier, requires it to map as a managed focused window, and requires clean session shutdown; the MVP 1 contract verifier collects bare-session readiness evidence without claiming the real compositor loop is finished; the MVP 0 contract verifier checks that the executable harness still covers the design deliverables. CI runs the same Linux E2E gate on GitHub Actions.

When a Parallels Ubuntu VM is available, the full Linux guest verification can be run from macOS with:

```bash
./scripts/verify-parallels-linux-e2e.sh
./scripts/render-parallels-gui-preview.sh
```

The Parallels E2E wrapper also exports a host-side evidence bundle to `target/linux-e2e-parallels/`, including the guest E2E manifest, Smithay compositor runtime manifest, DRM readiness/session smoke manifests, Debian package build/install/system-install manifests, installed-package replay manifests, nested Wayland manifest, MVP 0/MVP 1 contract manifests, and an E2E GUI preview image.

The session smoke path also verifies input and xdg-shell lifecycle interactions: app switching, terminal launch resolution, pointer-driven focus, move/resize routing, workspace switching, left/right window snapping, xdg toplevel map/configure/maximize/fullscreen/close behavior, popup mapping under a parent, clean session shutdown, deterministic terminal spawn with `WAYLAND_DISPLAY` propagation, desktop-entry mapping into managed focused window policy, scripted visual replay frames, and nested Wayland launch of the real terminal target.

See [DEVELOPMENT.md](DEVELOPMENT.md) for environment setup, VM workflow, project layout, and contribution rules.
