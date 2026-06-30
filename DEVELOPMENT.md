# Backlit Development

Backlit is being scaffolded for MVP 0 from `backlit-design.md`: a testable development harness for a fast Wayland-native Linux desktop.

## Current Direction

- Project identity: `backlit`.
- Rust crate prefix: `backlit-*`.
- Packaging names will follow the design's `fastgui-*` package split later where useful.
- First compositor framework target: Smithay.
- First supported development target: Ubuntu 26.04 LTS.
- Secondary compatibility target: Ubuntu 24.04 LTS.
- macOS is supported for editing, Git, and pure Rust tests only. Real graphics/input validation belongs in Linux VMs and on real Linux hardware.

## Repository Layout

```text
crates/
  common/              Shared config, logging, metrics helpers.
  compositor/          Compositor binary entrypoint.
  compositor-backend/  Backend selection and runtime config.
  window-policy/       Focus, placement, workspaces, snapping; pure logic.
  shell-protocol/      Private shell/compositor protocol model.
  shell/               Shell host stub for panel/launcher/wallpaper work.
  settings-daemon/     Future power/input/display state daemon.
  portal-backend/      Future xdg-desktop-portal backend.
apps/
  settings/            Future settings UI.
  launcher/            Future launcher UI.
  terminal-wrapper/    Future terminal launch wrapper.
packaging/
  debian/              Debian package metadata skeleton.
  sessions/            Desktop/session entries.
  systemd/             User/system service skeletons.
tools/
  perf/                Benchmarks and metrics tooling.
  screenshot-tests/    Golden screenshot test harness.
  hardware-lab/        Hardware matrix and lab scripts.
docs/
  architecture/        Architecture notes.
  protocols/           Protocol support tracking.
  performance/         Budgets and measurement notes.
```

## Host Prerequisites

The current scaffold has no third-party Rust dependencies, so the workspace can build with the local Rust toolchain:

```bash
rustc --version
cargo --version
cargo test --workspace
```

Recommended local tools:

- Rust 1.76 or newer.
- `rustfmt` and `clippy`.
- Git.
- On Linux: Wayland/Mesa/input development packages listed below.

## Ubuntu Development Setup

On Ubuntu 26.04 or 24.04:

```bash
sudo ./scripts/bootstrap-ubuntu.sh
cargo test --workspace
cargo run -p backlit-compositor -- --backend=headless --smoke-test
```

The bootstrap script installs compiler, Wayland, Mesa, libinput, systemd/logind, seat, Weston, Xwayland, and a small terminal for nested testing.

## macOS + Multipass Workflow

Use macOS as the editor and Git host, and use an Ubuntu VM for Linux builds and headless/nested checks.

```bash
brew install --cask multipass
multipass launch 26.04 \
  --name backlit-dev \
  --cpus 6 \
  --memory 12G \
  --disk 80G \
  --cloud-init dev/multipass-cloud-init.yaml
multipass mount "$PWD" backlit-dev:/work/backlit
multipass shell backlit-dev
cd /work/backlit
cargo test --workspace
```

Do not treat VM graphics performance as real product performance. Use it for build checks, protocol smoke tests, nested Wayland behavior, and CI reproduction.

## macOS + Parallels Linux E2E

The Parallels runner bootstraps an Ubuntu guest, updates a clean checkout from `https://github.com/juncoflockleader/backlit.git`, installs a current Rust toolchain with `rustfmt` and `clippy`, and runs the Linux E2E gate:

```bash
./scripts/verify-parallels-linux-e2e.sh
```

The runner reads a local credential file that is ignored by Git:

```bash
mkdir -p .local
cat > .local/parallels-ubuntu.env <<'EOF'
BACKLIT_PARALLELS_UBUNTU_USER=<guest-admin-user>
BACKLIT_PARALLELS_UBUNTU_PASSWORD=<guest-password>
EOF
chmod 600 .local/parallels-ubuntu.env
```

Useful overrides:

```bash
BACKLIT_PARALLELS_VM="Ubuntu 22.04.2 ARM64" ./scripts/verify-parallels-linux-e2e.sh
BACKLIT_E2E_BRANCH=main BACKLIT_E2E_OUT_DIR=target/linux-e2e-parallels ./scripts/verify-parallels-linux-e2e.sh
```

The Linux-side verifier can also be run directly inside any Ubuntu checkout:

```bash
./scripts/verify-linux-e2e.sh
```

It runs `cargo fmt`, workspace tests, `cargo clippy`, the deterministic GUI smoke verifier, and the nested Wayland smoke verifier, then writes `target/linux-e2e/manifest.json`.

## GUI Linux VM Workflow

Inside a GUI Linux VM, start a parent Wayland session:

```bash
dbus-run-session -- weston --socket=parent-wayland
```

In another terminal inside the VM:

```bash
export WAYLAND_DISPLAY=parent-wayland
cargo run -p backlit-compositor -- --backend=wayland --socket=backlit-0 --smoke-test
```

The nested Wayland path can be checked without a visible VM desktop by running Weston's headless backend:

```bash
./scripts/verify-nested-wayland-smoke.sh
```

This starts a temporary parent Weston compositor, verifies the parent socket with `wayland-info` or `weston-info`, then runs Backlit's Wayland backend preflight and compositor smoke path against that socket.

When the real compositor loop lands, launch clients into Backlit with:

```bash
WAYLAND_DISPLAY=backlit-0 foot
```

## Real Hardware Workflow

Use real Intel/AMD Linux hardware before trusting anything about:

- DRM/KMS.
- libinput behavior.
- GPU buffer paths.
- monitor hotplug.
- suspend/resume.
- frame pacing.
- idle CPU and memory.

The Mac remains a good workstation. The Linux machine is the display lab.

## Useful Commands

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo run -p backlit-compositor -- --backend=headless --smoke-test
cargo run -p backlit-compositor-backend -- --backend=headless --verify
cargo run -p backlit-protocols -- --verify --list
cargo run -p backlit-perf -- --verify
cargo run -p backlit-launcher -- --verify --list --target=terminal --desktop-dir=crates/launcher/fixtures
cargo run -p backlit-shortcuts -- --verify --list --resolve=Super+Enter
cargo run -p backlit-session-supervisor -- --verify
cargo run -p backlit-clipboard -- --verify
cargo run -p backlit-session -- --backend=headless --screenshot target/backlit-session.ppm --verify
./scripts/verify-gui-smoke.sh
./scripts/verify-nested-wayland-smoke.sh
./scripts/verify-linux-e2e.sh
cargo run -p backlit-shell -- --component=all --socket=backlit-0 --verify
```

Current compositor flags:

```text
--backend=headless|wayland|drm
--socket=<name>
--smoke-test
--help
```

## GUI Smoke Verification

MVP 0 includes a deterministic headless GUI harness. It does not replace nested Wayland or real DRM/KMS testing, but it proves that the session launch path can create a visible shell preview and verify expected GUI regions in CI.

```bash
./scripts/verify-gui-smoke.sh
```

Artifacts are written to `target/gui-smoke/`:

- `manifest.json`: top-level verification summary.
- `backlit-session.ppm`: deterministic session screenshot.
- `demo-client.ppm`: deterministic demo client screenshot.
- `*.jsonl`: structured launch and verification events.

The verifier also runs `backlit-protocols --verify --list` so MVP protocol coverage stays explicit while the real Smithay compositor is being brought up.

It also runs `backlit-perf --verify`, which measures the deterministic GUI render path and headless backend present path against generous MVP 0 smoke budgets.

The default GUI render is guarded by checksum `5635038614353063225`; update it only when an intentional visual change is made.

The launcher catalog is verified in dry-run mode for the first required targets: terminal, browser, and settings.

The launcher also parses visible `.desktop` entries from a fixture directory in smoke tests, so freedesktop app discovery has a regression path before a full app indexer exists.

Keyboard shortcut routing is also verified in dry-run mode for launcher, terminal, browser, settings, and app-switcher actions.

The session smoke path consumes those dry-run routes too: `Alt+Tab` cycles focus and `Super+Enter` resolves the terminal launch path, then records the resulting window-policy state in `session.jsonl`.

Session verification also checks output-aware geometry: maximized windows use the panel-reserved work area, while fullscreen uses the whole output.

Move and resize behavior is also verified through the session smoke path before maximize/fullscreen checks run.

Minimized windows are kept in policy state but skipped by focus cycling; this is verified in the session smoke path.

Closing the focused window is verified too, including fallback focus that skips minimized windows.

Crash isolation is covered by `backlit-session-supervisor --verify`: shell crashes restart without killing the compositor, while compositor crashes end the session.

Clipboard state is covered by `backlit-clipboard --verify`, which checks text ownership, replacement, clearing, and generation tracking.

Backend preflight can be run directly:

```bash
cargo run -p backlit-compositor-backend -- --backend=headless --verify
cargo run -p backlit-compositor-backend -- --backend=wayland --verify
cargo run -p backlit-compositor-backend -- --backend=drm --verify
```

The Wayland preflight expects `WAYLAND_DISPLAY` and `XDG_RUNTIME_DIR`; the DRM preflight only becomes meaningful inside a real Linux session.

## Engineering Rules

- Keep compositor policy small, measurable, and hard to crash.
- Keep shell clients separate from compositor state where practical.
- Put focus, placement, workspace, and snapping behavior in `backlit-window-policy` so it can be tested without a GPU.
- Keep the headless backend path testable without Wayland so CI can prove client/surface/damage behavior before DRM/KMS work lands.
- Emit structured JSON metrics for startup, backend selection, smoke tests, and future frame timing.
- No feature that touches startup, input, rendering, app launch, or session services should merge without a benchmark or regression check.
- Avoid animations and plugin systems until frame pacing is excellent.
- Prefer standards: Wayland protocols, freedesktop specs, systemd/logind, Mesa, libinput, portals, Debian packaging.

## Git Remote

This workspace is initialized with:

```bash
git remote add origin https://github.com/juncoflockleader/backlit.git
```

Use `main` as the default branch.
