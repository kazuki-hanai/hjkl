# hjkl

A small macOS and Windows keyboard remapper written in Rust. It recreates a semicolon-based Karabiner-Elements style layer without requiring Karabiner-Elements at runtime.

`hjkl` installs a `hjkl` command. On macOS it uses `CGEventTap` to observe and rewrite keyboard events. On Windows it uses a low-level keyboard hook plus `SendInput`.

| Input | Output |
| --- | --- |
| Tap `;` alone | `;` |
| Hold `;` and press `h` | Left Arrow |
| Hold `;` and press `j` | Down Arrow |
| Hold `;` and press `k` | Up Arrow |
| Hold `;` and press `l` | Right Arrow |
| Hold `;` and press any other key | `Command` + that key on macOS, `Control` + that key on Windows |

## Requirements

- macOS or Windows
- Rust toolchain, only needed to build from source
- macOS Accessibility permission for the installed binary
- On Windows, run `hjkl` as administrator only if you need to remap administrator/elevated applications

## Quick start

### macOS

```sh
scripts/install.sh
```

The macOS installer builds the release binary, copies it to `~/.local/bin/hjkl`, installs a per-user LaunchAgent, and tries to start the service immediately.

### Windows

Build from source:

```powershell
cargo build --release
```

Then put `target\release\hjkl.exe` somewhere on your `PATH` and enable it:

```powershell
hjkl enable
```

`enable` creates a per-user startup entry that starts `hjkl` at login and starts the remapper immediately. To run without auto-start:

```powershell
hjkl start
```

### Prebuilt binary

Tagged releases publish binaries on the
[Releases page](https://github.com/kazuki-hanai/hjkl/releases):

- macOS: `hjkl-<version>-macos-<arch>.tar.gz`
- Windows: `hjkl-<version>-windows-x86_64.zip`

Download the archive together with its `.sha256` file, then verify and extract it.

macOS:

```sh
# Confirm the archive matches the published checksum.
shasum -a 256 -c hjkl-<version>-macos-<arch>.tar.gz.sha256

# Optional but stronger: verify the build provenance attestation signed by
# the release workflow (requires the GitHub CLI).
gh attestation verify hjkl-<version>-macos-<arch>.tar.gz \
  --repo kazuki-hanai/hjkl

tar -xzf hjkl-<version>-macos-<arch>.tar.gz
```

Windows PowerShell:

```powershell
Get-FileHash .\hjkl-<version>-windows-x86_64.zip -Algorithm SHA256

# Optional but stronger: verify the build provenance attestation signed by
# the release workflow (requires the GitHub CLI).
gh attestation verify .\hjkl-<version>-windows-x86_64.zip `
  --repo kazuki-hanai/hjkl

Expand-Archive .\hjkl-<version>-windows-x86_64.zip
```

Place the extracted `hjkl`/`hjkl.exe` on your `PATH` (for example in `~/.local/bin` on macOS, or a tools directory on Windows). The macOS binary is not code-signed or notarized, so on first launch Gatekeeper will warn; open it once via **System Settings -> Privacy & Security -> Open Anyway** rather than disabling Gatekeeper or clearing quarantine globally. You still need to grant Accessibility permission on macOS as described below.

> **Note:** the `.sha256` file only proves the archive was not corrupted in
> transit — it is published alongside the artifact, so it does not by itself
> prove authenticity. The `gh attestation verify` step above is what ties the
> binary to this repository's release workflow.

Add `~/.local/bin` to your `PATH` if you want to run `hjkl` directly. If it is not on your `PATH`, use the full path instead:

```sh
~/.local/bin/hjkl status
```

On macOS, after installation, grant Accessibility permission to this binary:

```text
~/.local/bin/hjkl
```

You can ask macOS to show the permission prompt with:

```sh
hjkl permissions
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

A healthy macOS setup should show:

```text
enabled: yes (auto-start at login)
running: yes
key remapping: active
accessibility: granted
```

## Service commands

The `hjkl` binary manages its own per-user background process:

- macOS: launchd LaunchAgent
- Windows: per-user Startup folder script

You usually do not need to call `launchctl` or edit startup entries directly.

```sh
hjkl start      # Start in the background now; do not enable auto-start at login.
hjkl stop       # Stop the background service.
hjkl restart    # Restart the background service.
hjkl enable     # Enable auto-start at login and start now.
hjkl disable    # Disable auto-start and stop now.
hjkl status     # Show service state and file paths.
hjkl permissions # Show platform-specific input permission guidance.
```

Notes:

- `start` starts the service now, but does not install it for future logins.
- `enable` writes a plist under `~/Library/LaunchAgents` on macOS or a Startup folder script on Windows, starts the service now, and makes it auto-start on future logins.
- `disable` removes that auto-start entry and stops the service.
- `stop` only stops the current service. If the service is still enabled, it will start again on the next login.
- The macOS launchd service target is `gui/<uid>/com.kazuki-hanai.hjkl`.
- The Windows startup script is `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\hjkl.vbs`.

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

`start` and `enable` run the remapper in the background through the platform service manager. Internally, the service manager owns a foreground process, so you should not normally run the internal service mode by hand.

## Choosing the layer key

By default the layer ("super") key is the semicolon `;`. Use `--layer-key` to pick a different key:

```sh
hjkl enable --layer-key quote           # use the ' key as the layer key
hjkl run --layer-key grave              # foreground, using the ` key
hjkl enable --layer-key right_command   # use right Command/Windows as the layer key
```

`--layer-key` works with `run`, `start`, `restart`, and `enable`. When you set it with `enable`/`start`, the choice is saved into the platform service configuration, so it survives login and a plain `hjkl restart` keeps it (pass `--layer-key` again to change it). `hjkl status` shows the key currently in effect.

The key may be a friendly name or a raw platform key code: macOS virtual key code on macOS, Windows virtual-key code on Windows. Recognized names:

```text
semicolon  quote/apostrophe  grave/backtick  tab  return/enter  space
escape  delete  backslash  left_bracket  right_bracket  comma  period
slash  minus  equal

# modifier keys (left/right are distinct):
left_command   right_command
left_windows   right_windows  (also left_win / right_win on Windows)
left_option    right_option    (also left_alt / right_alt)
left_control   right_control   (also left_ctrl / right_ctrl)
left_shift     right_shift
```

A modifier used as the layer key is held to activate the layer and does nothing when tapped on its own. Caps Lock and Fn-like special/toggle keys are not supported, and `h`/`j`/`k`/`l` and the arrow keys are reserved as arrow targets.

## macOS permissions

macOS must allow the binary to observe keyboard events. If permission is missing, the process can be loaded by launchd but the remapping will not work.

Grant permission here:

```text
System Settings -> Privacy & Security -> Accessibility
```

You can open/request the prompt with:

```sh
hjkl permissions
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

## Windows input access

Windows does not require an Accessibility-style permission prompt for the low-level keyboard hook. The normal user-level process can remap normal user-level applications.

Windows integrity levels still apply: a non-administrator `hjkl.exe` generally cannot inject input into applications running as administrator. If remapping does not affect an elevated application, run `hjkl.exe` as administrator too.

## Troubleshooting

### macOS: `enabled: yes` and `running: yes`, but keys do not remap

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

### macOS logs

```text
~/Library/Logs/hjkl.log
~/Library/Logs/hjkl.err.log
```

### Windows status says not active

Check:

```powershell
hjkl status
```

If `key remapping` is not active, restart the background process:

```powershell
hjkl restart
```

If remapping works in normal applications but not in an application running as administrator, start `hjkl.exe` as administrator as well.

### Windows: `hjkl enable` says `schtasks /Create ... Access is denied`

That error is from v0.3.0's Task Scheduler-based auto-start path. Upgrade to v0.3.1 or newer, then run:

```powershell
hjkl enable
```

Current releases use a per-user Startup folder script instead, so normal user installs do not need permission to create a scheduled task.

## Uninstall

macOS:

```sh
scripts/uninstall.sh               # Disable auto-start, stop, and remove the binary.
scripts/uninstall.sh --keep-binary # Disable/stop only; keep the installed binary.
```

Windows:

```powershell
hjkl disable
```

Then remove `hjkl.exe` from wherever you installed it.

## Relation to the original Karabiner-Elements setup

The original setup treated `;` as `right_command` when used with another key, then mapped `right_command + h/j/k/l` to arrow keys.

This Rust implementation maps `; + h/j/k/l` directly to arrow keys. For other chords, it adds the platform shortcut modifier, so `; + c` behaves like `Command + c` on macOS and `Control + c` on Windows.

## Power usage

Power usage should be minimal. The implementation is event-driven, not polling-based.

- `CFRunLoopRun()` sleeps until keyboard events arrive.
- Windows waits on the standard message loop until keyboard hook events arrive.
- The macOS event tap listens only to `keyDown`, `keyUp`, and modifier `flagsChanged` events, not high-frequency mouse events.
- The Windows hook listens only to low-level keyboard events, not mouse events.
- Per-key work is limited to reading and rewriting a few event fields.

In practice, it should be comparable to other always-on keyboard remappers.

## Development

Commands and the Rust toolchain are managed with [mise](https://mise.jdx.dev):

```sh
mise install     # install the toolchain pinned in mise.toml
mise tasks       # list available tasks
mise run ci      # macOS CI: common Rust checks plus shell script checks
mise run ci-core # cross-platform Rust checks used by Windows CI
mise run test    # or run an individual task
```

CI runs `mise run ci` on macOS and `mise run ci-core` on Windows. The tasks are thin wrappers over `cargo` (see `mise.toml`) if you prefer to run cargo directly. See [CONTRIBUTING.md](CONTRIBUTING.md) for more.

The OS abstraction layer is `src/platform.rs`. It routes shared CLI behavior to `src/macos/` or `src/windows/`; the layer state machine itself stays in `src/keymap.rs`. The LaunchAgent plist is generated by the binary itself in `src/macos/service.rs`, while the Windows Startup folder script is generated in `src/windows/service.rs`.

## License

MIT
