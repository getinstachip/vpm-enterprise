mod cmd;
mod error;
//mod types;
// mod versions; will add once versiong is needed

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;

use crate::cmd::{Cmd, Execute};
use crate::error::SilentExit;

pub fn main() -> ExitCode {
    // Forcibly disable backtraces.
    env::remove_var("RUST_LIB_BACKTRACE");
    env::remove_var("RUST_BACKTRACE");

    match Cmd::parse().execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => match e.downcast::<SilentExit>() {
            Ok(SilentExit { code }) => code.into(),
            Err(e) => {
                _ = writeln!(io::stderr(), "vpm: {e:?}");
                ExitCode::FAILURE
            }
        },
    }
}
