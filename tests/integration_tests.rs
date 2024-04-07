use rand::distributions::Alphanumeric;
use rand::Rng;
use rstest::rstest;
use std::env::temp_dir;
use std::fs::{self, metadata, read_to_string, remove_dir_all, File};
use std::io::Write;
use std::path::PathBuf;

use rip::{args, util};

struct TestEnv {
    tmpdir: PathBuf,
    graveyard: PathBuf,
    src: PathBuf,
}

fn setup_test_env() -> TestEnv {
    let rand_string = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect::<String>();
    let tmpdir = temp_dir().join(format!("rip_test_{}", rand_string));
    let graveyard = tmpdir.join("graveyard");
    let src = tmpdir.join("data");

    // The graveyard should be created, so we don't test this:
    //// fs::create_dir_all(&graveyard).unwrap();
    fs::create_dir_all(&src).unwrap();

    TestEnv {
        tmpdir,
        graveyard,
        src,
    }
}
fn teardown_test_env(test_env: TestEnv) {
    let _ = remove_dir_all(test_env.tmpdir);
}

fn make_test_data() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(100)
        .map(char::from)
        .collect::<String>()
}

#[rstest]
#[case::unbury(false)]
#[case::decomposition(true)]
fn test_bury_unbury(#[case] decompose: bool) {
    let data = make_test_data();
    let test_env = setup_test_env();

    println!("Graveyard dir: {}", test_env.graveyard.display());
    println!("Src dir: {}", test_env.src.display());

    let datafile_path = test_env.src.join("test_file.txt");
    let mut file = File::create(&datafile_path).unwrap();
    let datafile_path_canonical = datafile_path.canonicalize().unwrap();

    file.write_all(data.as_bytes()).unwrap();

    let _ = rip::run(args::Args {
        targets: [datafile_path.clone()].to_vec(),
        graveyard: Some(test_env.graveyard.clone()),
        decompose: false,
        force: false,
        seance: false,
        unbury: None,
        inspect: false,
        completions: None,
    });

    // Verify that the file no longer exists
    assert!(!metadata(&datafile_path).is_ok());
    // Verify that the graveyard exists
    assert!(metadata(&test_env.graveyard).is_ok());

    // And is now in the graveyard
    let grave_datafile_path = util::join_absolute(&test_env.graveyard, &datafile_path_canonical);
    // test_env.graveyard.join(&datafile_path);
    assert!(metadata(&grave_datafile_path).is_ok());
    // with the right data
    let restored_data_from_grave = read_to_string(&grave_datafile_path).unwrap();
    assert_eq!(restored_data_from_grave, data);

    if decompose {
        let _ = rip::run(args::Args {
            targets: Vec::new(),
            graveyard: Some(test_env.graveyard.clone()),
            decompose: true,
            force: true,
            seance: false,
            unbury: None,
            inspect: false,
            completions: None,
        });

        // Verify that the graveyard is completely deleted
        assert!(!metadata(&test_env.graveyard).is_ok());
        // And that the file was not restored
        assert!(!metadata(&datafile_path).is_ok());
    } else {
        // Unbury the file using the CLI
        let _ = rip::run(args::Args {
            targets: Vec::new(),
            graveyard: Some(test_env.graveyard.clone()),
            decompose: false,
            force: false,
            seance: false,
            unbury: Some(Vec::new()),
            inspect: false,
            completions: None,
        });

        // Verify that the file exists in the original location with the correct data
        assert!(metadata(&datafile_path).is_ok());
        let restored_data = read_to_string(&datafile_path).unwrap();
        assert_eq!(restored_data, data);
    }

    teardown_test_env(test_env)
}
