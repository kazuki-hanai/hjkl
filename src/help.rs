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

OPTIONS:
    --layer-key <key>   Use <key> as the layer (\"super\") key instead of the
                        default semicolon. Accepts a key name or a macOS key
                        code. Names include semicolon, quote, grave, tab,
                        return, space, and the modifier keys left_command,
                        right_command, left_option, right_option,
                        left_control, right_control, left_shift, right_shift.
                        Valid for run/start/restart/enable; a plain restart
                        keeps the previously configured key. Caps Lock, Fn, and
                        h/j/k/l are not allowed.

BEHAVIOR (with the default layer key ';'):
    ;          -> ;     (when tapped by itself)
    ; + h      -> Left Arrow
    ; + j      -> Down Arrow
    ; + k      -> Up Arrow
    ; + l      -> Right Arrow
    ; + other  -> Command + other

    With a different layer key, that key plays the role ';' has above.

NOTES:
    The program must keep running to remap keys. `enable`/`start` do this in the
    background via a per-user launchd LaunchAgent.
    `start` runs now but does NOT auto-start at login; `enable` does both.
    Internal launchd mode runs in the foreground because launchd manages the
    background service process.
    macOS will require Accessibility permission for this binary. After granting
    it, run `hjkl restart`.
    Example: `hjkl enable --layer-key quote` uses the quote key as the layer.
"
    );
}
