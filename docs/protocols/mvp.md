# Protocol Tracking

MVP compositor protocol targets from the design:

- `wl_compositor`
- `wl_shm`
- `xdg-shell`
- `xdg-output`
- `viewporter`
- `presentation-time`
- `linux-dmabuf`

Shell surface support should start from a layer-shell-style model for panels, wallpaper, launcher overlays, and notifications.

The current source of truth for protocol smoke coverage is `crates/protocols`. Run:

```bash
cargo run -p backlit-protocols -- --verify --list
```

This verifies that the MVP protocol registry contains the required globals and emits JSON lines for CI artifacts.
