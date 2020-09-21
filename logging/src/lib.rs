//! # Stackable Common: Logging
//!
//! This crate contains methods needed for all logging needs in the Stackable platform.
//!
//! Under the hood it uses the [slog](https://github.com/slog-rs/slog) crate to provide structured logging.
//!
//! It is our goal to have descriptive error messages that allow our users to get more information without relying on String parsing.
//!
//! That's why we provide macros that allow error and fatal messages to have unique error codes that can then be used to find more information in our online documentation.

extern crate proc_macros; /* to avoid a cargo bug when cross-compiling (e.g. wasm) */

pub use proc_macros::gen_log_error;

#[doc(hidden)]
pub use ::slog; /* hide from doc since we just need it for the generated macro */

pub use ::slog::debug;
pub use ::slog::info;
pub use ::slog::trace;
pub use ::slog::warn;

use slog::Logger;
use sloggers::{
    terminal::{Destination, TerminalLoggerBuilder},
    types::Severity,
    Build,
};

/// This returns a `slog` `Logger` instance which will print to stdout
/// It is currently hardcoded to print everything up to the _Debug_ level.
pub fn build_terminal_logger() -> Logger {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stdout);

    return builder
        .build()
        .expect("Creating the Logger failed, this should not happen; aborting");
}

#[cfg(test)]
mod tests {
    use slog::info;

    #[test]
    fn it_works() {
        let logger = crate::build_terminal_logger();
        info!(logger, "Test log message");
    }
}
