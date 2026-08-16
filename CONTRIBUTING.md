# Contributing

Thanks for your interest in Wisp. This is a small, focused project — please
open an issue or start a discussion before sending a pull request for a new
feature, so we agree on the direction first. Bugs and small fixes are welcome
directly.

## Project layout

- `wisp-core/` — shared tracking logic + database (used by both platforms)
- `wisp-daemon/` — Linux D-Bus service (GNOME)
- `wisp-ui/` — Tauri v2 frontend (React) + Windows-specific hooks
  (`wisp-ui/src-tauri/src/windows_{media,session}.rs`)

## Setup

```sh
# Linux: system dependencies
sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel rpm-build
# (Debian: libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev)

npm ci          # UI deps
cargo build -p wisp-daemon
npm run tauri dev
```

## Testing

```sh
cargo test -p wisp-core    # 100+ unit tests; must stay green
cargo check --target x86_64-pc-windows-msvc -p wisp-ui   # Windows side (see docs/RELEASE.md for the cross-check toolchain)
```

## Pull requests

- Keep changes small and focused; one feature per PR.
- Run `cargo fmt` and `cargo clippy` on your changes.
- Update `docs/RELEASE.md` and the README if the build/install changes.
- CI runs the Windows smoke test on every push — it must stay green.

## Releases

Releases are tagged `v1`, `v2`, … (package versions keep semver). Pushing a
`v*` tag builds the Windows installers and renames them to the unified scheme
(`wisp-<tag>-windows-x86_64[-setup].*`); rename the Linux bundles the same
way and upload everything as release assets (see `docs/RELEASE.md`).