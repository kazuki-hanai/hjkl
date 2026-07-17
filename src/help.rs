//! User-facing help text.

pub(crate) fn print_help() {
    println!(
        "\
hjkl

USAGE:
    hjkl [SUBCOMMAND]

SUBCOMMANDS:
    (none) | run   Run the remapper in the foreground (Ctrl-C to stop).
    start          Start the remapper in the background now.
    stop           Stop the background remapper.
    restart        Restart the background remapper.
    enable         Install a LaunchAgent so it auto-starts at login (starts now too).
    disable        Remove the LaunchAgent so it no longer auto-starts (stops now too).
    status         Show whether it is enabled/running and where files live.
    permissions    Ask macOS to show the Accessibility permission prompt.

BEHAVIOR:
    ;          -> ;     (when tapped by itself)
    ; + h      -> Left Arrow
    ; + j      -> Down Arrow
    ; + k      -> Up Arrow
    ; + l      -> Right Arrow
    ; + other  -> Command + other

NOTES:
    The program must keep running to remap keys. `enable`/`start` do this in the
    background via a per-user launchd LaunchAgent.
    `start` runs now but does NOT auto-start at login; `enable` does both.
    Internal launchd mode runs in the foreground because launchd manages the
    background service process.
    macOS will require Accessibility permission for this binary. After granting
    it, run `hjkl restart`.
"
    );
}
