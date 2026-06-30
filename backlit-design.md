# Fast Linux GUI Design Plan

## 1. Product thesis

Build a **Wayland-native desktop session** on top of **Ubuntu Server / headless Ubuntu**, not a fork of Unity, GNOME Shell, or Xfce. The product should begin as a small, fast compositor plus shell, then grow into a complete desktop environment.

The current stable base should be **Ubuntu 26.04 LTS** for new development, with Ubuntu 24.04 LTS kept as a secondary compatibility target if your hardware or driver matrix demands it. Ubuntu LTS releases get five years of standard security maintenance, and Ubuntu 26.04 LTS was released on April 23, 2026. Source: [Ubuntu release cycle](https://ubuntu.com/about/release-cycle)

The core bet: **own the compositor and shell policy, but do not rebuild the entire Linux app ecosystem.** Use existing Wayland, freedesktop, systemd, Mesa, libinput, portal, and packaging standards where they help. Be ruthless only about the parts that affect perceived speed: boot, input latency, frame scheduling, app launching, memory use, and idle CPU.

---

## 2. Goals

The first version should feel fast before it feels complete.

### Primary goals

| Area | Goal |
|---|---|
| Startup | Get from login/session launch to usable shell quickly. |
| Input | Pointer, keyboard, touchpad, and window focus must feel immediate. |
| Frame pacing | No visible jank for basic window movement, resizing, fullscreen video, or workspace switching. |
| Idle behavior | Near-zero CPU when nothing changes on screen. |
| Memory | Compositor and shell should be small enough to run well on low-end laptops and VMs. |
| Simplicity | Small codebase, measurable behavior, few background daemons. |
| Compatibility | Run normal Linux GUI apps through Wayland first, Xwayland later. |
| Incremental delivery | MVPs should be usable without waiting for a complete desktop suite. |

### Non-goals for the first year

| Non-goal | Reason |
|---|---|
| Replacing Firefox/Chromium | Browser engineering will consume the project. Use existing browsers. |
| Replacing LibreOffice | Same issue. Integrate, do not clone. |
| Supporting every legacy X11 workflow on day one | Xwayland is useful, but native Wayland should be the design center. |
| Heavy visual effects | Blur, complex animations, extension systems, and live wallpapers should not enter the critical path. |
| “GNOME but faster” | That leads to copying historical complexity. Build a smaller model. |

---

## 3. High-level architecture

Recommended stack:

```text
Ubuntu Server / headless Ubuntu LTS
        |
systemd + logind / libseat + udev
        |
FastGUI session launcher
        |
Wayland compositor  <---->  Shell clients
        |                       |
Wayland apps                 panel / launcher / notifications / settings
GTK / Qt / SDL / Electron
        |
Optional compatibility:
Xwayland, xdg-desktop-portal backend, Flatpak/Snap integration
```

Wayland is the correct center for this project. The official Wayland project describes Wayland as the protocol applications use to talk to a display server, and a Wayland server is called a **compositor**. It is positioned as a replacement for the X11 protocol and architecture. Source: [Wayland](https://wayland.freedesktop.org/)

The compositor should be the heart of the product. It owns outputs, input routing, focus, window placement, presentation timing, and privileged desktop protocols. The shell should be a set of ordinary Wayland clients where possible: panel, launcher, wallpaper, notifications, lock screen, and settings UI. Keep policy out of the compositor unless it directly affects correctness or latency.

---

## 4. Core design decision: framework, not raw compositor from scratch

Do **not** start by writing raw DRM/KMS, GBM, EGL, libinput, Wayland protocol, and seat management code from nothing. You will spend too long below the user-visible layer.

Use one of these two paths:

| Option | Recommendation | Why |
|---|---:|---|
| **Rust + Smithay** | Best default if the team likes Rust | Smithay provides building blocks for Wayland compositors, handles much of the low-level Wayland/system interaction, and leaves window management and drawing policy to you. That matches a new desktop project well. Source: [Smithay docs](https://docs.rs/smithay) |
| **C/C++/Zig + wlroots** | Best if you want the fastest path to mature compositor features | wlroots is a modular Wayland compositor library used to avoid writing a large amount of compositor plumbing yourself. Source: [wlroots GitHub](https://github.com/swaywm/wlroots) |

Recommendation: **start with Smithay if Rust is acceptable**. The safety and maintainability payoff is worth it for a new desktop. Keep wlroots as a fallback if protocol support or hardware bring-up becomes a bottleneck.

---

## 5. Compositor design

Working name: `fast-compositor`.

### Responsibilities

#### 5.1 Output management

- Detect monitors.
- Set modes.
- Handle scale, rotation, refresh rate.
- Add multi-monitor layout later.
- Use Linux DRM/KMS for real hardware display control; KMS is the kernel-side mode-setting layer in the Linux graphics stack. Source: [Linux DRM/KMS documentation](https://docs.kernel.org/gpu/drm-kms.html)

#### 5.2 Input management

- Keyboard, pointer, touchpad, touchscreen later.
- Use libinput rather than custom device handling. libinput is the common input stack for mice, keyboards, touchpads, touchscreens, and tablets, and it handles device quirks and event delivery. Source: [libinput documentation](https://wayland.freedesktop.org/libinput/doc/latest/what-is-libinput.html)

#### 5.3 Window management

- Toplevel windows.
- Popups.
- Move, resize, maximize, fullscreen.
- Simple tiling/snap later.
- Workspaces later.

#### 5.4 Wayland protocols

MVP protocols:

- `wl_compositor`
- `wl_shm`
- `xdg-shell`
- `xdg-output`
- `viewporter`
- `presentation-time`
- `linux-dmabuf`

`xdg-shell` is the standard Wayland protocol for desktop-style application windows: drag, resize, maximize, transient windows, popups, and related behavior. Source: [xdg-shell protocol](https://wayland.app/protocols/xdg-shell)

`linux-dmabuf` matters because it lets GPU buffers move between clients and compositor without forcing CPU copies. Source: [Wayland Book: linux-dmabuf](https://wayland-book.com/surfaces/dmabuf.html)

#### 5.5 Rendering

First renderer: EGL/GLES through Mesa.

Later renderer: Vulkan only if profiling proves it helps.

Required optimizations:

- Damage tracking.
- Direct scanout for fullscreen surfaces where possible.
- No redraw when nothing changes.
- No animations in MVP.
- Avoid per-frame heap allocation.

#### 5.6 Crash behavior

- If shell clients crash, compositor stays alive.
- If compositor crashes, session restarts cleanly.
- Logs must be readable from `journalctl`.

#### 5.7 Security

- No arbitrary global keylogging by normal apps.
- No arbitrary screenshot/screencast by normal apps.
- Use portals for screenshot, screencast, file chooser, and remote desktop flows. XDG Desktop Portal is the common framework that lets sandboxed and other apps interact with system services through secure, well-defined APIs. Source: [XDG Desktop Portal documentation](https://flatpak.github.io/xdg-desktop-portal/docs/)

---

## 6. Shell design

Working name: `fast-shell`.

Shell components should be separate processes where practical:

| Component | MVP? | Notes |
|---|---:|---|
| Wallpaper/background | Yes | Simple solid color first, image later. |
| Launcher | Yes | Keyboard-first. Must open terminal, browser, settings. |
| Panel/status bar | Yes | Clock, battery, network, volume, workspace indicator. |
| Notifications | MVP 2 | Use D-Bus notification spec. |
| Lock screen | MVP 2/3 | Security-sensitive; keep small. |
| Settings app | MVP 3 | Display, keyboard, mouse/touchpad, appearance, power. |
| App switcher | MVP 1 | Keyboard shortcut. |
| File manager | Later | Do not block desktop MVP on this. |
| App store/updater | Later | Integrate apt, Flatpak, firmware updates. |

For shell surfaces such as panels, notifications, wallpapers, and overlays, implement a layer-shell-style protocol. The `wlr-layer-shell` protocol is commonly used for desktop shell components such as panels, notifications, and wallpapers. Source: [gtk-layer-shell](https://github.com/wmww/gtk-layer-shell)

Do not put panel rendering, launcher search, file indexing, or notification history inside the compositor. The compositor should be boring, fast, and hard to crash.

---

## 7. Performance budget

Pick a reference machine early, for example:

- Low-end Intel laptop.
- Midrange AMD laptop.
- One NVIDIA machine later.
- One low-resource VM.
- One HiDPI laptop.

Initial targets:

| Metric | MVP target | Stretch target |
|---|---:|---:|
| Compositor startup after session launch | `< 500 ms` | `< 250 ms` |
| Shell ready after session launch | `< 2 s` | `< 1 s` |
| Compositor idle CPU | `< 0.5%` | near zero |
| Compositor + shell idle RSS | `< 250 MB` | `< 150 MB` |
| Pointer-to-frame latency at 60 Hz | `< 16 ms p99` | `< 8 ms p95` |
| Dropped frames during window drag | `< 1%` | near zero |
| Terminal launch after hotkey | `< 300 ms` | `< 150 ms` |
| Fullscreen video | direct scanout when possible | stable zero-copy path |

Hard rule: **no new feature merges without a benchmark or at least a regression check** if it touches compositor, launcher, startup, input, rendering, or session services.

---

## 8. MVP sequence

### MVP 0 — Development harness

Purpose: make the project buildable, testable, and Mac-friendly before real desktop work explodes.

Deliverables:

- Cargo workspace / build system.
- Headless compositor backend for CI.
- Nested Wayland backend for development inside a VM.
- Tiny demo client.
- Performance logging.
- Golden screenshot tests.
- Basic protocol smoke tests.
- Packaging skeleton.

Acceptance criteria:

- `cargo test` / CI passes on Ubuntu.
- Compositor can run headless and accept a test client.
- Compositor can run nested inside another Wayland session.
- Metrics are emitted as JSON.
- No real GPU required for most unit tests.

---

### MVP 1 — Bare graphical session

Purpose: boot into your own session and manage real windows.

Deliverables:

- Ubuntu Server install plus `fastgui-core` package.
- TTY/session launcher.
- Real DRM/KMS backend.
- libinput keyboard and pointer support.
- `xdg-shell` toplevel windows.
- Focus, move, resize, maximize, fullscreen.
- One terminal app launchable by hotkey.
- Basic app switcher.
- Crash logs.

Acceptance criteria:

- On reference hardware: install Ubuntu Server, install your package, start session.
- Launch a terminal.
- Launch a Wayland app.
- Move/resize windows smoothly.
- Exit session cleanly.
- Idle CPU and memory hit MVP budget.

This is the first “real” product milestone.

---

### MVP 2 — Minimal daily desktop

Purpose: make the session usable for simple daily work.

Deliverables:

- Panel.
- Launcher.
- Wallpaper.
- Notification daemon.
- Clipboard.
- Keyboard shortcuts.
- Basic display settings.
- Power menu: lock, logout, reboot, shutdown.
- Network/audio status integration.
- Suspend/resume testing.
- `.desktop` app discovery.

Acceptance criteria:

- User can log in, launch apps, switch apps, use clipboard, connect Wi-Fi through existing system tools, adjust volume, suspend/resume, and shut down.
- No GNOME Shell, Unity, or full desktop environment dependency.
- Shell crash does not kill running apps.

---

### MVP 3 — Compatibility layer

Purpose: make real Linux apps work.

Deliverables:

- Xwayland support.
- Portal backend:
  - Screenshot.
  - Screencast.
  - File chooser.
  - Settings.
  - Open URI.
- Basic Flatpak compatibility.
- Better clipboard and drag-and-drop.
- IME/input method plan.
- Multi-monitor v1.
- HiDPI scaling v1.

Xwayland is important, but keep expectations honest: the Wayland project documentation says Xwayland compatibility with a native X server will probably never reach 100%. Source: [Wayland Book: Xwayland](https://wayland.freedesktop.org/docs/book/Xwayland.html)

Acceptance criteria:

- Firefox/Chromium works.
- VS Code or another Electron app works.
- A GTK app works.
- A Qt app works.
- At least one legacy X11 app works through Xwayland.
- Screen sharing works in at least one browser or conferencing app through the portal path.

---

### MVP 4 — Alpha desktop distribution

Purpose: package the environment like a real installable desktop.

Deliverables:

- Ubuntu Server based install path.
- `fastgui-desktop` meta-package.
- Greeter or minimal login manager integration.
- Settings app v1.
- File manager v1 or selected existing lightweight file manager.
- Terminal selection.
- Text editor selection.
- Image viewer selection.
- Update path.
- Crash reporter.
- Documentation.

Acceptance criteria:

- Fresh install on real hardware.
- Non-developer can log in and perform basic tasks.
- Upgrade does not destroy user config.
- Logs and bug reports are usable.
- Hardware matrix is published.

---

### MVP 5 — Beta-quality desktop

Purpose: stop being a demo.

Deliverables:

- Accessibility plan implemented enough for basic navigation.
- Better multi-monitor.
- Touchpad gestures.
- Fractional scaling if needed.
- Color management plan.
- Keyboard layout UI.
- Bluetooth integration.
- Printer integration.
- Power/battery tuning.
- Theming.
- Installer polish.
- More portal coverage.
- Long-running stability tests.

Acceptance criteria:

- Several team members can daily-drive it.
- No memory growth over long sessions.
- No common compositor crashers.
- Performance remains within budget.
- Known unsupported cases are documented instead of hidden.

---

## 9. First milestone plan

| Milestone | Focus | Output |
|---|---|---|
| M0 | Architecture freeze | Choose Smithay vs wlroots, define performance budget, create repo, define coding standards. |
| M1 | Dev harness | Headless backend, nested backend, CI, test client, metrics. |
| M2 | First windows | `xdg-shell`, draw app surfaces, focus, move, resize. |
| M3 | Real hardware | DRM/KMS backend, libinput, session launch from Ubuntu Server. |
| M4 | Minimal shell | Launcher, panel, hotkeys, wallpaper, app discovery. |
| M5 | Usability | Clipboard, notifications, power menu, basic display settings. |
| M6 | Compatibility | Xwayland, portal backend, Flatpak smoke tests. |
| M7 | Alpha packaging | Debian packages, meta-package, install docs, hardware matrix. |
| M8 | Performance hardening | Benchmark dashboard, frame pacing, startup trimming, idle tuning. |

Sequencing principle: **window management before shell polish, shell before app suite, app compatibility before custom apps.**

---

## 10. Suggested repo layout

```text
fastgui/
  crates/
    compositor/          # Wayland compositor core
    compositor-backend/  # DRM/KMS, nested Wayland, headless backends
    window-policy/       # Focus, placement, workspaces; pure logic
    shell-protocol/      # Private shell/compositor IPC
    shell/               # Panel, launcher, notifications host
    settings-daemon/     # Power, input, display state
    portal-backend/      # xdg-desktop-portal implementation
    common/              # Config, logging, metrics
  apps/
    settings/
    launcher/
    terminal-wrapper/
  packaging/
    debian/
    systemd/
    sessions/
  tools/
    perf/
    screenshot-tests/
    hardware-lab/
  docs/
    architecture/
    protocols/
    performance/
```

Important split: keep `window-policy` mostly platform-independent. That lets you test focus, placement, workspaces, snapping, and keyboard behavior on macOS without booting the compositor.

---

## 11. Packaging model

Ship as packages first, ISO later.

Initial packages:

| Package | Contents |
|---|---|
| `fastgui-compositor` | compositor binary, protocol XML, defaults |
| `fastgui-shell` | panel, launcher, notifications |
| `fastgui-session` | session launcher, systemd units, `.desktop` session file |
| `fastgui-portal` | xdg-desktop-portal backend |
| `fastgui-settings` | settings UI and daemon |
| `fastgui-desktop` | meta-package depending on the above |
| `fastgui-dev-tools` | debug tools, protocol tracing, benchmarks |

Start from Ubuntu Server/headless Ubuntu, then install:

```bash
sudo apt install fastgui-desktop
```

Later, build an installer image. Do not make the ISO the core product too early; it will distract from compositor quality.

---

## 12. Development on macOS

Development on macOS is possible, but with a hard limitation: **macOS cannot validate the real Linux graphics/input path.** You can write code, build in Linux VMs, run nested/headless tests, and do visual smoke testing in a VM. Final performance and hardware validation must happen on real Linux machines.

Apple’s Virtualization framework supports creating and managing Linux VMs on Apple silicon and Intel Macs, and Apple also documents running GUI Linux in a VM on a Mac. Sources: [Apple Virtualization framework](https://developer.apple.com/documentation/virtualization), [Running GUI Linux in a VM on Mac](https://developer.apple.com/documentation/virtualization/running-gui-linux-in-a-virtual-machine-on-a-mac)

Multipass is also a good CLI path for quick Ubuntu VMs on macOS, and it supports cloud-init customization. Source: [Canonical Multipass](https://canonical.com/multipass)

### Recommended Mac workflow

Use three environments:

| Environment | Purpose |
|---|---|
| macOS host | Editor, Git, design docs, pure Rust unit tests. |
| Ubuntu VM | Linux builds, headless tests, nested compositor tests. |
| Real Linux hardware | DRM/KMS, libinput, GPU, latency, suspend/resume, multi-monitor. |

### Option A: Multipass for builds and headless tests

On macOS:

```bash
brew install --cask multipass
```

Create `cloud-init.yaml`:

```yaml
#cloud-config
package_update: true
packages:
  - build-essential
  - git
  - pkg-config
  - clang
  - lld
  - cmake
  - meson
  - ninja-build
  - rustc
  - cargo
  - libwayland-dev
  - wayland-protocols
  - libxkbcommon-dev
  - libinput-dev
  - libudev-dev
  - libsystemd-dev
  - libseat-dev
  - libgbm-dev
  - libegl1-mesa-dev
  - libgles2-mesa-dev
  - libdrm-dev
  - mesa-utils
  - weston
  - xwayland
  - foot
```

Launch the VM:

```bash
multipass launch 26.04 \
  --name fastgui-dev \
  --cpus 6 \
  --memory 12G \
  --disk 80G \
  --cloud-init cloud-init.yaml
```

Mount your repo:

```bash
multipass mount "$PWD" fastgui-dev:/work/fastgui
multipass shell fastgui-dev
cd /work/fastgui
cargo test --workspace
```

Use this for:

- compiler checks;
- unit tests;
- protocol tests;
- headless compositor tests;
- packaging tests;
- CI reproduction.

Do **not** trust this for final graphics performance.

### Option B: GUI Linux VM for visual nested testing

Use UTM, Parallels, VMware Fusion, or a small custom Virtualization.framework wrapper.

Inside the GUI Linux VM:

```bash
sudo apt update
sudo apt install \
  build-essential git pkg-config rustc cargo \
  libwayland-dev wayland-protocols libxkbcommon-dev \
  libinput-dev libudev-dev libsystemd-dev libseat-dev \
  libgbm-dev libegl1-mesa-dev libgles2-mesa-dev libdrm-dev \
  weston foot
```

Run a parent Wayland session, then run your compositor nested inside it:

```bash
dbus-run-session -- weston --socket=parent-wayland
```

In another terminal inside the VM:

```bash
export WAYLAND_DISPLAY=parent-wayland
cargo run -p fastgui-compositor -- --backend=wayland --socket=fastgui-0
```

Then launch a client into your compositor:

```bash
WAYLAND_DISPLAY=fastgui-0 foot
```

This lets you debug window management and shell behavior without owning the physical GPU.

### Option C: real Linux dev box for hardware work

Buy or provision one small Intel/AMD mini PC or laptop early. The Mac remains your workstation; the Linux box is your display lab.

Use:

```bash
ssh devbox
cd ~/fastgui
cargo run -p fastgui-compositor -- --backend=drm
```

For serious testing, boot into a clean TTY/session and run your compositor as the active session, not nested. This is where you validate:

- DRM/KMS;
- libinput;
- real touchpad behavior;
- monitor hotplug;
- suspend/resume;
- GPU buffer paths;
- frame pacing;
- power consumption;
- NVIDIA/AMD/Intel differences.

### Apple Silicon warning

On Apple Silicon, the VM architecture is ARM64 unless you deliberately emulate x86_64. Apple’s Linux VM documentation notes that the Linux ISO must support the Mac’s CPU architecture. Source: [Running GUI Linux in a VM on Mac](https://developer.apple.com/documentation/virtualization/running-gui-linux-in-a-virtual-machine-on-a-mac)

If your release target is x86_64 PCs, you need either:

- x86_64 CI;
- an x86_64 Linux dev box;
- an x86_64 VM/emulator for build checks, not performance;
- cross-compilation only for non-hardware-dependent crates.

---

## 13. Early engineering rules

Use these from day one:

1. **No compositor plugins in MVP.** Plugins are where latency and crashes hide.
2. **No animation framework until frame pacing is excellent.**
3. **No feature without an owner and a benchmark.**
4. **Keep shell crash separate from compositor crash.**
5. **Keep app compatibility standards-based.**
6. **Use existing apps before writing replacements.**
7. **Treat VM performance as fake.**
8. **Make debug logs good early.**
9. **Maintain a hardware matrix.**
10. **Cut scope aggressively.**

---

## 14. Biggest risks

| Risk | Why it matters | Mitigation |
|---|---|---|
| Wayland protocol gaps | Screenshare, global shortcuts, IME, accessibility, and color management can get messy. | Implement portals early, track protocol support explicitly. |
| GPU diversity | Intel, AMD, NVIDIA, hybrid graphics, and VMs behave differently. | Start with Intel/AMD Mesa, add NVIDIA after MVP 2. |
| Accessibility debt | Hard to bolt on later. | Design settings, focus, keyboard navigation, and assistive hooks early. |
| App compatibility expectations | Users expect browsers, Electron, GTK, Qt, Flatpak, and some X11 apps. | MVP 3 must be compatibility-focused. |
| Shell creep | Panels, launchers, settings, and app stores can balloon. | Keep compositor and shell minimal; ship selected existing apps first. |
| “Fast” becoming subjective | People will argue about feel. | Define budgets and publish measurements. |

---

## 15. Best first target

The first credible public alpha should be:

> A headless-Ubuntu-based Wayland desktop that boots into our compositor, launches common Linux apps, uses less memory than mainstream desktops, has low idle CPU, supports basic settings, and is stable enough for developers to use.

That is ambitious but reachable. A full GUI suite can come later. The first win is not having a file manager or app store. The first win is this:

```text
Ubuntu Server -> login -> your compositor -> launcher -> terminal/browser
```

and it feels instant.
