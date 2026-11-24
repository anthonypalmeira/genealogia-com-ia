//! The `garimpo` CLI: session, pacing, tree, writes, searches and images on
//! FamilySearch, with the coverage log of what was searched; the digitarq
//! archives; and the drawing of a lineage with its holes marked.
//!
//! The code is in English. What the agent sees (command and flag names, help,
//! messages and output) is in Portuguese, the language of the kit.

mod bridge;
pub mod cli;
mod coverage;
mod diagram;
mod digitarq;
pub mod error;
pub mod familysearch;
mod images;
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
        Command::Images(command) => images::run(&cli.config, command),
        Command::Digitarq(command) => digitarq::run(&cli.config, command),
        Command::Diagram(args) => diagram::run(&cli.config, args),
    }
}
