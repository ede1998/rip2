use clap::Parser;
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
    pub inspect: bool
}
