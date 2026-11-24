//! The `garimpo` CLI: session, pacing, tree, writes and searches on
//! FamilySearch, with the coverage log of what was searched.
//!
//! The code is in English. What the agent sees (command and flag names, help,
//! messages and output) is in Portuguese, the language of the kit.

mod bridge;
pub mod cli;
mod coverage;
pub mod error;
pub mod familysearch;
pub mod internal;
mod pacing;
mod session;
mod text;
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
        Command::Search(command) => internal::run(&cli.config, command),
        Command::Coverage(command) => coverage::run(&cli.config, command),
        Command::Bridge(command) => bridge::run(&cli.config, command),
    }
}
