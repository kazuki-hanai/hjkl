# Contributing

Thanks for your interest in contributing to `hjkl-for-mac`.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
plutil -lint launchd/com.kazuki-hanai.hjkl-for-mac.plist.template
sh -n scripts/install-launch-agent.sh scripts/uninstall-launch-agent.sh
```

## Pull requests

Please include:

- a short description of the user-visible behavior change,
- tests or a clear manual verification note,
- any macOS permission or LaunchAgent implications.

## Security-sensitive changes

This project installs a per-user LaunchAgent and reads keyboard events through
macOS accessibility APIs. Please keep changes small and explain why new
permissions, background behavior, or file-system writes are necessary.
