use super::*;

fn ended(code: Option<i32>, stderr: &str) -> Captured {
    Captured {
        success: false,
        code,
        stdout: String::new(),
        stderr: stderr.to_owned(),
    }
}

#[test]
fn the_exit_code_says_who_ended_the_run() {
    let refused = helper_failure(&ended(
        Some(3_i32),
        "drakeflake-helper: refusing the plan: x\n",
    ));

    assert!(matches!(&refused, FileError::HelperRefused(text) if text.ends_with("plan: x")));

    for code in [126_i32, 127_i32] {
        let dismissed = helper_failure(&ended(Some(code), "Error: Not authorized"));
        assert!(
            matches!(dismissed, FileError::AuthenticationDismissed(_)),
            "{code}"
        );
    }

    for code in [Some(1_i32), Some(4_i32), Some(5_i32), None] {
        assert!(
            matches!(
                helper_failure(&ended(code, "boom")),
                FileError::HelperFailed(_)
            ),
            "{code:?}"
        );
    }
}

#[test]
fn the_message_stays_the_same_for_every_class() {
    let message = "could not write the settings with administrator rights: boom";

    for code in [3_i32, 126_i32, 4_i32] {
        assert_eq!(
            helper_failure(&ended(Some(code), " boom\n")).to_string(),
            message
        );
    }
}

#[test]
fn a_missing_helper_is_reported_before_anything_runs() {
    let error = write_elevated(&Context::system(), &WritePlan::default(), None).unwrap_err();

    assert!(matches!(error, FileError::NoHelper), "{error}");
}
