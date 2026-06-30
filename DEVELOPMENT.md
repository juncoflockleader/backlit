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
cargo run -p backlit-shell -- --component=panel --socket=backlit-0
```

Current compositor flags:

```text
--backend=headless|wayland|drm
--socket=<name>
--smoke-test
--help
```

## Engineering Rules

- Keep compositor policy small, measurable, and hard to crash.
- Keep shell clients separate from compositor state where practical.
- Put focus, placement, workspace, and snapping behavior in `backlit-window-policy` so it can be tested without a GPU.
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

