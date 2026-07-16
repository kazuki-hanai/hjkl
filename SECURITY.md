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
