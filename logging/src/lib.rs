extern crate proc_macros; /* to avoid a cargo bug when cross-compiling (e.g. wasm) */

pub use proc_macros::gen_log_error;
#[doc(hidden)] pub use ::slog; /* hide from doc since it is just a tool for your macros */

use sloggers::{
    Build,
    terminal::{TerminalLoggerBuilder, Destination},
    types::Severity
};
use slog::Logger;


pub fn build_file_logger() -> Logger {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);

    return builder.build().unwrap();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        crate::build_file_logger();
    }
}
