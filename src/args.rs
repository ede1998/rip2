use clap::Parser;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// File or directory to remove
    pub targets: Vec<PathBuf>,

    /// Directory where deleted files rest
    #[arg(long)]
    pub graveyard: Option<PathBuf>,

    /// Permanently deletes the graveyard
    #[arg(short, long)]
    pub decompose: bool,

    /// Deletes without confirmation
    #[arg(short, long)]
    pub force: bool,

    /// Prints files that were deleted
    /// in the current working directory
    #[arg(short, long)]
    pub seance: bool,

    /// Restore the specified
    /// files or the last file
    /// if none are specified
    #[arg(short, long, num_args = 0)]
    pub unbury: Option<Vec<PathBuf>>,

    /// Print some info about TARGET before
    /// burying
    #[arg(short, long)]
    pub inspect: bool,

    /// Generate shell completions file
    /// for the specified shell
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<String>,
}

struct IsDefault {
    graveyard: bool,
    decompose: bool,
    force: bool,
    seance: bool,
    unbury: bool,
    inspect: bool,
    completions: bool,
}
// Make this with ::new instead the proper RustLang way:
impl IsDefault {
    fn new(cli: &Args) -> IsDefault {
        IsDefault {
            graveyard: cli.graveyard.is_none(),
            decompose: !cli.decompose,
            force: !cli.force,
            seance: !cli.seance,
            unbury: cli.unbury.is_none(),
            inspect: !cli.inspect,
            completions: cli.completions.is_none(),
        }
    }
}

#[allow(clippy::nonminimal_bool)]
pub fn validate_args(cli: &Args) -> Result<(), Error> {
    let defaults = IsDefault::new(cli);

    // [completions] can only be used by itself
    if !defaults.completions
        && !(defaults.graveyard
            && defaults.decompose
            && defaults.force
            && defaults.seance
            && defaults.unbury
            && defaults.inspect)
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "--completions can only be used by itself",
        ));
    }
    // Furthermore, [force] and [decompose] only work with eachother
    if !defaults.force && !(defaults.seance && defaults.unbury && defaults.inspect) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "-f,--force can only be used with -d,--decompose and --graveyard",
        ));
    }
    if !defaults.decompose && !(defaults.seance && defaults.unbury && defaults.inspect) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "-d,--decompose can only be used with -f,--force and --graveyard",
        ));
    }

    Ok(())
}
