//! Windows input-permission guidance.

use crate::cli::COMMAND_NAME;
use crate::error::Result;

pub(crate) fn request_permissions() -> Result<()> {
    println!("Windows does not require an Accessibility-style permission for {COMMAND_NAME}.");
    println!(
        "If remapping does not affect applications running as administrator, run {COMMAND_NAME} as administrator too."
    );
    Ok(())
}
