//! Maps a parsed CLI command onto the platform services.

use crate::cli::{self, COMMAND_NAME, Command};
use crate::error::Result;
use crate::help;
use crate::keymap::DEFAULT_LAYER_KEY;
use crate::macos::{accessibility, event_tap, remapper, service};

pub(crate) fn run() -> Result<()> {
    match cli::parse_args(std::env::args().skip(1))? {
        Command::Run {
            service_mode,
            layer_key,
        } => {
            remapper::set_layer_key(layer_key.unwrap_or(DEFAULT_LAYER_KEY));
            event_tap::run_event_loop(service_mode)
        }
        Command::Start { layer_key } => service::start(layer_key),
        Command::Stop => service::stop(),
        Command::Restart { layer_key } => service::restart(layer_key),
        Command::Enable { layer_key } => service::enable(layer_key),
        Command::Disable => service::disable(),
        Command::Status => service::status(),
        Command::Permissions => accessibility::request_permissions(),
        Command::Help => {
            help::print_help();
            Ok(())
        }
        Command::Version => {
            println!("{COMMAND_NAME} {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
