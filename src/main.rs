//! A tiny macOS keyboard remapper:
//!
//! - tap `;` by itself -> `;`
//! - hold `;` and press `h/j/k/l` -> left/down/up/right arrow
//! - hold `;` and press another key -> Command + that key
//!
//! This implements the same composed behavior as the original
//! Karabiner-Elements setup without depending on Karabiner at runtime.

#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = macos::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("hjkl-for-mac only supports macOS.");
    std::process::exit(1);
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::os::raw::c_long;
    use std::ptr;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicPtr, Ordering};
    use std::thread;
    use std::time::Duration;

    type Boolean = bool;
    type CFAllocatorRef = *const c_void;
    type CFIndex = c_long;
    type CFMachPortRef = *mut c_void;
    type CFRunLoopRef = *mut c_void;
    type CFRunLoopSourceRef = *mut c_void;
    type CFStringRef = *const c_void;
    type CFTypeRef = *const c_void;
    type CGEventRef = *mut c_void;
    type CGEventTapProxy = *mut c_void;
    type CGEventType = u32;
    type CGEventMask = u64;
    type CGEventField = u32;
    type CGEventFlags = u64;
    type CGKeyCode = u16;
    type UniCharCount = u64;

    type CGEventTapCallBack =
        unsafe extern "C" fn(CGEventTapProxy, CGEventType, CGEventRef, *mut c_void) -> CGEventRef;

    const K_CG_HID_EVENT_TAP: u32 = 0;
    const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
    const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

    const K_CG_EVENT_KEY_DOWN: CGEventType = 10;
    const K_CG_EVENT_KEY_UP: CGEventType = 11;
    const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: CGEventType = 0xFFFF_FFFE;
    const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: CGEventType = 0xFFFF_FFFF;

    const K_CG_KEYBOARD_EVENT_KEYCODE: CGEventField = 9;
    const K_CG_EVENT_SOURCE_USER_DATA: CGEventField = 42;

    const KEY_H: CGKeyCode = 4;
    const KEY_L: CGKeyCode = 37;
    const KEY_J: CGKeyCode = 38;
    const KEY_K: CGKeyCode = 40;
    const KEY_SEMICOLON: CGKeyCode = 41;

    const KEY_LEFT_ARROW: CGKeyCode = 123;
    const KEY_RIGHT_ARROW: CGKeyCode = 124;
    const KEY_DOWN_ARROW: CGKeyCode = 125;
    const KEY_UP_ARROW: CGKeyCode = 126;

    const K_CG_EVENT_FLAG_MASK_COMMAND: CGEventFlags = 1 << 20;
    const EVENT_TAP_RETRY_INTERVAL: Duration = Duration::from_secs(30);

    // Marker placed on events synthesized by this process. Without it, the
    // event tap would see its own synthetic semicolon key events and suppress
    // them again.
    const SYNTHETIC_EVENT_TAG: i64 = 0x686A_6B6C_5F72_7374; // "hjkl_rst"

    static STATE: Mutex<LayerState> = Mutex::new(LayerState::new());
    static EVENT_TAP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

    #[derive(Debug, Clone, Copy)]
    struct LayerState {
        semicolon_down: bool,
        used_as_layer: bool,
        command_layer_active: bool,
        semicolon_flags: CGEventFlags,
        mapped_keys_down: u8,
    }

    impl LayerState {
        const fn new() -> Self {
            Self {
                semicolon_down: false,
                used_as_layer: false,
                command_layer_active: false,
                semicolon_flags: 0,
                mapped_keys_down: 0,
            }
        }

        fn clear_semicolon(&mut self) {
            self.semicolon_down = false;
            self.used_as_layer = false;
            self.command_layer_active = false;
            self.semicolon_flags = 0;
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Command {
        Run { daemon: bool },
        Start,
        Stop,
        Restart,
        Enable,
        Disable,
        Status,
        Help,
        Version,
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: CGEventMask,
            callback: CGEventTapCallBack,
            user_info: *mut c_void,
        ) -> CFMachPortRef;

        fn CGEventTapEnable(tap: CFMachPortRef, enable: Boolean);
        fn CGEventGetIntegerValueField(event: CGEventRef, field: CGEventField) -> i64;
        fn CGEventSetIntegerValueField(event: CGEventRef, field: CGEventField, value: i64);
        fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
        fn CGEventSetFlags(event: CGEventRef, flags: CGEventFlags);
        fn CGEventKeyboardSetUnicodeString(
            event: CGEventRef,
            string_length: UniCharCount,
            unicode_string: *const u16,
        );
        fn CGEventCreateKeyboardEvent(
            source: *const c_void,
            virtual_key: CGKeyCode,
            key_down: Boolean,
        ) -> CGEventRef;
        fn CGEventPost(tap: u32, event: CGEventRef);
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFRunLoopCommonModes: CFStringRef;

        fn CFMachPortCreateRunLoopSource(
            allocator: CFAllocatorRef,
            port: CFMachPortRef,
            order: CFIndex,
        ) -> CFRunLoopSourceRef;
        fn CFRunLoopGetCurrent() -> CFRunLoopRef;
        fn CFRunLoopAddSource(
            run_loop: CFRunLoopRef,
            source: CFRunLoopSourceRef,
            mode: CFStringRef,
        );
        fn CFRunLoopRun();
        fn CFRelease(cf: CFTypeRef);
    }

    pub fn run() -> Result<(), String> {
        match parse_args(std::env::args().skip(1))? {
            Command::Run { daemon } => run_event_loop(daemon),
            Command::Start => service::start(),
            Command::Stop => service::stop(),
            Command::Restart => service::restart(),
            Command::Enable => service::enable(),
            Command::Disable => service::disable(),
            Command::Status => service::status(),
            Command::Help => {
                print_help();
                Ok(())
            }
            Command::Version => {
                println!("hjkl-for-mac {}", env!("CARGO_PKG_VERSION"));
                Ok(())
            }
        }
    }

    fn run_event_loop(daemon: bool) -> Result<(), String> {
        let mask = event_mask(K_CG_EVENT_KEY_DOWN) | event_mask(K_CG_EVENT_KEY_UP);
        let tap = create_event_tap(mask, daemon)?;

        EVENT_TAP.store(tap, Ordering::SeqCst);

        let source = unsafe { CFMachPortCreateRunLoopSource(ptr::null(), tap, 0) };
        if source.is_null() {
            unsafe {
                CFRelease(tap.cast());
            }
            return Err("Failed to create a run loop source for the event tap.".to_string());
        }

        unsafe {
            CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
        }

        println!("hjkl-for-mac is running.");
        println!("Tap ';' alone for ';'. Hold ';' + h/j/k/l for left/down/up/right arrows.");
        println!("Hold ';' + another key to send Command + that key.");
        println!("Keep this process running. Press Ctrl-C to stop.");

        unsafe {
            CFRunLoopRun();
            CFRelease(source.cast());
            CFRelease(tap.cast());
        }

        Ok(())
    }

    fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
        let args: Vec<String> = args.into_iter().collect();

        // `--help`/`--version` win regardless of position or subcommand.
        if args.iter().any(|arg| arg == "-h" || arg == "--help") {
            return Ok(Command::Help);
        }
        if args.iter().any(|arg| arg == "-V" || arg == "--version") {
            return Ok(Command::Version);
        }

        let mut iter = args.iter();
        let Some(first) = iter.next() else {
            return Ok(Command::Run { daemon: false });
        };

        match first.as_str() {
            // Legacy invocation kept for older LaunchAgent plists: `hjkl --daemon`.
            "--daemon" => {
                ensure_no_extra_args(iter)?;
                Ok(Command::Run { daemon: true })
            }
            "run" => {
                let mut daemon = false;
                for arg in iter {
                    match arg.as_str() {
                        "--daemon" => daemon = true,
                        other => return Err(unknown_argument(other)),
                    }
                }
                Ok(Command::Run { daemon })
            }
            "start" => ensure_no_extra_args(iter).map(|()| Command::Start),
            "stop" => ensure_no_extra_args(iter).map(|()| Command::Stop),
            "restart" => ensure_no_extra_args(iter).map(|()| Command::Restart),
            "enable" => ensure_no_extra_args(iter).map(|()| Command::Enable),
            "disable" => ensure_no_extra_args(iter).map(|()| Command::Disable),
            "status" => ensure_no_extra_args(iter).map(|()| Command::Status),
            other => Err(unknown_argument(other)),
        }
    }

    fn ensure_no_extra_args<'a>(mut iter: impl Iterator<Item = &'a String>) -> Result<(), String> {
        match iter.next() {
            Some(extra) => Err(unknown_argument(extra)),
            None => Ok(()),
        }
    }

    fn unknown_argument(arg: &str) -> String {
        format!("Unknown argument: {arg}\n\nRun `hjkl-for-mac --help` for usage.")
    }

    fn create_event_tap(
        mask: CGEventMask,
        retry_until_available: bool,
    ) -> Result<CFMachPortRef, String> {
        loop {
            let tap = unsafe {
                CGEventTapCreate(
                    K_CG_HID_EVENT_TAP,
                    K_CG_HEAD_INSERT_EVENT_TAP,
                    K_CG_EVENT_TAP_OPTION_DEFAULT,
                    mask,
                    event_callback,
                    ptr::null_mut(),
                )
            };

            if !tap.is_null() {
                return Ok(tap);
            }

            if !retry_until_available {
                return Err(event_tap_permission_error());
            }

            eprintln!(
                "{}\nRetrying in {} seconds for daemon mode...",
                event_tap_permission_error(),
                EVENT_TAP_RETRY_INTERVAL.as_secs()
            );
            thread::sleep(EVENT_TAP_RETRY_INTERVAL);
        }
    }

    fn event_tap_permission_error() -> String {
        "Failed to create a keyboard event tap.\n\
         Grant this terminal/binary permission in macOS System Settings:\n\
         Privacy & Security -> Accessibility, and if necessary Input Monitoring.\n\
         Then restart the terminal or daemon."
            .to_string()
    }

    /// Self-contained management of the per-user launchd LaunchAgent, so the
    /// binary can offer `start`/`stop`/`restart`/`enable`/`disable`/`status`
    /// without any external shell scripts.
    mod service {
        use std::fs;
        use std::path::{Path, PathBuf};
        use std::process::{Command as SysCommand, Stdio};

        pub const LABEL: &str = "com.kazuki-hanai.hjkl-for-mac";

        // getuid() lives in libSystem, which is always linked on macOS.
        unsafe extern "C" {
            fn getuid() -> u32;
        }

        fn uid() -> u32 {
            unsafe { getuid() }
        }

        fn service_target() -> String {
            format!("gui/{}/{}", uid(), LABEL)
        }

        fn domain_target() -> String {
            format!("gui/{}", uid())
        }

        fn home_dir() -> Result<PathBuf, String> {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|path| !path.as_os_str().is_empty())
                .ok_or_else(|| "HOME environment variable is not set".to_string())
        }

        /// Plist under `~/Library/LaunchAgents`. Its mere presence there is what
        /// makes launchd auto-load the agent at login, i.e. "enabled".
        fn launch_agents_plist() -> Result<PathBuf, String> {
            Ok(home_dir()?
                .join("Library/LaunchAgents")
                .join(format!("{LABEL}.plist")))
        }

        /// Plist used by `start` when the agent is not enabled. It lives outside
        /// `LaunchAgents` so launchd runs it now but not automatically at login.
        fn runtime_plist() -> Result<PathBuf, String> {
            Ok(home_dir()?
                .join("Library/Application Support")
                .join(LABEL)
                .join(format!("{LABEL}.plist")))
        }

        fn log_paths() -> Result<(PathBuf, PathBuf), String> {
            let dir = home_dir()?.join("Library/Logs");
            Ok((
                dir.join("hjkl-for-mac.log"),
                dir.join("hjkl-for-mac.err.log"),
            ))
        }

        fn binary_path() -> Result<PathBuf, String> {
            let exe = std::env::current_exe()
                .map_err(|error| format!("failed to resolve current executable: {error}"))?;
            Ok(exe.canonicalize().unwrap_or(exe))
        }

        fn path_to_str(path: &Path) -> Result<&str, String> {
            path.to_str()
                .ok_or_else(|| format!("path is not valid UTF-8: {}", path.display()))
        }

        fn xml_escape(input: &str) -> String {
            input
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        }

        pub fn render_plist() -> Result<String, String> {
            let binary = binary_path()?;
            let (stdout_log, stderr_log) = log_paths()?;

            Ok(format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n\
<dict>\n\
\t<key>Label</key>\n\
\t<string>{label}</string>\n\
\t<key>ProgramArguments</key>\n\
\t<array>\n\
\t\t<string>{binary}</string>\n\
\t\t<string>run</string>\n\
\t\t<string>--daemon</string>\n\
\t</array>\n\
\t<key>RunAtLoad</key>\n\
\t<true/>\n\
\t<key>KeepAlive</key>\n\
\t<true/>\n\
\t<key>ThrottleInterval</key>\n\
\t<integer>30</integer>\n\
\t<key>ProcessType</key>\n\
\t<string>Interactive</string>\n\
\t<key>StandardOutPath</key>\n\
\t<string>{stdout}</string>\n\
\t<key>StandardErrorPath</key>\n\
\t<string>{stderr}</string>\n\
</dict>\n\
</plist>\n",
                label = xml_escape(LABEL),
                binary = xml_escape(path_to_str(&binary)?),
                stdout = xml_escape(path_to_str(&stdout_log)?),
                stderr = xml_escape(path_to_str(&stderr_log)?),
            ))
        }

        fn write_plist_to(path: &Path) -> Result<(), String> {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
            let contents = render_plist()?;
            fs::write(path, contents)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))
        }

        fn ensure_log_dir() -> Result<(), String> {
            let (stdout_log, _) = log_paths()?;
            if let Some(dir) = stdout_log.parent() {
                fs::create_dir_all(dir)
                    .map_err(|error| format!("failed to create {}: {error}", dir.display()))?;
            }
            Ok(())
        }

        fn launchctl(args: &[&str]) -> Result<(), String> {
            let status = SysCommand::new("launchctl")
                .args(args)
                .status()
                .map_err(|error| format!("failed to run launchctl: {error}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "`launchctl {}` failed ({})",
                    args.join(" "),
                    status
                        .code()
                        .map(|code| format!("exit code {code}"))
                        .unwrap_or_else(|| "terminated by signal".to_string()),
                ))
            }
        }

        fn launchctl_quiet(args: &[&str]) {
            let _ = SysCommand::new("launchctl")
                .args(args)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }

        fn is_loaded() -> bool {
            SysCommand::new("launchctl")
                .args(["print", &service_target()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        }

        fn print_permission_hint() {
            if let Ok(binary) = binary_path() {
                println!();
                println!("If keys are not remapped yet, grant Accessibility permission to:");
                println!("  {}", binary.display());
                println!("System Settings -> Privacy & Security -> Accessibility");
                println!("Then run `hjkl-for-mac restart`.");
            }
        }

        pub fn start() -> Result<(), String> {
            ensure_log_dir()?;

            let launch_agents = launch_agents_plist()?;
            let enabled = launch_agents.exists();
            let plist = if enabled {
                launch_agents
            } else {
                let runtime = runtime_plist()?;
                write_plist_to(&runtime)?;
                runtime
            };

            // Reload cleanly and clear any prior `disable` override so a manual
            // start still works.
            launchctl_quiet(&["bootout", &service_target()]);
            launchctl_quiet(&["enable", &service_target()]);
            launchctl(&["bootstrap", &domain_target(), path_to_str(&plist)?])?;
            launchctl(&["kickstart", "-k", &service_target()])?;

            println!("hjkl-for-mac started in the background.");
            if !enabled {
                println!("It will NOT auto-start at login. Run `hjkl-for-mac enable` for that.");
            }
            print_permission_hint();
            Ok(())
        }

        pub fn enable() -> Result<(), String> {
            ensure_log_dir()?;

            let plist = launch_agents_plist()?;
            write_plist_to(&plist)?;

            launchctl_quiet(&["bootout", &service_target()]);
            launchctl_quiet(&["enable", &service_target()]);
            launchctl(&["bootstrap", &domain_target(), path_to_str(&plist)?])?;
            launchctl(&["kickstart", "-k", &service_target()])?;

            println!("hjkl-for-mac enabled: it will auto-start at login and is running now.");
            print_permission_hint();
            Ok(())
        }

        pub fn stop() -> Result<(), String> {
            if !is_loaded() {
                println!("hjkl-for-mac is not running.");
                return Ok(());
            }
            launchctl(&["bootout", &service_target()])?;
            println!("hjkl-for-mac stopped.");
            Ok(())
        }

        pub fn restart() -> Result<(), String> {
            if is_loaded() {
                launchctl(&["kickstart", "-k", &service_target()])?;
                println!("hjkl-for-mac restarted.");
                Ok(())
            } else {
                println!("hjkl-for-mac was not running; starting it...");
                start()
            }
        }

        pub fn disable() -> Result<(), String> {
            launchctl_quiet(&["bootout", &service_target()]);
            launchctl_quiet(&["disable", &service_target()]);

            let launch_agents = launch_agents_plist()?;
            if launch_agents.exists() {
                fs::remove_file(&launch_agents).map_err(|error| {
                    format!("failed to remove {}: {error}", launch_agents.display())
                })?;
            }
            if let Ok(runtime) = runtime_plist() {
                let _ = fs::remove_file(runtime);
            }

            println!("hjkl-for-mac disabled: it will not auto-start at login and is stopped.");
            Ok(())
        }

        pub fn status() -> Result<(), String> {
            let launch_agents = launch_agents_plist()?;
            let (stdout_log, stderr_log) = log_paths()?;

            println!("label:   {LABEL}");
            println!(
                "enabled: {}",
                if launch_agents.exists() {
                    "yes (auto-start at login)"
                } else {
                    "no"
                }
            );
            println!("running: {}", if is_loaded() { "yes" } else { "no" });
            match binary_path() {
                Ok(binary) => println!("binary:  {}", binary.display()),
                Err(error) => println!("binary:  <unknown> ({error})"),
            }
            println!("plist:   {}", launch_agents.display());
            println!("stdout:  {}", stdout_log.display());
            println!("stderr:  {}", stderr_log.display());
            println!();
            println!("launchctl print {}:", service_target());
            if launchctl(&["print", &service_target()]).is_err() {
                println!("(not loaded)");
            }
            Ok(())
        }
    }

    fn print_help() {
        println!(
            "\
hjkl-for-mac

USAGE:
    hjkl-for-mac [SUBCOMMAND]

SUBCOMMANDS:
    (none) | run   Run the remapper in the foreground (Ctrl-C to stop).
    start          Start the remapper in the background now.
    stop           Stop the background remapper.
    restart        Restart the background remapper.
    enable         Install a LaunchAgent so it auto-starts at login (starts now too).
    disable        Remove the LaunchAgent so it no longer auto-starts (stops now too).
    status         Show whether it is enabled/running and where files live.
    run --daemon   Foreground loop that retries permission every 30s (used by launchd).

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
    macOS will require Accessibility permission for this binary. After granting
    it, run `hjkl-for-mac restart`.
"
        );
    }

    unsafe extern "C" fn event_callback(
        _proxy: CGEventTapProxy,
        event_type: CGEventType,
        event: CGEventRef,
        _user_info: *mut c_void,
    ) -> CGEventRef {
        if event_type == K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT
            || event_type == K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT
        {
            let tap = EVENT_TAP.load(Ordering::SeqCst);
            if !tap.is_null() {
                unsafe {
                    CGEventTapEnable(tap, true);
                }
            }
            return event;
        }

        if event.is_null() {
            return event;
        }

        let user_data = unsafe { CGEventGetIntegerValueField(event, K_CG_EVENT_SOURCE_USER_DATA) };
        if user_data == SYNTHETIC_EVENT_TAG {
            return event;
        }

        if event_type != K_CG_EVENT_KEY_DOWN && event_type != K_CG_EVENT_KEY_UP {
            return event;
        }

        let key_code = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) };
        if !(0..=u16::MAX as i64).contains(&key_code) {
            return event;
        }
        let key_code = key_code as CGKeyCode;

        let mut state = match STATE.lock() {
            Ok(state) => state,
            Err(_) => return event,
        };

        if let Some(key_bit) = hjkl_key_bit(key_code)
            && state.mapped_keys_down & key_bit != 0
        {
            if event_type == K_CG_EVENT_KEY_UP {
                state.mapped_keys_down &= !key_bit;
            }

            let arrow_key = hjkl_to_arrow(key_code).expect("hjkl bit must have arrow mapping");
            drop(state);
            rewrite_as_arrow(event, arrow_key);
            return event;
        }

        match (event_type, key_code) {
            (K_CG_EVENT_KEY_DOWN, KEY_SEMICOLON) => {
                // Delay semicolon until key-up. If another key is pressed in
                // between, the semicolon became the layer key and should not
                // be emitted as text.
                if !state.semicolon_down {
                    state.semicolon_down = true;
                    state.used_as_layer = false;
                    state.semicolon_flags = unsafe { CGEventGetFlags(event) };
                }
                suppress()
            }
            (K_CG_EVENT_KEY_UP, KEY_SEMICOLON) if state.semicolon_down => {
                let should_post_semicolon = !state.used_as_layer;
                let semicolon_flags = state.semicolon_flags;

                state.clear_semicolon();
                drop(state);

                if should_post_semicolon {
                    post_key(KEY_SEMICOLON, semicolon_flags);
                }
                suppress()
            }
            (event_type, key_code) if state.semicolon_down => {
                if let Some(arrow_key) = hjkl_to_arrow(key_code) {
                    if event_type == K_CG_EVENT_KEY_DOWN {
                        state.used_as_layer = true;
                        if let Some(key_bit) = hjkl_key_bit(key_code) {
                            state.mapped_keys_down |= key_bit;
                        }
                        drop(state);

                        rewrite_as_arrow(event, arrow_key);
                    }
                    event
                } else {
                    // Karabiner's first rule turns semicolon into
                    // right_command when it is used with any other key. We
                    // emulate that for normal shortcuts by adding the Command
                    // flag to non-hjkl events while the semicolon layer is
                    // held. hjkl is handled above and intentionally becomes a
                    // plain arrow key instead.
                    if event_type == K_CG_EVENT_KEY_DOWN {
                        state.used_as_layer = true;
                        state.command_layer_active = true;
                    }
                    let should_add_command = state.command_layer_active;
                    drop(state);

                    if should_add_command {
                        add_command_flag(event);
                    }
                    event
                }
            }
            _ => event,
        }
    }

    fn event_mask(event_type: CGEventType) -> CGEventMask {
        1u64 << event_type
    }

    fn suppress() -> CGEventRef {
        ptr::null_mut()
    }

    fn hjkl_to_arrow(key_code: CGKeyCode) -> Option<CGKeyCode> {
        match key_code {
            KEY_H => Some(KEY_LEFT_ARROW),
            KEY_J => Some(KEY_DOWN_ARROW),
            KEY_K => Some(KEY_UP_ARROW),
            KEY_L => Some(KEY_RIGHT_ARROW),
            _ => None,
        }
    }

    fn hjkl_key_bit(key_code: CGKeyCode) -> Option<u8> {
        match key_code {
            KEY_H => Some(1 << 0),
            KEY_J => Some(1 << 1),
            KEY_K => Some(1 << 2),
            KEY_L => Some(1 << 3),
            _ => None,
        }
    }

    fn rewrite_as_arrow(event: CGEventRef, arrow_key: CGKeyCode) {
        unsafe {
            CGEventSetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE, i64::from(arrow_key));
            // The original event still has the text payload for h/j/k/l. Clear
            // it so apps that inspect Unicode see a real non-text arrow key
            // event.
            CGEventKeyboardSetUnicodeString(event, 0, ptr::null());
        }
    }

    fn add_command_flag(event: CGEventRef) {
        unsafe {
            CGEventSetFlags(event, with_command_flag(CGEventGetFlags(event)));
        }
    }

    fn with_command_flag(flags: CGEventFlags) -> CGEventFlags {
        flags | K_CG_EVENT_FLAG_MASK_COMMAND
    }

    fn post_key(key_code: CGKeyCode, flags: CGEventFlags) {
        post_keyboard_event(key_code, true, flags);
        post_keyboard_event(key_code, false, flags);
    }

    fn post_keyboard_event(key_code: CGKeyCode, key_down: bool, flags: CGEventFlags) {
        let event = unsafe { CGEventCreateKeyboardEvent(ptr::null(), key_code, key_down) };
        if event.is_null() {
            return;
        }

        unsafe {
            CGEventSetFlags(event, flags);
            CGEventSetIntegerValueField(event, K_CG_EVENT_SOURCE_USER_DATA, SYNTHETIC_EVENT_TAG);
            CGEventPost(K_CG_HID_EVENT_TAP, event);
            CFRelease(event.cast());
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn maps_hjkl_to_arrow_keys() {
            assert_eq!(hjkl_to_arrow(KEY_H), Some(KEY_LEFT_ARROW));
            assert_eq!(hjkl_to_arrow(KEY_J), Some(KEY_DOWN_ARROW));
            assert_eq!(hjkl_to_arrow(KEY_K), Some(KEY_UP_ARROW));
            assert_eq!(hjkl_to_arrow(KEY_L), Some(KEY_RIGHT_ARROW));
        }

        #[test]
        fn does_not_map_unrelated_keys() {
            assert_eq!(hjkl_to_arrow(KEY_SEMICOLON), None);
            assert_eq!(hjkl_to_arrow(0), None); // A
        }

        #[test]
        fn maps_hjkl_keys_to_distinct_state_bits() {
            assert_eq!(hjkl_key_bit(KEY_H), Some(1 << 0));
            assert_eq!(hjkl_key_bit(KEY_J), Some(1 << 1));
            assert_eq!(hjkl_key_bit(KEY_K), Some(1 << 2));
            assert_eq!(hjkl_key_bit(KEY_L), Some(1 << 3));
            assert_eq!(hjkl_key_bit(KEY_SEMICOLON), None);
        }

        #[test]
        fn command_flag_is_added_without_dropping_other_flags() {
            let shift_flag: CGEventFlags = 1 << 17;
            assert_eq!(
                with_command_flag(shift_flag),
                shift_flag | K_CG_EVENT_FLAG_MASK_COMMAND
            );
            assert_eq!(
                with_command_flag(shift_flag | K_CG_EVENT_FLAG_MASK_COMMAND),
                shift_flag | K_CG_EVENT_FLAG_MASK_COMMAND
            );
        }

        #[test]
        fn parses_cli_modes() {
            assert_eq!(
                parse_args(Vec::<String>::new()).unwrap(),
                Command::Run { daemon: false }
            );
            assert_eq!(
                parse_args(["--daemon".to_string()]).unwrap(),
                Command::Run { daemon: true }
            );
            assert_eq!(
                parse_args(["run".to_string()]).unwrap(),
                Command::Run { daemon: false }
            );
            assert_eq!(
                parse_args(["run".to_string(), "--daemon".to_string()]).unwrap(),
                Command::Run { daemon: true }
            );
            assert_eq!(parse_args(["start".to_string()]).unwrap(), Command::Start);
            assert_eq!(parse_args(["stop".to_string()]).unwrap(), Command::Stop);
            assert_eq!(
                parse_args(["restart".to_string()]).unwrap(),
                Command::Restart
            );
            assert_eq!(parse_args(["enable".to_string()]).unwrap(), Command::Enable);
            assert_eq!(
                parse_args(["disable".to_string()]).unwrap(),
                Command::Disable
            );
            assert_eq!(parse_args(["status".to_string()]).unwrap(), Command::Status);
            assert_eq!(parse_args(["--help".to_string()]).unwrap(), Command::Help);
            assert_eq!(
                parse_args(["--version".to_string()]).unwrap(),
                Command::Version
            );
            assert!(parse_args(["--wat".to_string()]).is_err());
            assert!(parse_args(["start".to_string(), "extra".to_string()]).is_err());
            assert!(parse_args(["run".to_string(), "--nope".to_string()]).is_err());
        }

        #[test]
        fn rendered_plist_has_expected_structure() {
            let plist = service::render_plist().expect("plist should render");
            assert!(plist.contains("<key>Label</key>"));
            assert!(plist.contains(service::LABEL));
            assert!(plist.contains("<string>run</string>"));
            assert!(plist.contains("<string>--daemon</string>"));
            assert!(plist.contains("<key>RunAtLoad</key>"));
            assert!(plist.contains("<key>KeepAlive</key>"));
            assert!(plist.trim_start().starts_with("<?xml"));
        }

        #[test]
        fn rendered_plist_passes_plutil_lint() {
            let plist = service::render_plist().expect("plist should render");
            let path = std::env::temp_dir()
                .join(format!("hjkl-for-mac-test-{}.plist", std::process::id()));
            std::fs::write(&path, plist).expect("write temp plist");

            let output = std::process::Command::new("plutil")
                .arg("-lint")
                .arg(&path)
                .output();
            let _ = std::fs::remove_file(&path);

            // plutil is macOS-only; skip silently where it is unavailable.
            if let Ok(out) = output {
                assert!(
                    out.status.success(),
                    "plutil -lint failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        }
    }
}
