# Security Policy

`hjkl-for-mac` is a local macOS utility that requires Accessibility permission
to observe and rewrite keyboard events.

## Reporting a vulnerability

Please report security issues privately to the repository owner instead of
opening a public issue first.

Include:

- affected version or commit,
- macOS version,
- reproduction steps,
- expected and actual behavior,
- whether the issue requires Accessibility permission or LaunchAgent
  installation.

## Scope

In scope:

- unintended key logging or persistence behavior,
- unsafe LaunchAgent installation behavior,
- privilege escalation or unexpected file writes.

Out of scope:

- expected local key-event observation after the user grants Accessibility
  permission,
- behavior changes caused by third-party keyboard remappers running at the same
  time.

## Trust model

`hjkl` runs entirely as your own user account. It has no network code, no
third-party dependencies, and never logs, stores, or transmits keystrokes;
key codes exist only transiently in memory while a `;`-layer chord is held.

Its sensitive asset is the **Accessibility grant**, which lets the process
observe and rewrite every keystroke system-wide. That grant is bound to the
installed binary at `~/.local/bin/hjkl` — a path your own user can write. As a
result:

- Any process already running as your user could replace that binary or point
  the LaunchAgent at other code. This is the same capability such a process
  already has over your account, but because `hjkl` holds a standing
  Accessibility grant and auto-starts at login, treat write access to the
  install path and `~/Library/LaunchAgents/com.kazuki-hanai.hjkl-for-mac.plist`
  as security-relevant.
- The management commands always re-render the LaunchAgent plist from the
  binary's own state before loading it, so a tampered on-disk plist cannot make
  `launchctl` load an arbitrary job.
- Prebuilt releases are currently **not** code-signed with an Apple Developer ID.
  Because macOS keys the Accessibility grant to the binary's code identity, an
  unsigned binary's grant is pinned by content hash/path rather than a stable
  signature. Signing with a Developer ID and a pinned designated requirement is
  planned; it would make the grant non-transferable to a replacement binary and
  remove the re-approval churn after each rebuild. Until then, only install
  `hjkl` from a location you control and verify downloads (see the README).
