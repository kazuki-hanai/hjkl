//! Maps a parsed CLI command onto the platform services.

use crate::cli::{self, COMMAND_NAME, Command};
use crate::error::Result;
use crate::help;
use crate::macos::{accessibility, event_tap, service};

pub(crate) fn run() -> Result<()> {
    match cli::parse_args(std::env::args().skip(1))? {
        Command::Run { service_mode } => event_tap::run_event_loop(service_mode),
        Command::Start => service::start(),
        Command::Stop => service::stop(),
        Command::Restart => service::restart(),
        Command::Enable => service::enable(),
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
