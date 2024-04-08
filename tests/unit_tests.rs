use rip2::args::{validate_args, Args};
use rstest::rstest;

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
