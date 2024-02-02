use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs, io};
use clap::{CommandFactory, Parser};
use walkdir::WalkDir;

mod util;
mod args;

const GRAVEYARD: &str = "/tmp/graveyard";
const RECORD: &str = ".record";
const LINES_TO_INSPECT: usize = 6;
const FILES_TO_INSPECT: usize = 6;
const BIG_FILE_THRESHOLD: u64 = 500000000; // 500 MB

struct RecordItem<'a> {
    _time: &'a str,
    orig: &'a Path,
    dest: &'a Path,
}

fn main() -> ExitCode {
    if let Err(ref e) = run() {
        println!("Exception: {}", e);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<(), Error> {
    let cli = args::Args::parse();
        
    // This selects the location of deleted
    // files based on the following order (from
    // first choice to last):
    // 1. Path passed with --graveyard
    // 2. Path pointed by the $GRAVEYARD variable
    // 3. $XDG_DATA_HOME/graveyard (only if XDG_DATA_HOME is defined)
    // 4. /tmp/graveyard-user
    let graveyard: PathBuf = {
        if let Some(flag) = cli.graveyard {
            flag
        } else if let Ok(env) = env::var("GRAVEYARD") {
            PathBuf::from(env)
        } else if let Ok(mut env) = env::var("XDG_DATA_HOME") {
            if !env.ends_with(std::path::MAIN_SEPARATOR) {
                env.push(std::path::MAIN_SEPARATOR);
            }
            env.push_str("graveyard");
            PathBuf::from(env)
        } else {
            PathBuf::from(format!("{}-{}", GRAVEYARD, util::get_user()))
        }
    }
    .into();

    if !graveyard.exists() {
        if let Err(e) = fs::create_dir(&graveyard){
            return Err(e);
        }
        let metadata = graveyard.metadata();
        if let Err(e) = metadata {
            return Err(e);
        }
        let mut permissions = metadata.unwrap().permissions();
        permissions.set_mode(0o700);
    }

    // If the user wishes to restore everything
    if cli.decompose {
        if util::prompt_yes("Really unlink the entire graveyard?") {
            if let Err(e) = fs::remove_dir_all(graveyard) {
                return Err(Error::new(e.kind(), "Couldn't unlink graveyard"));
            }
        }
        return Ok(());
    }

    // Stores the deleted files
    let record: &Path = &graveyard.join(RECORD);
    let cwd: PathBuf = match env::current_dir() {
        Ok(path) => path,
        Err(e) => return Err(e),
    };

    if let Some(t) = cli.unbury {
        // Vector to hold the grave path of items we want to unbury.
        // This will be used to determine which items to remove from the
        // record following the unbury.
        // Initialize it with the targets passed to -r
        let mut graves_to_exhume: Vec<PathBuf> = t.iter().map(PathBuf::from).collect();

        // If -s is also passed, push all files found by seance onto
        // the graves_to_exhume.
        if cli.seance {
            if let Ok(f) = fs::File::open(record) {
                let gravepath = util::join_absolute(graveyard, cwd)
                    .to_string_lossy()
                    .into_owned();
                for grave in seance(f, gravepath) {
                    graves_to_exhume.push(grave);
                }
            }
        }

        // Otherwise, add the last deleted file
        if graves_to_exhume.is_empty() {
            if let Ok(s) = get_last_bury(record) {
                graves_to_exhume.push(s);
            }
        }

        // Go through the graveyard and exhume all the graves
        let f = match fs::File::open(record) {
            Ok(file) => file,
            Err(err) => return Err(err),
        };

        for line in lines_of_graves(f, &graves_to_exhume) {
            let entry: RecordItem = record_entry(&line);
            let orig: PathBuf = match util::symlink_exists(entry.orig) {
                true => util::rename_grave(entry.orig),
                false => PathBuf::from(entry.orig),
            };

            if let Err(e) = bury(entry.dest, &orig) {
                return Err(Error::new(
                    e.kind(),
                    format!(
                        "Unbury failed: couldn't copy files from {} to {}",
                        entry.dest.display(),
                        orig.display()
                    ),
                ));
            };
            println!("Returned {} to {}", entry.dest.display(), orig.display());
        }

        // Reopen the record and then delete lines corresponding to exhumed graves
        if let Err(e) = fs::File::open(record)
            .and_then(|f| delete_lines_from_record(f, record, &graves_to_exhume))
        {
            return Err(Error::new(
                e.kind(),
                format!("Failed to remove unburied files from record: {}", e),
            ));
        }
        return Ok(());
    }

    if cli.seance {
        let gravepath = util::join_absolute(graveyard, cwd);
        let f = fs::File::open(record);
        if let Err(_) = f {
            return Err(Error::new(ErrorKind::NotFound, "Failed to read record!"));
        }
        for grave in seance(f.unwrap(), gravepath.to_string_lossy()) {
            println!("{}", grave.display());
        }
        return Ok(());
    }

    if !cli.targets.is_empty() {
        for target in cli.targets {
            // Check if source exists
            if let Ok(metadata) = fs::symlink_metadata(&target) {
                // Canonicalize the path unless it's a symlink
                let source = &if !metadata.file_type().is_symlink() {
                    let cwd = cwd.join(&target).canonicalize();
                    if let Err(e) = cwd {
                        return Err(Error::new(e.kind(), "Failed to canonicalize path"));
                    }
                    cwd.unwrap()
                } else {
                    cwd.join(&target)
                };

                if cli.inspect {
                    if metadata.is_dir() {
                        // Get the size of the directory and all its contents
                        println!(
                            "{}: directory, {} including:",
                            target.to_str().unwrap(),
                            util::humanize_bytes(
                                WalkDir::new(source)
                                    .into_iter()
                                    .filter_map(|x| x.ok())
                                    .filter_map(|x| x.metadata().ok())
                                    .map(|x| x.len())
                                    .sum::<u64>()
                            )
                        );

                        // Print the first few top-level files in the directory
                        for entry in WalkDir::new(source)
                            .min_depth(1)
                            .max_depth(1)
                            .into_iter()
                            .filter_map(|entry| entry.ok())
                            .take(FILES_TO_INSPECT)
                        {
                            println!("{}", entry.path().display());
                        }
                    } else {
                        println!("{}: file, {}", &target.to_str().unwrap(), util::humanize_bytes(metadata.len()));
                        // Read the file and print the first few lines
                        if let Ok(f) = fs::File::open(source) {
                            for line in BufReader::new(f)
                                .lines()
                                .take(LINES_TO_INSPECT)
                                .filter_map(|line| line.ok())
                            {
                                println!("> {}", line);
                            }
                        } else {
                            println!("Error reading {}", source.display());
                        }
                    }
                    if !util::prompt_yes(format!("Send {} to the graveyard?", target.to_str().unwrap())) {
                        continue;
                    }
                }

                // If rip is called on a file already in the graveyard, prompt
                // to permanently delete it instead.
                if source.starts_with(&graveyard) {
                    println!("{} is already in the graveyard.", source.display());
                    if util::prompt_yes("Permanently unlink it?") {
                        if fs::remove_dir_all(source).is_err() {
                            if let Err(e) = fs::remove_file(source) {
                                return Err(Error::new(e.kind(), "Couldn't unlink!"));
                            }
                        }
                        continue;
                    } else {
                        println!("Skipping {}", source.display());
                        return Ok(());
                    }
                }

                let dest: &Path = &{
                    let dest = util::join_absolute(&graveyard, source);
                    // Resolve a name conflict if necessary
                    if util::symlink_exists(&dest) {
                        util::rename_grave(dest)
                    } else {
                        dest
                    }
                };

                {
                    let res = bury(source, dest).or_else(|e| {
                        fs::remove_dir_all(dest).ok();
                        Err(e)
                    });
                    if let Err(e) = res {
                        return Err(Error::new(e.kind(), "Failed to bury file"));
                    }
                }
                // Clean up any partial buries due to permission error
                if let Err(e) = write_log(source, dest, record) {
                    return Err(Error::new(
                        e.kind(),
                        format!("Failed to write record at {}", record.display()),
                    ));
                }
            } else {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("Cannot remove {}: no such file or directory", target.to_str().unwrap()),
                ));
            }
        }
    } else {
        let _ = args::Args::command().print_help();
    }

    Ok(())
}

/// Write deletion history to record
fn write_log<S, D, R>(source: S, dest: D, record: R) -> io::Result<()>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
    R: AsRef<Path>,
{
    let (source, dest) = (source.as_ref(), dest.as_ref());
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(record)?;
    writeln!(
        f,
        "{}\t{}\t{}",
        time::now().ctime(),
        source.display(),
        dest.display()
    )?;

    Ok(())
}

fn bury<S: AsRef<Path>, D: AsRef<Path>>(source: S, dest: D) -> Result<(), Error> {
    let (source, dest) = (source.as_ref(), dest.as_ref());
    // Try a simple rename, which will only work within the same mount point.
    // Trying to rename across filesystems will throw errno 18.
    if fs::rename(source, dest).is_ok() {
        return Ok(());
    }

    // If that didn't work, then copy and rm.
    {
        let parent = dest.parent();
        if let None = parent {
            return Err(Error::new(
                ErrorKind::NotFound,
                "Could not get parent of dest!",
            ));
        }

        let parent = parent.unwrap();
        if let Err(e) = fs::create_dir_all(parent) {
            return Err(e);
        }
    }

    let sym_link_data = fs::symlink_metadata(source);
    if let Err(e) = sym_link_data {
        return Err(e);
    }
    let sym_link_data = sym_link_data.unwrap();
    if sym_link_data.is_dir() {
        // Walk the source, creating directories and copying files as needed
        for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
            // Path without the top-level directory
            let orphan: &Path = match entry.path().strip_prefix(source) {
                Ok(p) => p,
                Err(_) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Parent directory isn't a prefix of child directories?",
                    ))
                }
            };

            if entry.file_type().is_dir() {
                if let Err(e) = fs::create_dir_all(dest.join(orphan)) {
                    return Err(Error::new(
                        e.kind(),
                        format!(
                            "Failed to create {} in {}",
                            entry.path().display(),
                            dest.join(orphan).display()
                        ),
                    ));
                }
            } else {
                if let Err(e) = copy_file(entry.path(), dest.join(orphan)) {
                    return Err(Error::new(
                        e.kind(),
                        format!(
                            "Failed to copy file from {} to {}",
                            entry.path().display(),
                            dest.join(orphan).display()
                        ),
                    ));
                }
            }
        }
        if let Err(err) = fs::remove_dir_all(source) {
            return Err(Error::new(
                err.kind(),
                format!("Failed to remove dir: {}", source.display()),
            ));
        }
    } else {
        if let Err(e) = copy_file(source, dest) {
            return Err(Error::new(
                e.kind(),
                format!(
                    "Failed to copy file from {} to {}",
                    source.display(),
                    dest.display()
                ),
            ));
        }
        if let Err(e) = fs::remove_file(source) {
            return Err(Error::new(
                e.kind(),
                format!("Failed to remove file: {}", source.display()),
            ));
        }
    }

    Ok(())
}

fn copy_file<S: AsRef<Path>, D: AsRef<Path>>(source: S, dest: D) -> Result<(), Error> {
    let (source, dest) = (source.as_ref(), dest.as_ref());
    let metadata = fs::symlink_metadata(source)?;
    let filetype = metadata.file_type();

    if metadata.len() > BIG_FILE_THRESHOLD {
        println!(
            "About to copy a big file ({} is {})",
            source.display(),
            util::humanize_bytes(metadata.len())
        );
        if util::prompt_yes("Permanently delete this file instead?") {
            return Ok(());
        }
    }

    if filetype.is_file() {
        if let Err(e) = fs::copy(source, dest) {
            // println!("Failed to copy {} to {}", source.display(), dest.display());
            return Err(e);
        }
    } else if filetype.is_fifo() {
        let mode = metadata.permissions().mode();
        std::process::Command::new("mkfifo")
            .arg(dest)
            .arg("-m")
            .arg(mode.to_string());
    } else if filetype.is_symlink() {
        let target = fs::read_link(source)?;
        std::os::unix::fs::symlink(target, dest)?;
    } else if let Err(e) = fs::copy(source, dest) {
        // Special file: Try copying it as normal, but this probably won't work
        println!("Non-regular file or directory: {}", source.display());
        if !util::prompt_yes("Permanently delete the file?") {
            return Err(e);
        }
        // Create a dummy file to act as a marker in the graveyard
        let mut marker = fs::File::create(dest)?;
        marker.write_all(
            b"This is a marker for a file that was \
                           permanently deleted.  Requiescat in pace.",
        )?;
    }

    Ok(())
}

/// Return the path in the graveyard of the last file to be buried.
/// As a side effect, any valid last files that are found in the record but
/// not on the filesystem are removed from the record.
fn get_last_bury<R: AsRef<Path>>(record: R) -> Result<PathBuf, Error> {
    let f = match fs::File::open(record.as_ref()) {
        Ok(file) => file,
        Err(err) => return Err(err),
    };

    let contents = {
        let path_f = PathBuf::from(record.as_ref());
        let string_f = fs::read_to_string(&path_f);
        if let Err(e) = string_f {
            return Err(e);
        }
        string_f.unwrap()
    };

    // This will be None if there is nothing, or Some
    // if there is items in the vector
    let mut graves_to_exhume: Vec<PathBuf> = Vec::new();
    for entry in contents.lines().rev().map(record_entry) {
        // Check that the file is still in the graveyard.
        // If it is, return the corresponding line.
        if util::symlink_exists(entry.dest) {
            if !graves_to_exhume.is_empty() {
                if let Err(e) = delete_lines_from_record(f, record, &graves_to_exhume) {
                    return Err(e);
                }
            }
            return Ok(PathBuf::from(entry.dest));
        } else {
            // File is gone, mark the grave to be removed from the record
            graves_to_exhume.push(PathBuf::from(entry.dest));
        }
    }

    if !graves_to_exhume.is_empty() {
        if let Err(e) = delete_lines_from_record(f, record, &graves_to_exhume) {
            return Err(e);
        }
    }
    Err(Error::new(ErrorKind::NotFound, "No files in graveyard"))
}

/// Parse a line in the record into a `RecordItem`
fn record_entry(line: &str) -> RecordItem {
    let mut tokens = line.split('\t');
    let time: &str = tokens.next().expect("Bad format: column A");
    let orig: &str = tokens.next().expect("Bad format: column B");
    let dest: &str = tokens.next().expect("Bad format: column C");
    RecordItem {
        _time: time,
        orig: Path::new(orig),
        dest: Path::new(dest),
    }
}

/// Takes a vector of grave paths and returns the respective lines in the record
fn lines_of_graves<'a>(f: fs::File, graves: &'a [PathBuf]) -> impl Iterator<Item = String> + 'a {
    BufReader::new(f)
        .lines()
        .filter_map(|l| l.ok())
        .filter(move |l| graves.into_iter().any(|y| y == record_entry(l).dest))
}

/// Returns an iterator over all graves in the record that are under gravepath
fn seance<T: AsRef<str>>(f: fs::File, gravepath: T) -> impl Iterator<Item = PathBuf> {
    BufReader::new(f)
        .lines()
        .filter_map(|l| l.ok())
        .map(|l| PathBuf::from(record_entry(&l).dest))
        .filter(move |d| d.starts_with(gravepath.as_ref()))
}

/// Takes a vector of grave paths and removes the respective lines from the record
fn delete_lines_from_record<R: AsRef<Path>>(
    current_record: fs::File,
    record: R,
    graves: &[PathBuf],
) -> Result<(), Error> {
    let record = record.as_ref();
    // Get the lines to write back to the record, which is every line except
    // the ones matching the exhumed graves.  Store them in a vector
    // since we'll be overwriting the record in-place.
    let lines_to_write: Vec<String> = BufReader::new(current_record)
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| !graves.into_iter().any(|y| y == record_entry(l).dest))
        .collect();
    let f = fs::File::create(record);
    if let Err(err) = f {
        return Err(err);
    }
    for line in lines_to_write {
        writeln!(f.as_ref().unwrap(), "{}", line)?;
    }
    Ok(())
}
