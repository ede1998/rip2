use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, BufReader, Error, Read, Write};
use std::path::Prefix::Disk;
use std::path::{Component, Path, PathBuf};
use std::str::from_utf8;

#[cfg(not(feature = "testing"))]
macro_rules! debug {
    // Empty macro regardless of arguments
    ($($arg:tt)*) => {};
}

#[cfg(feature = "testing")]
use std::println as debug;

fn hash_component(c: &Component) -> String {
    let mut hasher = DefaultHasher::new();
    c.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
fn str_component(c: &Component) -> String {
    match c {
        Component::Prefix(prefix) => match prefix.kind() {
            // C:\\ is the most common, so we just make a readable name for it.
            Disk(disk) => format!("DISK_{}", from_utf8(&[disk]).unwrap()),
            _ => hash_component(c),
        },
        _ => hash_component(c),
    }
}

/// Concatenate two paths, even if the right argument is an absolute path.
pub fn join_absolute<A: AsRef<Path>, B: AsRef<Path>>(left: A, right: B) -> PathBuf {
    let (left, right) = (left.as_ref(), right.as_ref());
    debug!(
        "->run->____->______->canonicalize: Joining {:?} and {:?}",
        left, right
    );

    let mut result = left.to_path_buf();
    for c in right.components() {
        match c {
            Component::RootDir => {}
            Component::Prefix(_) => {
                // Hash the prefix component.
                // We do this because there are many ways to get prefix components
                // on Windows, so its safer to simply hash it.
                result.push(str_component(&c));
            }
            _ => {
                result.push(c);
            }
        }
    }

    debug!("->run->____->______->canonicalize: Result: {:?}", result);
    result
}

pub fn symlink_exists<P: AsRef<Path>>(path: P) -> bool {
    fs::symlink_metadata(path).is_ok()
}

pub fn get_user() -> String {
    #[cfg(unix)]
    {
        env::var("USER").unwrap_or_else(|_| String::from("unknown"))
    }
    #[cfg(target_os = "windows")]
    {
        env::var("USERNAME").unwrap_or_else(|_| String::from("unknown"))
    }
}

// Allows injection of test-specific behavior
pub trait TestingMode {
    fn is_test(&self) -> bool;
}

pub struct ProductionMode;
pub struct TestMode;

impl TestingMode for ProductionMode {
    fn is_test(&self) -> bool {
        false
    }
}
impl TestingMode for TestMode {
    fn is_test(&self) -> bool {
        true
    }
}

/// Prompt for user input, returning True if the first character is 'y' or 'Y'
/// Will create an error if given a 'q' or 'Q', equivalent to if the user
/// had passed a SIGINT.
pub fn prompt_yes(
    prompt: impl AsRef<str>,
    source: &impl TestingMode,
    stream: &mut impl Write,
) -> Result<bool, Error> {
    write!(stream, "{} (y/N) ", prompt.as_ref())?;
    if stream.flush().is_err() {
        // If stdout wasn't flushed properly, fallback to println
        writeln!(stream, "{} (y/N)", prompt.as_ref())?;
    }

    if source.is_test() {
        return Ok(true);
    }

    process_in_stream(io::stdin())
}

pub fn process_in_stream(in_stream: impl Read) -> Result<bool, Error> {
    let buffered = BufReader::new(in_stream);
    let char_result = buffered
        .bytes()
        .next()
        .and_then(|c| c.ok())
        .map(|c| c as char);

    match char_result {
        Some('y') | Some('Y') => Ok(true),
        Some('n') | Some('N') | Some('\n') | None => Ok(false),
        Some('q') | Some('Q') => Err(Error::new(
            io::ErrorKind::Interrupted,
            "User requested to quit",
        )),
        _ => Err(Error::new(io::ErrorKind::InvalidInput, "Invalid input")),
    }
}

/// Add a numbered extension to duplicate filenames to avoid overwriting files.
pub fn rename_grave(grave: impl AsRef<Path>) -> PathBuf {
    let grave = grave.as_ref();
    let name = grave.to_str().expect("Filename must be valid unicode.");
    (1_u64..)
        .map(|i| PathBuf::from(format!("{}~{}", name, i)))
        .find(|p| !symlink_exists(p))
        .expect("Failed to rename duplicate file or directory")
}

pub fn humanize_bytes(bytes: u64) -> String {
    let values = ["bytes", "KB", "MB", "GB", "TB"];
    let pair = values
        .iter()
        .enumerate()
        .take_while(|x| bytes as usize / (1000_usize).pow(x.0 as u32) > 10)
        .last();
    if let Some((i, unit)) = pair {
        format!("{} {}", bytes as usize / (1000_usize).pow(i as u32), unit)
    } else {
        format!("{} {}", bytes, values[0])
    }
}
