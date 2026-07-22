//! Windows background-process and auto-start management.

use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command as SysCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::cli::COMMAND_NAME;
use crate::error::{Error, Result};
use crate::keymap::{self, KeyCode};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Storage::FileSystem::SYNCHRONIZE;
use windows_sys::Win32::System::Threading::{
    CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, DETACHED_PROCESS, OpenProcess, PROCESS_TERMINATE,
    TerminateProcess, WaitForSingleObject,
};

pub(crate) const LABEL: &str = "com.kazuki-hanai.hjkl-for-mac";
const TASK_NAME: &str = "hjkl-for-mac";
const VERIFY_READY_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Health {
    Ok,
    HookFailed,
}

fn local_app_data() -> Result<PathBuf> {
    let path = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| Error::from("LOCALAPPDATA environment variable is not set"))?;
    if !path.is_absolute() {
        return Err(Error::from(format!(
            "LOCALAPPDATA must be an absolute path, got: {}",
            path.display()
        )));
    }
    Ok(path)
}

fn app_data_dir() -> Result<PathBuf> {
    Ok(local_app_data()?.join(LABEL))
}

fn health_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("health"))
}

fn config_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("layer-key"))
}

fn binary_path() -> Result<PathBuf> {
    let exe = std::env::current_exe()
        .map_err(|error| Error::from(format!("failed to resolve current executable: {error}")))?;
    Ok(exe.canonicalize().unwrap_or(exe))
}

fn ensure_app_data_dir() -> Result<()> {
    let dir = app_data_dir()?;
    fs::create_dir_all(&dir)
        .map_err(|error| Error::from(format!("failed to create {}: {error}", dir.display())))
}

fn read_configured_layer_key() -> Option<KeyCode> {
    let code: KeyCode = fs::read_to_string(config_path().ok()?)
        .ok()?
        .trim()
        .parse()
        .ok()?;
    keymap::parse_layer_key(&code.to_string()).ok()
}

fn write_configured_layer_key(layer_key: Option<KeyCode>) -> Result<()> {
    ensure_app_data_dir()?;
    let path = config_path()?;
    match layer_key {
        Some(code) => fs::write(&path, format!("{code}\n"))
            .map_err(|error| Error::from(format!("failed to write {}: {error}", path.display()))),
        None => {
            let _ = fs::remove_file(path);
            Ok(())
        }
    }
}

fn resolve_layer_key(explicit: Option<KeyCode>) -> Option<KeyCode> {
    explicit.or_else(read_configured_layer_key)
}

pub(crate) fn write_health(status: Health) {
    let Ok(path) = health_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let token = match status {
        Health::Ok => "ok",
        Health::HookFailed => "hook_failed",
    };
    let pid = std::process::id();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let _ = fs::write(&path, format!("{token} {pid} {ts}\n"));
}

fn clear_health() {
    if let Ok(path) = health_path() {
        let _ = fs::remove_file(path);
    }
}

fn parse_health_record(contents: &str) -> Option<(Health, u32)> {
    let mut parts = contents.split_whitespace();
    let status = match parts.next()? {
        "ok" => Health::Ok,
        "hook_failed" => Health::HookFailed,
        _ => return None,
    };
    let pid = parts.next()?.parse().ok()?;
    Some((status, pid))
}

fn read_health_record() -> Option<(Health, u32)> {
    let contents = fs::read_to_string(health_path().ok()?).ok()?;
    let record = parse_health_record(&contents)?;
    if process_exists(record.1) {
        Some(record)
    } else {
        None
    }
}

fn open_process(pid: u32, access: u32) -> Option<HANDLE> {
    let handle = unsafe { OpenProcess(access, 0, pid) };
    if handle.is_null() { None } else { Some(handle) }
}

fn process_exists(pid: u32) -> bool {
    let Some(handle) = open_process(pid, SYNCHRONIZE) else {
        return false;
    };
    unsafe {
        CloseHandle(handle);
    }
    true
}

fn terminate_process(pid: u32) -> Result<()> {
    let Some(handle) = open_process(pid, PROCESS_TERMINATE | SYNCHRONIZE) else {
        return Ok(());
    };

    let terminated = unsafe { TerminateProcess(handle, 0) != 0 };
    if terminated {
        unsafe {
            WaitForSingleObject(handle, 2_000);
        }
    }
    unsafe {
        CloseHandle(handle);
    }

    if terminated {
        Ok(())
    } else {
        Err(Error::from(format!("failed to stop process {pid}")))
    }
}

fn task_exists() -> bool {
    SysCommand::new("schtasks")
        .args(["/Query", "/TN", TASK_NAME])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn schtasks(args: &[&str]) -> Result<()> {
    let status = SysCommand::new("schtasks")
        .args(args)
        .status()
        .map_err(|error| Error::from(format!("failed to run schtasks: {error}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::from(format!(
            "`schtasks {}` failed ({})",
            args.join(" "),
            status
                .code()
                .map(|code| format!("exit code {code}"))
                .unwrap_or_else(|| "terminated by signal".to_string()),
        )))
    }
}

fn schtasks_quiet(args: &[&str]) {
    let _ = SysCommand::new("schtasks")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn run_args(layer_key: Option<KeyCode>) -> Vec<String> {
    let mut args = vec!["run".to_string(), "--service".to_string()];
    if let Some(code) = layer_key {
        args.push("--layer-key".to_string());
        args.push(code.to_string());
    }
    args
}

fn quote_windows_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.chars().any(|c| c.is_whitespace() || c == '"') {
        return arg.to_string();
    }

    let mut quoted = String::from("\"");
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                quoted.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.extend(std::iter::repeat_n('\\', backslashes));
                quoted.push(ch);
                backslashes = 0;
            }
        }
    }
    quoted.extend(std::iter::repeat_n('\\', backslashes * 2));
    quoted.push('"');
    quoted
}

fn task_command_line(binary: &Path, layer_key: Option<KeyCode>) -> Result<String> {
    let binary = binary
        .to_str()
        .ok_or_else(|| Error::from(format!("path is not valid UTF-8: {}", binary.display())))?;
    let mut parts = vec![quote_windows_arg(binary)];
    parts.extend(run_args(layer_key).iter().map(|arg| quote_windows_arg(arg)));
    Ok(parts.join(" "))
}

fn create_task(layer_key: Option<KeyCode>) -> Result<()> {
    let binary = binary_path()?;
    let task_command = task_command_line(&binary, layer_key)?;
    let status = SysCommand::new("schtasks")
        .args(["/Create", "/TN", TASK_NAME, "/SC", "ONLOGON", "/TR"])
        .arg(task_command)
        .args(["/RL", "LIMITED", "/F"])
        .status()
        .map_err(|error| Error::from(format!("failed to run schtasks: {error}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::from(format!(
            "`schtasks /Create /TN {TASK_NAME} ...` failed ({})",
            status
                .code()
                .map(|code| format!("exit code {code}"))
                .unwrap_or_else(|| "terminated by signal".to_string()),
        )))
    }
}

fn spawn_background(layer_key: Option<KeyCode>) -> Result<()> {
    let binary = binary_path()?;
    let mut command = SysCommand::new(&binary);
    command
        .args(run_args(layer_key))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    command.spawn().map_err(|error| {
        Error::from(format!(
            "failed to start background process {}: {error}",
            binary.display()
        ))
    })?;
    Ok(())
}

fn stop_running_process_quiet() {
    if task_exists() {
        schtasks_quiet(&["/End", "/TN", TASK_NAME]);
    }
    if let Some((_, pid)) = read_health_record() {
        let _ = terminate_process(pid);
    }
    clear_health();
}

fn verify_ready() -> Result<()> {
    let deadline = Instant::now() + VERIFY_READY_TIMEOUT;
    loop {
        match read_health_record() {
            Some((Health::Ok, _)) => return Ok(()),
            Some((Health::HookFailed, _)) => {
                return Err(Error::from(
                    "The background process started, but the Windows keyboard hook is NOT active.",
                ));
            }
            None => {}
        }

        if Instant::now() >= deadline {
            break;
        }
        thread::sleep(Duration::from_millis(150));
    }

    Err(Error::from(format!(
        "Could not confirm key remapping started within {} seconds.\n\
         Check `{COMMAND_NAME} status`.",
        VERIFY_READY_TIMEOUT.as_secs()
    )))
}

fn print_layer_key(layer_key: Option<KeyCode>) {
    if let Some(code) = layer_key {
        println!("layer key: {}", keymap::layer_key_label(code));
    }
}

pub(crate) fn start(layer_key: Option<KeyCode>) -> Result<()> {
    let layer_key = resolve_layer_key(layer_key);
    write_configured_layer_key(layer_key)?;
    stop_running_process_quiet();

    if task_exists() {
        create_task(layer_key)?;
        schtasks(&["/Run", "/TN", TASK_NAME])?;
    } else {
        spawn_background(layer_key)?;
    }

    let result = verify_ready();
    match &result {
        Ok(()) => println!("{COMMAND_NAME} started; key remapping is active now."),
        Err(_) => println!("{COMMAND_NAME} was started, but it is NOT working yet."),
    }
    print_layer_key(layer_key);
    if !task_exists() {
        println!("It will NOT auto-start at login. Run `{COMMAND_NAME} enable` for that.");
    }
    result
}

pub(crate) fn enable(layer_key: Option<KeyCode>) -> Result<()> {
    let layer_key = resolve_layer_key(layer_key);
    write_configured_layer_key(layer_key)?;
    create_task(layer_key)?;
    stop_running_process_quiet();
    schtasks(&["/Run", "/TN", TASK_NAME])?;

    let result = verify_ready();
    match &result {
        Ok(()) => println!(
            "{COMMAND_NAME} enabled: it will auto-start at login and key remapping is active now."
        ),
        Err(_) => println!(
            "{COMMAND_NAME} enabled: it will auto-start at login, but it is NOT working yet."
        ),
    }
    print_layer_key(layer_key);
    result
}

pub(crate) fn stop() -> Result<()> {
    let running = read_health_record().is_some();
    if task_exists() {
        schtasks_quiet(&["/End", "/TN", TASK_NAME]);
    }
    if let Some((_, pid)) = read_health_record() {
        terminate_process(pid)?;
    }
    clear_health();

    if running {
        println!("{COMMAND_NAME} stopped.");
    } else {
        println!("{COMMAND_NAME} is not running.");
    }
    Ok(())
}

pub(crate) fn restart(layer_key: Option<KeyCode>) -> Result<()> {
    println!("{COMMAND_NAME} restarting...");
    start(layer_key)
}

pub(crate) fn disable() -> Result<()> {
    let _ = stop();
    if task_exists() {
        schtasks(&["/Delete", "/TN", TASK_NAME, "/F"])?;
    }
    let _ = fs::remove_file(config_path()?);
    println!("{COMMAND_NAME} disabled: it will not auto-start at login and is stopped.");
    Ok(())
}

pub(crate) fn status() -> Result<()> {
    let configured_layer_key = read_configured_layer_key().unwrap_or(keymap::DEFAULT_LAYER_KEY);
    let enabled = task_exists();
    let health = read_health_record();

    println!("label:   {LABEL}");
    println!(
        "layer key: {}",
        keymap::layer_key_label(configured_layer_key)
    );
    println!(
        "enabled: {}",
        if enabled {
            "yes (auto-start at login)"
        } else {
            "no"
        }
    );
    println!("running: {}", if health.is_some() { "yes" } else { "no" });
    println!(
        "key remapping: {}",
        match health {
            Some((Health::Ok, _)) => "active",
            Some((Health::HookFailed, _)) => "not active (hook failed)",
            None => "not active (not running)",
        }
    );
    println!("input access: available for the current desktop session");
    println!("task:    {TASK_NAME}");
    match binary_path() {
        Ok(binary) => println!("binary:  {}", binary.display()),
        Err(error) => println!("binary:  <unknown> ({error})"),
    }
    println!("state:   {}", app_data_dir()?.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_health_records() {
        assert_eq!(parse_health_record("ok 123 456\n"), Some((Health::Ok, 123)));
        assert_eq!(
            parse_health_record("hook_failed 7 8"),
            Some((Health::HookFailed, 7))
        );
        assert!(parse_health_record("").is_none());
        assert!(parse_health_record("weird").is_none());
        assert!(parse_health_record("ok").is_none());
        assert!(parse_health_record("ok nope 123").is_none());
    }

    #[test]
    fn quotes_windows_arguments_for_task_scheduler() {
        assert_eq!(quote_windows_arg("simple"), "simple");
        assert_eq!(quote_windows_arg("two words"), "\"two words\"");
        assert_eq!(quote_windows_arg(""), "\"\"");
        assert_eq!(quote_windows_arg("quote\"here"), "\"quote\\\"here\"");
        assert_eq!(
            quote_windows_arg(r"C:\Program Files\hjkl\"),
            r#""C:\Program Files\hjkl\\""#
        );
    }
}
