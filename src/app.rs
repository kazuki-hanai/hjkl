//! Maps a parsed CLI command onto the platform services.

use crate::cli::{self, COMMAND_NAME, Command};
use crate::error::Result;
use crate::help;
use crate::keymap::DEFAULT_LAYER_KEY;
use crate::platform;

pub(crate) fn run() -> Result<()> {
    match cli::parse_args(std::env::args().skip(1))? {
        Command::Run {
            service_mode,
            layer_key,
        } => {
            platform::set_layer_key(layer_key.unwrap_or(DEFAULT_LAYER_KEY));
            platform::run_event_loop(service_mode)
        }
        Command::Start { layer_key } => platform::start(layer_key),
        Command::Stop => platform::stop(),
        Command::Restart { layer_key } => platform::restart(layer_key),
        Command::Enable { layer_key } => platform::enable(layer_key),
        Command::Disable => platform::disable(),
        Command::Status => platform::status(),
        Command::Permissions => platform::request_permissions(),
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
