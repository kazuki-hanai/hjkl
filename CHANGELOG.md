# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0]

### Added
- Semicolon layer that maps `; + h/j/k/l` to arrow keys and `; + <other>` to `Command + <other>`.
- Self-managed per-user launchd LaunchAgent via `hjkl start|stop|restart|enable|disable|status`.
- Health verification so `enable`/`restart`/`start` fail when key remapping is not actually active.
- Install and uninstall scripts under `scripts/`.
- macOS CI workflow (fmt, clippy, test, build, package, script lint).
- Release workflow that builds and publishes a macOS binary on `v*` tags.

[Unreleased]: https://github.com/kazuki-hanai/hjkl-for-mac/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kazuki-hanai/hjkl-for-mac/releases/tag/v0.1.0
