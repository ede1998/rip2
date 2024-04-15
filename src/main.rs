use clap::Parser;
use std::env;
use std::io;
use std::process::ExitCode;

use rip2::args::Commands;
use rip2::{args, completions, util};

fn main() -> ExitCode {
    let cli = args::Args::parse();
    let mut stream = io::stdout();
    let mode = util::ProductionMode;

    match &cli.command {
        Some(Commands::Completions { shell }) => {
            let result = completions::generate_shell_completions(shell, &mut io::stdout());
            if result.is_err() {
                eprintln!("{}", result.unwrap_err());
                return ExitCode::FAILURE;
            }
            return ExitCode::SUCCESS;
        }
        Some(Commands::Graveyard { seance }) => {
            let graveyard = rip2::get_graveyard(None);
            if *seance {
                let cwd = &env::current_dir().unwrap();
                let gravepath = util::join_absolute(graveyard, dunce::canonicalize(cwd).unwrap());
                println!("{}", gravepath.display());
            } else {
                println!("{}", graveyard.display());
            }
            return ExitCode::SUCCESS;
        }
        None => {}
    }

    ////////////////////////////////////////////////////////////
    // Main code ///////////////////////////////////////////////
    let result = rip2::run(cli, mode, &mut stream);
    ////////////////////////////////////////////////////////////

    if let Err(ref e) = result {
        println!("Exception: {}", e);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
