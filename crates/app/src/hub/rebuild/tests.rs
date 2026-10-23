use super::super::test_support::console_lines;
use super::*;

#[test]
fn nothing_runs_without_a_command_or_a_system() {
    let hub = Arc::new(Hub::default());

    assert_eq!(
        Exclusive::detached(&hub).stream_command(&[], Path::new("/")),
        Outcome::Failed
    );
    assert_eq!(console_lines(&hub), vec!["Nothing to run."]);

    assert_eq!(
        hub.rebuild_only(Mode::Switch)
            .map_err(|error| error.to_string()),
        Err("No system is selected yet".to_owned())
    );
    assert!(!hub.operations().is_running());
}

#[test]
fn a_review_needs_a_local_flake() {
    let hub = Arc::new(Hub::default());
    let remote = Target {
        flake: "github:me/config".to_owned(),
        host: "laptop".into(),
        ..Target::default()
    };

    assert_eq!(
        Exclusive::detached(&hub).rebuild_with_review(&remote, Mode::Switch),
        Outcome::Failed
    );
    assert_eq!(
        console_lines(&hub),
        vec!["Reviewing a rebuild needs a local flake directory."]
    );
}

#[test]
fn an_activation_is_announced_before_it_runs() {
    let hub = Arc::new(Hub::default());
    let command = ["sh", "-c", "echo hidden"].map(arg);

    assert_eq!(
        Exclusive::detached(&hub).stream_quiet(&command, Path::new("/")),
        Outcome::Succeeded
    );
    assert_eq!(
        console_lines(&hub),
        vec![
            "Activating the system (asks for administrator rights)…",
            "Finished successfully."
        ]
    );
}
