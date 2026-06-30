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
cargo run -p backlit-settings-daemon -- --verify
cargo run -p backlit-portal-backend -- --verify
cargo run -p backlit-shell -- --component=all --verify
cargo run -p backlit-session -- --backend=headless --screenshot target/backlit-session.ppm --verify --verify-services --verify-clean-exit
./scripts/render-gui-preview.sh
./scripts/render-parallels-gui-preview.sh
./scripts/verify-gui-smoke.sh
./scripts/verify-launch-performance.sh
./scripts/verify-resource-budget.sh
./scripts/verify-settings-daemon.sh
./scripts/verify-portal-security.sh
./scripts/verify-ci-contract.sh
./scripts/verify-launch-readiness.sh
./scripts/verify-session-launch.sh
./scripts/verify-session-clean-exit.sh
./scripts/verify-drm-session-smoke.sh
./scripts/verify-mvp0-contract.sh
./scripts/verify-packaging-contract.sh
./scripts/verify-staged-session-install.sh
./scripts/verify-linux-e2e.sh
```

The preview renderer writes `target/gui-preview/backlit-session.ppm` and, on macOS, a PNG you can open. The Parallels preview renderer runs the same path in Ubuntu and copies the artifact back to `target/gui-preview-parallels/`. The smoke verifier writes a top-level artifact manifest to `target/gui-smoke/manifest.json`; the perf smoke checks render/present time, idle no-redraw behavior, targeted surface damage, drag-frame pacing, and headless direct-scanout eligibility; the launch-performance verifier enforces MVP startup, service-ready, and terminal-hotkey budgets from built binaries; the resource-budget verifier checks Linux idle CPU and compositor+shell RSS budgets from bounded idle probes; the settings-daemon verifier checks display, input, and power policy validation; the portal-security verifier checks that direct screenshot, screencast, and remote-desktop capture are denied while consented portal-mediated requests are allowed; the launch-readiness verifier records whether the current host has the runtime, DRM, input, and session state needed for the future DRM backend; the session launch verifier checks the desktop session entry and `backlit-session --preflight-only`; the session clean-exit verifier checks that requested shutdown closes managed windows and clears focus; the DRM session smoke verifier runs the full DRM session path on launch-ready Linux hosts and requires clean session shutdown; the MVP 0 contract verifier checks that the executable harness still covers the design deliverables. CI runs the same Linux E2E gate on GitHub Actions.

When a Parallels Ubuntu VM is available, the full Linux guest verification can be run from macOS with:

```bash
./scripts/verify-parallels-linux-e2e.sh
./scripts/render-parallels-gui-preview.sh
```

The session smoke path also verifies input and toplevel lifecycle interactions: app switching, terminal launch resolution, pointer-driven focus, move/resize routing, xdg-shell-style map/configure/maximize/fullscreen/close behavior, clean session shutdown, and deterministic terminal spawn with `WAYLAND_DISPLAY` propagation.

See [DEVELOPMENT.md](DEVELOPMENT.md) for environment setup, VM workflow, project layout, and contribution rules.
