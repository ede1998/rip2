use rand::distributions::Alphanumeric;
use rand::Rng;
use rstest::rstest;
use std::env::temp_dir;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use rip::{args, util};

struct TestEnv {
    tmpdir: PathBuf,
    graveyard: PathBuf,
    src: PathBuf,
}

impl TestEnv {
    fn new() -> TestEnv {
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
    // Rustc Opposite of new:
    fn teardown(self) {
        let _ = fs::remove_dir_all(self.tmpdir);
    }
}

struct TestData {
    data: String,
    path: PathBuf,
}

impl TestData {
    fn new(test_env: &TestEnv) -> TestData {
        let data = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(100)
            .map(char::from)
            .collect::<String>();

        println!("Graveyard dir: {}", test_env.graveyard.display());
        println!("Src dir: {}", test_env.src.display());

        let path = test_env.src.join("test_file.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all(data.as_bytes()).unwrap();

        TestData { data, path }
    }
}

#[rstest]
#[case::unbury(false)]
#[case::decomposition(true)]
fn test_bury_unbury(#[case] decompose: bool) {
    let test_env = TestEnv::new();
    let test_data = TestData::new(&test_env);
    // And is now in the graveyard
    let expected_graveyard_path =
        util::join_absolute(&test_env.graveyard, test_data.path.canonicalize().unwrap());

    let _ = rip::run(args::Args {
        targets: [test_data.path.clone()].to_vec(),
        graveyard: Some(test_env.graveyard.clone()),
        ..args::Args::default()
    });

    // Verify that the file no longer exists
    assert!(!test_data.path.exists());

    // Verify that the graveyard exists
    assert!(test_env.graveyard.exists());
    assert!(expected_graveyard_path.exists());

    // with the right data
    let restored_data_from_grave = fs::read_to_string(&expected_graveyard_path).unwrap();
    assert_eq!(restored_data_from_grave, test_data.data);

    let _ = rip::run(args::Args {
        graveyard: Some(test_env.graveyard.clone()),
        decompose,
        force: decompose,
        unbury: if decompose { None } else { Some(Vec::new()) },
        ..args::Args::default()
    });

    if decompose {
        // Verify that the graveyard is completely deleted
        assert!(!test_env.graveyard.exists());
        // And that the file was not restored
        assert!(!test_data.path.exists());
    } else {
        // Verify that the file exists in the original location with the correct data
        assert!(test_data.path.exists());
        let restored_data = fs::read_to_string(&test_data.path).unwrap();
        assert_eq!(restored_data, test_data.data);
    }

    test_env.teardown();
}
