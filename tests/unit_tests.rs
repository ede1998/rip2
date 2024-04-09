use rip2::args::{validate_args, Args};
use rip2::copy_file;
use rip2::util::TestMode;
use rstest::rstest;
use std::fs;
use std::os::unix;
use std::path::PathBuf;
use std::process;
use tempfile::tempdir;

#[rstest]
fn test_validation() {
    let bad_completions = Args {
        completions: Some("bash".to_string()),
        decompose: true,
        ..Args::default()
    };
    validate_args(&bad_completions).expect_err("--completions can only be used by itself");

    let bad_decompose = Args {
        decompose: true,
        seance: true,
        ..Args::default()
    };
    validate_args(&bad_decompose).expect_err("-d,--decompose can only be used with --graveyard");
}

#[rstest]
#[case::regular("regular")]
#[case::fifo("fifo")]
#[case::symlink("symlink")]
fn test_filetypes(#[case] file_type: &str) {
    let tmpdir = tempdir().unwrap();
    let path = PathBuf::from(tmpdir.path());
    let source_path = path.join("test_file");
    let dest_path = path.join("test_file_copy");

    match file_type {
        "regular" => {
            fs::File::create(&source_path).unwrap();
        }
        "fifo" => {
            process::Command::new("mkfifo")
                .arg(&source_path)
                .output()
                .unwrap();
        }
        "symlink" => {
            let target_path = path.join("symlink_target");
            fs::File::create(&target_path).unwrap();
            unix::fs::symlink(&target_path, &source_path).unwrap();
        }
        _ => unreachable!(),
    }

    let mut log = Vec::new();
    let mode = TestMode;

    copy_file(&source_path, &dest_path, &mode, &mut log).unwrap();
}
