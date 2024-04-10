use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::{generate, Shell};
use clap_complete_nushell::Nushell;

use rip2::{args, util};
use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = args::Args::parse();

    if let Some(shell) = cli.completions.as_deref() {
        if ["nu", "nushell"].contains(&shell) {
            let shell = Nushell;
            generate(shell, &mut args::Args::command(), "rip", &mut io::stdout());
        } else {
            let shell = Shell::from_str(shell, true).unwrap_or_else(|_| {
                eprintln!("Invalid shell specification: {}", shell);
                std::process::exit(1);
            });
            generate(shell, &mut args::Args::command(), "rip", &mut io::stdout());
        }
        return ExitCode::SUCCESS;
    }

    let mode = util::ProductionMode;
    let mut stream = io::stdout();

    if let Err(ref e) = rip2::run(cli, mode, &mut stream) {
        println!("Exception: {}", e);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
