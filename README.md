# hjkl

`hjkl` is a small macOS and Windows keyboard remapper written in Rust.

It gives you a semicolon-based layer:

| Input | Output |
| --- | --- |
| Tap `;` | `;` |
| Hold `;` + `h` | Left Arrow |
| Hold `;` + `j` | Down Arrow |
| Hold `;` + `k` | Up Arrow |
| Hold `;` + `l` | Right Arrow |
| Hold `;` + another key | `Command` + key on macOS, `Control` + key on Windows |

On macOS, `hjkl` uses `CGEventTap`. On Windows, it uses a low-level keyboard hook plus `SendInput`. It does not require Karabiner-Elements.

## Contents

- [Requirements](#requirements)
- [Install](#install)
- [First Run](#first-run)
- [Commands](#commands)
- [Layer Key](#layer-key)
- [Platform Notes](#platform-notes)
- [Troubleshooting](#troubleshooting)
- [Uninstall](#uninstall)
- [How It Works](#how-it-works)
- [Development](#development)

## Requirements

- macOS or Windows
- Rust toolchain, only if building from source
- macOS: Accessibility permission for the installed binary
- Windows: administrator mode only if you need remapping inside elevated applications

## Install

### Prebuilt Release

Download the archive for your platform from the [Releases page](https://github.com/kazuki-hanai/hjkl/releases):

- macOS: `hjkl-<version>-macos-<arch>.tar.gz`
- Windows: `hjkl-<version>-windows-x86_64.zip`

Each release also includes a `.sha256` file.

macOS:

```sh
shasum -a 256 -c hjkl-<version>-macos-<arch>.tar.gz.sha256
tar -xzf hjkl-<version>-macos-<arch>.tar.gz
```

Windows PowerShell:

```powershell
Get-FileHash .\hjkl-<version>-windows-x86_64.zip -Algorithm SHA256
Expand-Archive .\hjkl-<version>-windows-x86_64.zip
```

The `.sha256` file checks for download corruption. For authenticity, use the GitHub Actions provenance attestation.

Optional provenance check, using GitHub CLI:

```sh
gh attestation verify <downloaded-archive> --repo kazuki-hanai/hjkl
```

Put the extracted `hjkl` or `hjkl.exe` somewhere on your `PATH`.

### Build From Source

```sh
cargo build --release
```

The built binary is:

- macOS/Linux-style path: `target/release/hjkl`
- Windows path: `target\release\hjkl.exe`

For macOS source installs, the helper script builds, installs to `~/.local/bin/hjkl`, and enables the LaunchAgent:

```sh
scripts/install.sh
```

## First Run

### macOS

Grant Accessibility permission to the installed binary:

```text
System Settings -> Privacy & Security -> Accessibility
```

Allow:

```text
~/.local/bin/hjkl
```

You can ask macOS to open the permission prompt:

```sh
hjkl permissions
```

Then enable the background process:

```sh
hjkl enable
hjkl status
```

A healthy setup shows:

```text
enabled: yes (auto-start at login)
running: yes
key remapping: active
accessibility: granted
```

The macOS binary is not code-signed or notarized. On first launch, Gatekeeper may warn. Open it once from **System Settings -> Privacy & Security -> Open Anyway** instead of disabling Gatekeeper globally.

### Windows

Enable auto-start and start the remapper:

```powershell
hjkl enable
hjkl status
```

A healthy setup shows:

```text
enabled: yes (auto-start at login)
running: yes
key remapping: active
input access: available for the current desktop session
```

`hjkl enable` creates this per-user Startup folder script:

```text
%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\hjkl.vbs
```

It starts `hjkl.exe run --service` at login without needing administrator permission.

## Commands

```sh
hjkl start       # Start in the background now; do not enable login auto-start.
hjkl stop        # Stop the background process.
hjkl restart     # Restart the background process.
hjkl enable      # Enable login auto-start and start now.
hjkl disable     # Disable login auto-start and stop now.
hjkl status      # Show service state and file paths.
hjkl permissions # Show platform-specific permission guidance.
hjkl --help
hjkl --version
```

Use foreground mode for quick testing:

```sh
hjkl run
```

or from source:

```sh
cargo run --release
```

Foreground mode remaps keys only while the process is running. Press `Ctrl-C` to stop it.

## Layer Key

The default layer key is semicolon:

```text
;
```

Pick another layer key with `--layer-key`:

```sh
hjkl enable --layer-key quote
hjkl run --layer-key grave
hjkl enable --layer-key right_command
```

`--layer-key` works with:

```text
run  start  restart  enable
```

When used with `start` or `enable`, the choice is saved. A later plain `hjkl restart` keeps it. Pass `--layer-key` again to change it.

Accepted names:

```text
semicolon  quote/apostrophe  grave/backtick  tab  return/enter  space
escape  delete  backslash  left_bracket  right_bracket  comma  period
slash  minus  equal

left_command   right_command
left_windows   right_windows  (also left_win / right_win on Windows)
left_option    right_option    (also left_alt / right_alt)
left_control   right_control   (also left_ctrl / right_ctrl)
left_shift     right_shift
```

You can also pass a raw platform key code:

- macOS: virtual key code
- Windows: virtual-key code

Limitations:

- A modifier layer key is held to activate the layer and does nothing when tapped alone.
- Caps Lock and Fn-like special/toggle keys are not supported.
- `h`, `j`, `k`, `l`, and arrow keys cannot be used as the layer key.

## Platform Notes

### macOS

The background process is a per-user launchd LaunchAgent.

Paths:

```text
~/Library/LaunchAgents/com.kazuki-hanai.hjkl.plist
~/Library/Logs/hjkl.log
~/Library/Logs/hjkl.err.log
```

If you rebuild or reinstall the binary, macOS may keep the old Accessibility entry while denying the new binary. Remove `~/.local/bin/hjkl` from Accessibility, add it again, then run:

```sh
hjkl restart
```

If Input Monitoring lists `hjkl`, allow it there too.

### Windows

The background process is a normal user process. `hjkl enable` installs a per-user Startup folder script, not a Windows Service and not a scheduled task.

Windows integrity levels still apply. A non-administrator `hjkl.exe` generally cannot inject input into applications running as administrator. If remapping works in normal apps but not in an elevated app, run `hjkl.exe` as administrator too.

v0.3.0 used Task Scheduler and could fail with:

```text
Access is denied.
`schtasks /Create /TN hjkl ...` failed (exit code 1)
```

Upgrade to v0.3.1 or newer and run:

```powershell
hjkl enable
```

## Troubleshooting

Start with:

```sh
hjkl status
```

The key line is:

```text
key remapping: active
```

If `enable`, `start`, or `restart` exits with an error, the background process was started but `hjkl` could not confirm remapping is active. Follow the error message, then run:

```sh
hjkl restart
```

Common cases:

- macOS: Accessibility permission is missing or attached to an older binary.
- macOS: Input Monitoring is present and disabled for `hjkl`.
- Windows: the target app is running as administrator but `hjkl.exe` is not.
- Windows v0.3.0: `schtasks /Create` failed; upgrade to v0.3.1 or newer.

## Uninstall

macOS:

```sh
scripts/uninstall.sh
scripts/uninstall.sh --keep-binary
```

Windows:

```powershell
hjkl disable
```

Then remove the binary from wherever you installed it.

## How It Works

The layer behavior is implemented once in `src/keymap.rs`.

Platform adapters translate OS keyboard events into that shared state machine:

- macOS: `src/macos/` uses `CGEventTap`.
- Windows: `src/windows/` uses `SetWindowsHookExW(WH_KEYBOARD_LL, ...)` and emits replacement input with `SendInput`.

The original Karabiner-Elements setup treated `;` as `right_command` when used with another key, then mapped `right_command + h/j/k/l` to arrows. `hjkl` maps `; + h/j/k/l` directly to arrows. For other chords, it adds the platform shortcut modifier: Command on macOS, Control on Windows.

Power usage should be minimal. The implementation is event-driven, not polling-based.

## Development

Commands and the Rust toolchain are managed with [mise](https://mise.jdx.dev):

```sh
mise install
mise tasks
mise run ci
mise run ci-core
mise run test
```

CI runs `mise run ci` on macOS and `mise run ci-core` on Windows. The tasks are thin wrappers over `cargo` if you prefer to run cargo directly.

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution details.

## License

MIT
