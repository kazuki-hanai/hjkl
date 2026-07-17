//! Self-contained management of the per-user launchd LaunchAgent, so the
//! binary can offer `start`/`stop`/`restart`/`enable`/`disable`/`status`
//! without any external shell scripts.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as SysCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::cli::COMMAND_NAME;
use crate::error::{Error, Result};
use crate::macos::accessibility;

pub(crate) const LABEL: &str = "com.kazuki-hanai.hjkl-for-mac";

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

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| Error::from("HOME environment variable is not set"))
}

/// Plist under `~/Library/LaunchAgents`. Its mere presence there is what
/// makes launchd auto-load the agent at login, i.e. "enabled".
fn launch_agents_plist() -> Result<PathBuf> {
    Ok(home_dir()?
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist")))
}

/// Plist used by `start` when the agent is not enabled. It lives outside
/// `LaunchAgents` so launchd runs it now but not automatically at login.
fn runtime_plist() -> Result<PathBuf> {
    Ok(home_dir()?
        .join("Library/Application Support")
        .join(LABEL)
        .join(format!("{LABEL}.plist")))
}

fn log_paths() -> Result<(PathBuf, PathBuf)> {
    let dir = home_dir()?.join("Library/Logs");
    Ok((dir.join("hjkl.log"), dir.join("hjkl.err.log")))
}

fn binary_path() -> Result<PathBuf> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    Ok(exe.canonicalize().unwrap_or(exe))
}

fn path_to_str(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| Error::from(format!("path is not valid UTF-8: {}", path.display())))
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(crate) fn render_plist() -> Result<String> {
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
\t\t<string>--launchd</string>\n\
\t</array>\n\
\t<key>RunAtLoad</key>\n\
\t<true/>\n\
\t<key>KeepAlive</key>\n\
\t<true/>\n\
\t<key>ExitTimeOut</key>\n\
\t<integer>1</integer>\n\
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

fn write_plist_to(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let contents = render_plist()?;
    fs::write(path, contents)
        .map_err(|error| Error::from(format!("failed to write {}: {error}", path.display())))
}

fn ensure_log_dir() -> Result<()> {
    let (stdout_log, _) = log_paths()?;
    if let Some(dir) = stdout_log.parent() {
        fs::create_dir_all(dir)
            .map_err(|error| format!("failed to create {}: {error}", dir.display()))?;
    }
    Ok(())
}

/// Ground-truth signal written by the launchd-run daemon itself, so the
/// management commands report whether keys are really being remapped
/// rather than merely whether launchd loaded the job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Health {
    Ok,
    TapFailed,
}

fn health_path() -> Result<PathBuf> {
    Ok(home_dir()?
        .join("Library/Application Support")
        .join(LABEL)
        .join("health"))
}

/// Best-effort: recording health must never break the daemon.
pub(crate) fn write_health(status: Health) {
    let Ok(path) = health_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let token = match status {
        Health::Ok => "ok",
        Health::TapFailed => "tap_failed",
    };
    let pid = std::process::id();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let _ = fs::write(&path, format!("{token} {pid} {ts}\n"));
}

pub(crate) fn clear_health() {
    if let Ok(path) = health_path() {
        let _ = fs::remove_file(path);
    }
}

pub(crate) fn parse_health_record(contents: &str) -> Option<(Health, u32)> {
    let mut parts = contents.split_whitespace();
    let status = match parts.next()? {
        "ok" => Health::Ok,
        "tap_failed" => Health::TapFailed,
        _ => return None,
    };
    let pid = parts.next()?.parse().ok()?;
    Some((status, pid))
}

fn current_service_pid() -> Option<u32> {
    let output = SysCommand::new("launchctl")
        .args(["print", &service_target()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().find_map(|line| {
        line.trim()
            .strip_prefix("pid = ")
            .and_then(|pid| pid.parse().ok())
    })
}

fn read_health() -> Option<Health> {
    let path = health_path().ok()?;
    let contents = fs::read_to_string(path).ok()?;
    let (health, pid) = parse_health_record(&contents)?;
    if current_service_pid() == Some(pid) {
        Some(health)
    } else {
        None
    }
}

fn launchctl(args: &[&str]) -> Result<()> {
    let status = SysCommand::new("launchctl")
        .args(args)
        .status()
        .map_err(|error| format!("failed to run launchctl: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::from(format!(
            "`launchctl {}` failed ({})",
            args.join(" "),
            status
                .code()
                .map(|code| format!("exit code {code}"))
                .unwrap_or_else(|| "terminated by signal".to_string()),
        )))
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

const VERIFY_READY_TIMEOUT: Duration = Duration::from_secs(2);

/// Poll the daemon's health signal to confirm it actually started
/// remapping keys. This is written by the launchd-run process itself,
/// so it stays accurate even when the controlling terminal has
/// different permissions than the installed binary.
fn verify_ready() -> Result<()> {
    let deadline = Instant::now() + VERIFY_READY_TIMEOUT;
    loop {
        match read_health() {
            Some(Health::Ok) => return Ok(()),
            Some(Health::TapFailed) => return Err(not_ready_message().into()),
            None => {}
        }

        if Instant::now() >= deadline {
            break;
        }
        thread::sleep(Duration::from_millis(150));
    }

    if !is_loaded() {
        return Err(Error::from(format!(
            "The launchd service did not stay loaded.\n\
             Check `{COMMAND_NAME} status` and the logs."
        )));
    }

    Err(Error::from(format!(
        "Could not confirm key remapping started within {} seconds.\n\
         Check `{COMMAND_NAME} status` and the logs.",
        VERIFY_READY_TIMEOUT.as_secs()
    )))
}

fn not_ready_message() -> String {
    let binary = binary_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| format!("the {COMMAND_NAME} binary"));

    let mut message = String::new();
    message.push_str("The service is loaded, but key remapping is NOT active.\n");
    if accessibility::is_trusted() {
        message.push_str(
            "macOS says Accessibility is granted, but it still denied the keyboard tap.\n",
        );
        message.push_str("This often happens after rebuilding/reinstalling the binary.\n\n");
        message.push_str("Remove and re-add this binary in:\n");
        message.push_str("  System Settings -> Privacy & Security -> Accessibility\n");
        message.push_str("Also allow it in Input Monitoring if that section lists it.\n");
        message.push_str(&format!("Then run `{COMMAND_NAME} restart`.\n"));
        message.push_str("Binary:\n");
        message.push_str(&format!("  {binary}"));
    } else {
        message.push_str("macOS has not allowed this binary to read keyboard events.\n\n");
        message.push_str(&format!(
            "Grant permission, then run `{COMMAND_NAME} restart`:\n"
        ));
        message.push_str("  System Settings -> Privacy & Security -> Accessibility\n");
        message.push_str("  (and Input Monitoring, if it is listed)\n");
        message.push_str("Allow this binary:\n");
        message.push_str(&format!("  {binary}"));
    }
    message
}

/// Clear any stale health, reload the agent, then confirm
/// it actually started remapping keys.
fn reload_and_verify(plist: &Path) -> Result<()> {
    clear_health();
    launchctl_quiet(&["bootout", &service_target()]);
    launchctl_quiet(&["enable", &service_target()]);
    launchctl(&["bootstrap", &domain_target(), path_to_str(plist)?])?;
    verify_ready()
}

pub(crate) fn start() -> Result<()> {
    ensure_log_dir()?;
    let _ = accessibility::request_prompt();

    let launch_agents = launch_agents_plist()?;
    let enabled = launch_agents.exists();
    let plist = if enabled {
        launch_agents
    } else {
        let runtime = runtime_plist()?;
        write_plist_to(&runtime)?;
        runtime
    };

    let result = reload_and_verify(&plist);
    match &result {
        Ok(()) => println!("{COMMAND_NAME} started; key remapping is active now."),
        Err(_) => println!("{COMMAND_NAME} was loaded, but it is NOT working yet."),
    }
    if !enabled {
        println!("It will NOT auto-start at login. Run `{COMMAND_NAME} enable` for that.");
    }
    result
}

pub(crate) fn enable() -> Result<()> {
    ensure_log_dir()?;
    let _ = accessibility::request_prompt();

    let plist = launch_agents_plist()?;
    write_plist_to(&plist)?;

    let result = reload_and_verify(&plist);
    match &result {
        Ok(()) => println!(
            "{COMMAND_NAME} enabled: it will auto-start at login and key remapping is active now."
        ),
        Err(_) => println!(
            "{COMMAND_NAME} enabled: it will auto-start at login, but it is NOT working yet."
        ),
    }
    result
}

pub(crate) fn stop() -> Result<()> {
    if !is_loaded() {
        clear_health();
        println!("{COMMAND_NAME} is not running.");
        return Ok(());
    }
    launchctl(&["bootout", &service_target()])?;
    clear_health();
    println!("{COMMAND_NAME} stopped.");
    Ok(())
}

pub(crate) fn restart() -> Result<()> {
    let _ = accessibility::request_prompt();

    let plist = if launch_agents_plist()?.exists() {
        launch_agents_plist()?
    } else if runtime_plist()?.exists() {
        runtime_plist()?
    } else {
        println!("{COMMAND_NAME} was not running; starting it...");
        return start();
    };

    let result = reload_and_verify(&plist);
    match &result {
        Ok(()) => println!("{COMMAND_NAME} restarted; key remapping is active now."),
        Err(_) => println!("{COMMAND_NAME} restarted, but it is NOT working yet."),
    }
    result
}

pub(crate) fn disable() -> Result<()> {
    launchctl_quiet(&["bootout", &service_target()]);
    launchctl_quiet(&["disable", &service_target()]);
    clear_health();

    let launch_agents = launch_agents_plist()?;
    if launch_agents.exists() {
        fs::remove_file(&launch_agents).map_err(|error| {
            Error::from(format!(
                "failed to remove {}: {error}",
                launch_agents.display()
            ))
        })?;
    }
    if let Ok(runtime) = runtime_plist() {
        let _ = fs::remove_file(runtime);
    }

    println!("{COMMAND_NAME} disabled: it will not auto-start at login and is stopped.");
    Ok(())
}

pub(crate) fn status() -> Result<()> {
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
    let loaded = is_loaded();
    println!("running: {}", if loaded { "yes" } else { "no" });
    println!(
        "key remapping: {}",
        if loaded {
            match read_health() {
                Some(Health::Ok) => "active",
                Some(Health::TapFailed) => "not active (permission needed)",
                None => "unknown",
            }
        } else {
            "not active (not running)"
        }
    );
    println!(
        "accessibility: {}",
        if accessibility::is_trusted() {
            "granted"
        } else {
            "not granted"
        }
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_plist_has_expected_structure() {
        let plist = render_plist().expect("plist should render");
        assert!(plist.contains("<key>Label</key>"));
        assert!(plist.contains(LABEL));
        assert!(plist.contains("<string>run</string>"));
        assert!(plist.contains("<string>--launchd</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<key>KeepAlive</key>"));
        assert!(plist.trim_start().starts_with("<?xml"));
    }

    #[test]
    fn rendered_plist_passes_plutil_lint() {
        let plist = render_plist().expect("plist should render");
        let path = std::env::temp_dir().join(format!("hjkl-test-{}.plist", std::process::id()));
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

    #[test]
    fn parses_health_records() {
        assert_eq!(parse_health_record("ok 123 456\n"), Some((Health::Ok, 123)));
        assert_eq!(
            parse_health_record("tap_failed 7 8"),
            Some((Health::TapFailed, 7))
        );
        assert!(parse_health_record("").is_none());
        assert!(parse_health_record("weird").is_none());
        assert!(parse_health_record("ok").is_none());
        assert!(parse_health_record("ok nope 123").is_none());
    }
}
