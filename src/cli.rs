//! Command-line argument parsing.

use crate::error::{Error, Result};
use crate::keymap::{self, KeyCode};

/// Name of the installed command, used in user-facing messages.
pub(crate) const COMMAND_NAME: &str = "hjkl";

/// A parsed layer-key selection. `None` means "use the built-in default".
pub(crate) type LayerKeyArg = Option<KeyCode>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Command {
    Run {
        service_mode: bool,
        layer_key: LayerKeyArg,
    },
    Start {
        layer_key: LayerKeyArg,
    },
    Stop,
    Restart {
        layer_key: LayerKeyArg,
    },
    Enable {
        layer_key: LayerKeyArg,
    },
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
            layer_key: None,
        });
    };

    match first.as_str() {
        // Legacy invocation kept for older LaunchAgent plists: `hjkl --daemon`.
        "--daemon" | "--launchd" => {
            ensure_no_extra_args(iter)?;
            Ok(Command::Run {
                service_mode: true,
                layer_key: None,
            })
        }
        "run" => {
            let (service_mode, layer_key) = parse_run_flags(iter, true)?;
            Ok(Command::Run {
                service_mode,
                layer_key,
            })
        }
        "start" => {
            let (_, layer_key) = parse_run_flags(iter, false)?;
            Ok(Command::Start { layer_key })
        }
        "restart" => {
            let (_, layer_key) = parse_run_flags(iter, false)?;
            Ok(Command::Restart { layer_key })
        }
        "enable" => {
            let (_, layer_key) = parse_run_flags(iter, false)?;
            Ok(Command::Enable { layer_key })
        }
        "stop" => ensure_no_extra_args(iter).map(|()| Command::Stop),
        "disable" => ensure_no_extra_args(iter).map(|()| Command::Disable),
        "status" => ensure_no_extra_args(iter).map(|()| Command::Status),
        "permissions" => ensure_no_extra_args(iter).map(|()| Command::Permissions),
        other => Err(unknown_argument(other)),
    }
}

/// Parse the flags shared by `run`/`start`/`restart`/`enable`: an optional
/// `--layer-key <name-or-code>` and, when `allow_service_mode` is set, the
/// internal `--launchd`/`--daemon` markers.
fn parse_run_flags<'a>(
    mut iter: impl Iterator<Item = &'a String>,
    allow_service_mode: bool,
) -> Result<(bool, LayerKeyArg)> {
    let mut service_mode = false;
    let mut layer_key = None;

    while let Some(arg) = iter.next() {
        let arg = arg.as_str();
        if allow_service_mode && (arg == "--launchd" || arg == "--daemon") {
            // `--daemon` is backward compatibility only; it does not detach
            // into the background (launchd manages the foreground process).
            service_mode = true;
        } else if arg == "--layer-key" {
            let value = iter.next().ok_or_else(|| {
                Error::from(format!(
                    "--layer-key requires a value (a key name like 'quote' or a \
                     macOS key code).\n\nRun `{COMMAND_NAME} --help` for usage."
                ))
            })?;
            layer_key = Some(parse_layer_key_value(value)?);
        } else if let Some(value) = arg.strip_prefix("--layer-key=") {
            layer_key = Some(parse_layer_key_value(value)?);
        } else {
            return Err(unknown_argument(arg));
        }
    }

    Ok((service_mode, layer_key))
}

fn parse_layer_key_value(value: &str) -> Result<KeyCode> {
    keymap::parse_layer_key(value).map_err(|message| {
        Error::from(format!(
            "invalid --layer-key: {message}\n\nRun `{COMMAND_NAME} --help` for usage."
        ))
    })
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

    fn parse(args: &[&str]) -> Result<Command> {
        parse_args(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_cli_modes() {
        assert_eq!(
            parse(&[]).unwrap(),
            Command::Run {
                service_mode: false,
                layer_key: None,
            }
        );
        assert_eq!(
            parse(&["--daemon"]).unwrap(),
            Command::Run {
                service_mode: true,
                layer_key: None,
            }
        );
        assert_eq!(
            parse(&["--launchd"]).unwrap(),
            Command::Run {
                service_mode: true,
                layer_key: None,
            }
        );
        assert_eq!(
            parse(&["run"]).unwrap(),
            Command::Run {
                service_mode: false,
                layer_key: None,
            }
        );
        assert_eq!(
            parse(&["run", "--launchd"]).unwrap(),
            Command::Run {
                service_mode: true,
                layer_key: None,
            }
        );
        assert_eq!(
            parse(&["run", "--daemon"]).unwrap(),
            Command::Run {
                service_mode: true,
                layer_key: None,
            }
        );
        assert_eq!(
            parse(&["start"]).unwrap(),
            Command::Start { layer_key: None }
        );
        assert_eq!(parse(&["stop"]).unwrap(), Command::Stop);
        assert_eq!(
            parse(&["restart"]).unwrap(),
            Command::Restart { layer_key: None }
        );
        assert_eq!(
            parse(&["enable"]).unwrap(),
            Command::Enable { layer_key: None }
        );
        assert_eq!(parse(&["disable"]).unwrap(), Command::Disable);
        assert_eq!(parse(&["status"]).unwrap(), Command::Status);
        assert_eq!(parse(&["permissions"]).unwrap(), Command::Permissions);
        assert_eq!(parse(&["--help"]).unwrap(), Command::Help);
        assert_eq!(parse(&["--version"]).unwrap(), Command::Version);
        assert!(parse(&["--wat"]).is_err());
        assert!(parse(&["start", "extra"]).is_err());
        assert!(parse(&["run", "--nope"]).is_err());
    }

    #[test]
    fn parses_layer_key_flag() {
        assert_eq!(
            parse(&["enable", "--layer-key", "quote"]).unwrap(),
            Command::Enable {
                layer_key: Some(39),
            }
        );
        assert_eq!(
            parse(&["start", "--layer-key=grave"]).unwrap(),
            Command::Start {
                layer_key: Some(50),
            }
        );
        assert_eq!(
            parse(&["restart", "--layer-key", "39"]).unwrap(),
            Command::Restart {
                layer_key: Some(39),
            }
        );
        assert_eq!(
            parse(&["run", "--launchd", "--layer-key", "quote"]).unwrap(),
            Command::Run {
                service_mode: true,
                layer_key: Some(39),
            }
        );
    }

    #[test]
    fn rejects_bad_layer_key_flag() {
        // Missing value.
        assert!(parse(&["enable", "--layer-key"]).is_err());
        // Unknown name.
        assert!(parse(&["enable", "--layer-key", "banana"]).is_err());
        // Modifier key code.
        assert!(parse(&["enable", "--layer-key", "57"]).is_err());
        // stop/status/disable take no layer key.
        assert!(parse(&["stop", "--layer-key", "quote"]).is_err());
    }
}
