# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-07-22

### Added
- Windows support through a low-level keyboard hook and `SendInput`, including
  foreground `run`, background `start`, auto-start `enable` via Task Scheduler,
  `stop`/`restart`/`disable`, `status`, and Windows input guidance.
- A platform abstraction layer (`src/platform.rs`) that routes shared CLI and
  keymap behavior to macOS or Windows backends.
- Windows CI checks and Windows release zip packaging.

### Changed
- Layer-key parsing now uses platform-specific key-code tables while keeping
  shared friendly names where possible.
- Project, crate, repository links, service identifiers, and release metadata
  now use the shorter `hjkl` name.
- `; + other` is documented as the platform shortcut modifier: Command on
  macOS and Control on Windows.

## [0.2.1]

### Added
- `--layer-key` to choose the layer ("super") key, by friendly name
  (`semicolon`, `quote`, `grave`, `tab`, …) or macOS key code. Modifier keys
  are supported with left/right distinction (`left_command`/`right_command`,
  `left_option`/`right_option`, `left_control`/`right_control`,
  `left_shift`/`right_shift`). Works with `run`/`start`/`restart`/`enable`,
  is baked into the LaunchAgent, and is shown by `hjkl status`.

### Changed
- Split the monolithic `src/main.rs` into focused modules.
- Manage the toolchain and project commands with [mise](https://mise.jdx.dev)
  (`mise run ci`), used by both local development and CI.

### Security
- Pin all GitHub Actions to commit SHAs and add build-provenance attestation
  to release artifacts (verifiable with `gh attestation verify`).
- Re-render the LaunchAgent plist from trusted state before `launchctl
  bootstrap`, and reject a non-absolute `HOME`.
- Call `/bin/launchctl` by absolute path, return macOS `Boolean` as `u8` to
  avoid undefined behavior, and clear the event-tap pointer before releasing
  it.
- Stop `install.sh --help` from executing a `hjkl` found on `PATH`.

### Fixed
- Keep `Cargo.lock` in sync with the crate version so `--locked` builds pass.

## [0.1.0]

### Added
- Semicolon layer that maps `; + h/j/k/l` to arrow keys and `; + <other>` to `Command + <other>`.
- Self-managed per-user launchd LaunchAgent via `hjkl start|stop|restart|enable|disable|status`.
- Health verification so `enable`/`restart`/`start` fail when key remapping is not actually active.
- Install and uninstall scripts under `scripts/`.
- macOS CI workflow (fmt, clippy, test, build, package, script lint).
- Release workflow that builds and publishes a macOS binary on `v*` tags.

[Unreleased]: https://github.com/kazuki-hanai/hjkl/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/kazuki-hanai/hjkl/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/kazuki-hanai/hjkl/compare/v0.1.0...v0.2.1
[0.1.0]: https://github.com/kazuki-hanai/hjkl/releases/tag/v0.1.0
