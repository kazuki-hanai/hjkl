# hjkl-for-mac

A small macOS keyboard remapper written in Rust. It recreates a semicolon-based Karabiner-Elements style layer without requiring Karabiner-Elements at runtime.

`hjkl-for-mac` installs a `hjkl` command and uses macOS `CGEventTap` to observe and rewrite keyboard events.

| Input | Output |
| --- | --- |
| Tap `;` alone | `;` |
| Hold `;` and press `h` | Left Arrow |
| Hold `;` and press `j` | Down Arrow |
| Hold `;` and press `k` | Up Arrow |
| Hold `;` and press `l` | Right Arrow |
| Hold `;` and press any other key | `Command` + that key |

## Requirements

- macOS
- Rust toolchain, only needed to build from source
- macOS Accessibility permission for the installed binary

## Quick start

```sh
scripts/install.sh
```

The installer builds the release binary, copies it to `~/.local/bin/hjkl`, installs a per-user LaunchAgent, and tries to start the service immediately.

Add `~/.local/bin` to your `PATH` if you want to run `hjkl` directly. If it is not on your `PATH`, use the full path instead:

```sh
~/.local/bin/hjkl status
```

After installation, grant Accessibility permission to this binary:

```text
~/.local/bin/hjkl
```

Open:

```text
System Settings -> Privacy & Security -> Accessibility
```

If Input Monitoring lists the binary, allow it there too.

Then restart the service:

```sh
hjkl restart
```

A healthy setup should show:

```text
enabled: yes (auto-start at login)
running: yes
key remapping: active
accessibility: granted
```

## Service commands

The `hjkl` binary manages its own per-user launchd LaunchAgent. You usually do not need to call `launchctl` directly.

```sh
hjkl start      # Start in the background now; do not enable auto-start at login.
hjkl stop       # Stop the background service.
hjkl restart    # Restart the background service.
hjkl enable     # Enable auto-start at login and start now.
hjkl disable    # Disable auto-start and stop now.
hjkl status     # Show service state and file paths.
```

Notes:

- `start` starts the service now, but does not install it for future logins.
- `enable` writes a plist under `~/Library/LaunchAgents`, starts the service now, and makes it auto-start on future logins.
- `disable` removes the LaunchAgent plist and stops the service.
- `stop` only stops the current service. If the service is still enabled, it will start again on the next login.
- The launchd service target is `gui/<uid>/com.kazuki-hanai.hjkl-for-mac`.

## Run in the foreground

For quick manual testing:

```sh
cargo run --release
```

or:

```sh
target/release/hjkl
```

The remapper is active only while the process is running. Press `Ctrl-C` to stop it.

## CLI

```sh
hjkl --help
hjkl --version
```

`start` and `enable` run the remapper in the background through LaunchAgent. Internally, launchd manages a foreground process, so you should not normally run the internal launchd mode by hand.

## macOS permissions

macOS must allow the binary to observe keyboard events. If permission is missing, the process can be loaded by launchd but the remapping will not work.

Grant permission here:

```text
System Settings -> Privacy & Security -> Accessibility
```

Allow this binary:

```text
~/.local/bin/hjkl
```

If you rebuild or reinstall the binary, macOS may still show the old entry as enabled while denying the new binary. If `hjkl status` says `key remapping: not active` even though `accessibility: granted`, remove `~/.local/bin/hjkl` from Accessibility and add it again. Then run:

```sh
hjkl restart
```

If Input Monitoring lists `hjkl`, allow it there too.

## Troubleshooting

### `enabled: yes` and `running: yes`, but keys do not remap

Check:

```sh
hjkl status
```

The important line is:

```text
key remapping: active
```

If it says `not active`, the daemon is loaded but cannot read keyboard events. Re-add `~/.local/bin/hjkl` in Accessibility, allow it in Input Monitoring if present, and run:

```sh
hjkl restart
```

### `hjkl enable` or `hjkl restart` exits with an error

This is intentional. These commands now fail if the service is loaded but key remapping is not actually active. Follow the error message, then run `hjkl restart` again.

### Logs

```text
~/Library/Logs/hjkl.log
~/Library/Logs/hjkl.err.log
```

## Uninstall

```sh
scripts/uninstall.sh                 # Disable auto-start and stop; keep the binary.
scripts/uninstall.sh --remove-binary # Also remove the installed binary.
```

## Relation to the original Karabiner-Elements setup

The original setup treated `;` as `right_command` when used with another key, then mapped `right_command + h/j/k/l` to arrow keys.

This Rust implementation maps `; + h/j/k/l` directly to arrow keys. For other chords, it adds the `Command` modifier, so `; + c` behaves like `Command + c`.

## Power usage

Power usage should be minimal. The implementation is event-driven, not polling-based.

- `CFRunLoopRun()` sleeps until keyboard events arrive.
- The event tap listens only to `keyDown` and `keyUp` events, not high-frequency mouse events.
- Per-key work is limited to reading and rewriting a few event fields.

In practice, it should be comparable to other always-on keyboard remappers.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
sh -n scripts/install.sh scripts/uninstall.sh
```

The LaunchAgent plist is generated by the binary itself in `src/main.rs`. `cargo test` renders the generated plist and validates it with `plutil`, so there is no separate plist template to keep in sync.

## License

MIT
