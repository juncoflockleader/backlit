# MVP 0 Architecture

MVP 0 is the development harness:

- Cargo workspace.
- Headless compositor entrypoint and backend state model.
- Nested Wayland backend flag.
- Pure window-policy logic.
- Tiny shell stubs.
- Deterministic demo GUI renderer.
- Headless session launch verification.
- Basic MVP protocol smoke registry.
- JSON metrics.
- CI.
- Packaging skeleton.

The real compositor will integrate Smithay behind `backlit-compositor` and keep policy logic testable in `backlit-window-policy`.
