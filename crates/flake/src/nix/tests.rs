use drakeflake_exec::ExecError;

use super::*;

#[test]
fn error_summary_prefers_error_lines() {
    let stderr = "warning: Git tree is dirty\ntrace: foo\nerror:\n  … while x\n  error: boom";
    assert_eq!(summarize_error(stderr), "error: boom");
    assert_eq!(summarize_error("just text"), "just text");
    assert_eq!(summarize_error(""), "");
    assert_eq!(summarize_error("a\n  b\nc\nd\ne"), "  b\nc\nd\ne");

    let long = "error: first\nerror: last\n1\ntrace: t\n2\n3\n4\n5\n6\n7";
    assert_eq!(summarize_error(long), "error: last\n1\n2\n3\n4\n5");
}

#[test]
fn json_arguments_are_nix_strings() {
    assert_eq!(
        json_arg(&vec!["x"]).unwrap(),
        "(builtins.fromJSON \"[\\\"x\\\"]\")"
    );
}

#[test]
fn json_output_of_the_wrong_shape_names_what_it_was() {
    let wrong_shape = parse_json::<Vec<u8>>("{}", "host list").unwrap_err();
    assert!(
        wrong_shape
            .to_string()
            .starts_with("unexpected host list output: "),
        "{wrong_shape}"
    );
}

#[test]
fn a_missing_attribute_is_told_from_other_nix_errors() {
    let stderr = "error: attribute 'images' missing\n       at /nix/store/x/y.nix:3:5:\n";
    let error = failure(stderr);

    assert_eq!(error.missing_attribute(), Some("images"));
    assert_eq!(error.to_string(), summarize_error(stderr));

    for other in [
        "error: builder failed",
        "error: attribute 'a' is not a string",
    ] {
        let parsed = failure(other);
        assert_eq!(parsed.missing_attribute(), None, "{other}");
        assert!(matches!(parsed, FlakeError::Nix(_)), "{other}");
    }
}

#[test]
fn cancelled_and_timed_out_runs_stay_recognisable() {
    let cancelled = FlakeError::from(ExecError::Cancelled(None));
    let timed_out = FlakeError::from(ExecError::TimedOut {
        program: "nix".to_owned(),
        after: Duration::from_secs(1),
    });

    assert!(cancelled.is_cancelled() && !cancelled.is_timeout());
    assert!(timed_out.is_timeout() && !timed_out.is_cancelled());
    assert!(!failure("error: x").is_cancelled());
}

#[test]
fn a_flake_installable_reports_a_missing_attribute_by_its_selection_path() {
    let stderr = "error: attribute 'images' in selection path 'nixosConfigurations.\"box\".config.system.build.images' not found\n";

    assert_eq!(failure(stderr).missing_attribute(), Some("images"));
}
