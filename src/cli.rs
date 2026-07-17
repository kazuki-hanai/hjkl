//! Command-line argument parsing.

use crate::error::{Error, Result};

/// Name of the installed command, used in user-facing messages.
pub(crate) const COMMAND_NAME: &str = "hjkl";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Command {
    Run { service_mode: bool },
    Start,
    Stop,
    Restart,
    Enable,
    Disable,
    Status,
    Permissions,
    Help,
    Version,
}

pub(crate) fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command> {
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
        return Ok(Command::Run {
            service_mode: false,
        });
    };

    match first.as_str() {
        // Legacy invocation kept for older LaunchAgent plists: `hjkl --daemon`.
        "--daemon" | "--launchd" => {
            ensure_no_extra_args(iter)?;
            Ok(Command::Run { service_mode: true })
        }
        "run" => {
            let mut service_mode = false;
            for arg in iter {
                match arg.as_str() {
                    "--launchd" => service_mode = true,
                    // Backward compatibility only. This is intentionally
                    // not documented because it does not detach into the
                    // background; launchd does that by managing the
                    // foreground process.
                    "--daemon" => service_mode = true,
                    other => return Err(unknown_argument(other)),
                }
            }
            Ok(Command::Run { service_mode })
        }
        "start" => ensure_no_extra_args(iter).map(|()| Command::Start),
        "stop" => ensure_no_extra_args(iter).map(|()| Command::Stop),
        "restart" => ensure_no_extra_args(iter).map(|()| Command::Restart),
        "enable" => ensure_no_extra_args(iter).map(|()| Command::Enable),
        "disable" => ensure_no_extra_args(iter).map(|()| Command::Disable),
        "status" => ensure_no_extra_args(iter).map(|()| Command::Status),
        "permissions" => ensure_no_extra_args(iter).map(|()| Command::Permissions),
        other => Err(unknown_argument(other)),
    }
}

fn ensure_no_extra_args<'a>(mut iter: impl Iterator<Item = &'a String>) -> Result<()> {
    match iter.next() {
        Some(extra) => Err(unknown_argument(extra)),
        None => Ok(()),
    }
}

fn unknown_argument(arg: &str) -> Error {
    Error::from(format!(
        "Unknown argument: {arg}\n\nRun `{COMMAND_NAME} --help` for usage."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cli_modes() {
        assert_eq!(
            parse_args(Vec::<String>::new()).unwrap(),
            Command::Run {
                service_mode: false
            }
        );
        assert_eq!(
            parse_args(["--daemon".to_string()]).unwrap(),
            Command::Run { service_mode: true }
        );
        assert_eq!(
            parse_args(["--launchd".to_string()]).unwrap(),
            Command::Run { service_mode: true }
        );
        assert_eq!(
            parse_args(["run".to_string()]).unwrap(),
            Command::Run {
                service_mode: false
            }
        );
        assert_eq!(
            parse_args(["run".to_string(), "--launchd".to_string()]).unwrap(),
            Command::Run { service_mode: true }
        );
        assert_eq!(
            parse_args(["run".to_string(), "--daemon".to_string()]).unwrap(),
            Command::Run { service_mode: true }
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
        assert_eq!(
            parse_args(["permissions".to_string()]).unwrap(),
            Command::Permissions
        );
        assert_eq!(parse_args(["--help".to_string()]).unwrap(), Command::Help);
        assert_eq!(
            parse_args(["--version".to_string()]).unwrap(),
            Command::Version
        );
        assert!(parse_args(["--wat".to_string()]).is_err());
        assert!(parse_args(["start".to_string(), "extra".to_string()]).is_err());
        assert!(parse_args(["run".to_string(), "--nope".to_string()]).is_err());
    }
}
