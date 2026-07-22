//! User-facing help text.

use crate::{keymap, platform};

pub(crate) fn print_help() {
    let service_manager = platform::SERVICE_MANAGER;
    let permissions_note = platform::PERMISSIONS_NOTE;
    let shortcut_modifier = keymap::shortcut_modifier_name();
    let key_code_name = keymap::platform_key_code_name();

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
    enable         Install auto-start at login (starts now too).
    disable        Remove auto-start at login (stops now too).
    status         Show whether it is enabled/running and where files live.
    permissions    Show platform-specific input permission guidance.

OPTIONS:
    --layer-key <key>   Use <key> as the layer (\"super\") key instead of the
                        default semicolon. Accepts a key name or a platform key
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
    ; + other  -> {shortcut_modifier} + other

    With a different layer key, that key plays the role ';' has above.

NOTES:
    The program must keep running to remap keys. `enable`/`start` do this in the
    background via {service_manager}.
    `start` runs now but does NOT auto-start at login; `enable` does both.
    Internal service mode runs in the foreground because the OS service manager
    owns the background process.
    Raw numeric layer-key values are interpreted as {key_code_name}s.
    {permissions_note}
    Example: `hjkl enable --layer-key quote` uses the quote key as the layer.
"
    );
}
