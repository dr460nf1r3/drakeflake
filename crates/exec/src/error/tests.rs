use super::*;

#[test]
fn messages_display_as_before() {
    let cases = [
        (
            ExecError::Failed("error: attribute missing".to_owned()),
            "error: attribute missing",
        ),
        (ExecError::Cancelled(None), "cancelled"),
        (
            ExecError::Cancelled(Some("nix".to_owned())),
            "nix was cancelled",
        ),
        (
            ExecError::TimedOut {
                program: "nix".to_owned(),
                after: Duration::from_mins(15),
            },
            "nix timed out after 900s",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn only_the_cancelled_variant_is_cancelled() {
    assert!(ExecError::Cancelled(None).is_cancelled());
    assert!(!ExecError::Failed("cancelled".to_owned()).is_cancelled());
}

#[test]
fn only_the_timed_out_variant_is_a_timeout() {
    let timed_out = ExecError::TimedOut {
        program: "nix".to_owned(),
        after: Duration::from_secs(1),
    };

    assert!(timed_out.is_timeout());
    assert!(!ExecError::Cancelled(None).is_timeout());
}
