use clap::Parser;
use std::io;
use std::process::ExitCode;

use env_logger::Env;
use rip2::{args, completions, util};

fn main() -> ExitCode {
    let env = Env::default().filter_or("RIP_LOG", "info");
    env_logger::init_from_env(env);

    let cli = args::Args::parse();
    let mut stream = io::stdout();
    let mode = util::ProductionMode;

    match &cli.command {
        Some(args::Commands::Completions { shell }) => {
            let result = completions::generate_shell_completions(shell, &mut io::stdout());
            if result.is_err() {
                eprintln!("{}", result.unwrap_err());
                return ExitCode::FAILURE;
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
