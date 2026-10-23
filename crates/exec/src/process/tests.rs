use std::time::{Duration, Instant};

use super::*;
use crate::context::Context;

const NO_ARGS: &[&str] = &[];

fn system() -> Context {
    Context::system()
}

fn logged(program: &str, args: &[&str]) -> String {
    command_line(
        OsStr::new(program),
        &args.iter().map(OsString::from).collect::<Vec<_>>(),
    )
}

#[test]
fn commands_time_out_and_report_missing_binaries() {
    let context = system();
    let err = context
        .command("sleep", ["5"], Duration::from_millis(200))
        .unwrap_err();

    assert!(err.is_timeout(), "{err}");

    let cancelled_context = context.child();
    cancelled_context.cancel();

    let cancelled = cancelled_context
        .command("sleep", ["5"], Duration::from_secs(5))
        .unwrap_err();

    assert!(cancelled.is_cancelled(), "{cancelled}");

    context
        .command("/nonexistent-binary", NO_ARGS, SHORT_TIMEOUT)
        .unwrap_err();

    let piped = context
        .command_with_input("cat", NO_ARGS, "from stdin", SHORT_TIMEOUT)
        .unwrap();

    assert_eq!(piped.stdout, "from stdin");

    let ok = context.command("echo", ["hi"], SHORT_TIMEOUT).unwrap();
    assert!(ok.success);
    assert_eq!(ok.stdout.trim(), "hi");
}

#[test]
fn a_cancelled_load_kills_its_commands() {
    let context = system();
    let token = context.cancel_token().clone();
    let started = Instant::now();

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        token.cancel();
    });

    let result = context.command("sleep", ["30"], SHORT_TIMEOUT);

    result.unwrap_err();
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn the_stderr_tail_keeps_the_last_lines() {
    assert_eq!(stderr_tail("just text"), "just text");
    assert_eq!(stderr_tail(""), "");
    assert_eq!(stderr_tail("a\n\n  b\nc\nd\ne"), "  b\nc\nd\ne");
}

#[test]
fn logged_command_lines_are_flat_and_short() {
    let expression = "let\n  x = 1;\nin\n  x";
    assert_eq!(
        logged("nix", &["eval", "--expr", expression]),
        "nix eval --expr let x = 1; in x"
    );

    let long = "a".repeat(LOGGED_ARG_CHARS + 1);
    let fits = "b".repeat(LOGGED_ARG_CHARS);
    let line = logged("nix", &[&long, &fits]);
    assert_eq!(
        line,
        format!("nix {}… {fits}", "a".repeat(LOGGED_ARG_CHARS))
    );
    assert_eq!(logged("true", NO_ARGS), "true");
}

#[test]
fn cancelled_and_timed_out_runs_are_typed() {
    let context = system();
    context.cancel();

    let cancelled = context.command("sleep", ["5"], SHORT_TIMEOUT).unwrap_err();

    assert!(cancelled.is_cancelled());
    assert_eq!(cancelled.to_string(), "sleep was cancelled");

    let timed_out = system()
        .command("sleep", ["5"], Duration::from_millis(100))
        .unwrap_err();

    assert!(!timed_out.is_cancelled());
    assert_eq!(timed_out.to_string(), "sleep timed out after 0s");
}

#[test]
fn a_working_directory_that_is_not_a_directory_is_an_error() {
    let invocation = Invocation::new("true", NO_ARGS, SHORT_TIMEOUT)
        .cwd(Some(Path::new("/nonexistent-drakeflake-dir")));

    let err = system().run(invocation).unwrap_err();

    assert!(
        err.to_string().contains("/nonexistent-drakeflake-dir"),
        "{err}"
    );
}

#[test]
fn a_timeout_stops_the_grandchildren_too() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("pid");
    let script = format!("sleep 30 & echo $! > {}; wait", pid_file.display());

    system()
        .command("sh", ["-c", &script], Duration::from_secs(1))
        .unwrap_err();

    let pid = std::fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse::<i32>()
        .unwrap();

    let pid = rustix::process::Pid::from_raw(pid).unwrap();

    let gone = (0_u8..60).any(|_| {
        std::thread::sleep(Duration::from_millis(50));
        rustix::process::test_kill_process(pid).is_err()
    });

    assert!(gone, "the grandchild {pid:?} survived the timeout");
}

#[test]
fn a_command_that_ignores_sigterm_is_killed() {
    let started = Instant::now();
    let script = "trap '' TERM; while :; do sleep 1; done";
    let err = system()
        .command("sh", ["-c", script], Duration::from_millis(200))
        .unwrap_err();

    assert!(err.is_timeout(), "{err}");
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn logged_command_lines_hide_url_credentials() {
    let url = "https://u:secret@h.example/a?token=abc";
    let line = logged("nix", &["flake", "lock", "--override-input", "x", url]);

    assert_eq!(
        line,
        "nix flake lock --override-input x https://***@h.example/a?token=***"
    );
}

#[cfg(unix)]
#[test]
fn an_argument_that_is_not_utf8_reaches_the_child_unchanged() {
    use std::os::unix::ffi::OsStringExt as _;

    let bytes = vec![b'a', 0xff, 0xfe, b'z'];
    let args = [
        OsString::from("-c"),
        OsString::from("printf '%s' \"$1\" | od -An -tx1"),
        OsString::from("sh"),
        OsString::from_vec(bytes),
    ];

    let captured = system().command("sh", &args, SHORT_TIMEOUT).unwrap();

    assert_eq!(captured.stdout.trim(), "61 ff fe 7a");
}
