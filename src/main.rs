use std::process::ExitCode;

use clap::Parser;
use garimpo::{cli::Cli, error::Error};

fn main() -> ExitCode {
    let result = match Cli::try_parse() {
        Ok(cli) => garimpo::run(&cli),
        // Help and version also arrive here, and they are not errors.
        Err(clap) if !clap.use_stderr() => {
            print!("{clap}");
            Ok(())
        }
        Err(clap) => {
            let text = clap.to_string();
            Err(Error::Input(
                text.trim_start_matches("error: ").trim_end().to_owned(),
            ))
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("garimpo: {error:#}");
            eprintln!("próximo passo: {}", error.next_step());
            ExitCode::from(error.exit_code())
        }
    }
}
