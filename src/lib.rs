//! Core of the `garimpo` CLI: session, pacing, tree and writes on FamilySearch.
//!
//! The code is in English. What the agent sees (command and flag names, help,
//! messages and output) is in Portuguese, the language of the kit.

pub mod cli;
pub mod error;
pub mod familysearch;
mod pacing;
mod session;
mod transport;

use cli::{Cli, Command};
use error::Error;

/// Runs the command given on the command line.
///
/// # Errors
/// The command's [`Error`], which carries the exit code and the next step.
pub fn run(cli: &Cli) -> Result<(), Error> {
    match &cli.command {
        Command::Session(command) => session::run(&cli.config, command),
        Command::Fs(command) => familysearch::run(&cli.config, command),
    }
}
