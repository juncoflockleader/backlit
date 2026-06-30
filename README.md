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
cargo run -p backlit-shortcuts -- --verify --list --resolve=Super+Enter
cargo run -p backlit-session-supervisor -- --verify
cargo run -p backlit-clipboard -- --verify
cargo run -p backlit-shell -- --component=all --verify
./scripts/verify-gui-smoke.sh
```

The smoke verifier writes a top-level artifact manifest to `target/gui-smoke/manifest.json`.

The session smoke path also verifies dry-run shortcut interactions: app switching and terminal launch resolution.

See [DEVELOPMENT.md](DEVELOPMENT.md) for environment setup, VM workflow, project layout, and contribution rules.
