//! Media collection API server.

#![warn(clippy::pedantic)]
// Allowed lints: clippy
#![expect(clippy::doc_markdown)]

use clap::Parser;

use crate::cli::{Cli, Command};
use crate::err::{Exit, Result};

pub mod axum;
pub mod cfg;
pub mod cli;
pub mod dir;
pub mod err;
pub mod exe;
pub mod log;

/// Application entry.
fn main() -> Exit {
    // Parse args
    let args = Cli::parse();
    // Initialize logger
    log::init(&args.log).unwrap_or_else(|err| eprintln!("warning: {err}"));
    tracing::trace!("{args:#?}");

    // Execute subcommand
    let out: Result<()> = match args.cmd {
        Command::Serve(cli) => {
            // media serve
            exe::serve::main(*cli)
        }
        Command::Dump(cli) => {
            // media dump
            exe::dump::main(*cli)
        }
        Command::Load(cli) => {
            // media load
            exe::load::main(*cli)
        }
    };

    // Return exit status
    match out {
        Ok(()) => Exit::Success,
        Err(e) => Exit::Failure(e),
    }
}
