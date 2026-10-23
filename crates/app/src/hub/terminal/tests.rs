use super::*;
use crate::hub::argv::arg;

#[test]
fn quoting() {
    assert_eq!(quote("a b"), "'a b'");
    assert_eq!(quote("it's"), r"'it'\''s'");
    assert_eq!(
        command_line(&[arg("nh"), arg("os"), arg("a b")]),
        "'nh' 'os' 'a b'"
    );
    assert!(wait_after("true").starts_with("true; status=$?;"));
}

#[test]
fn terminal_app_quoting() {
    let script = terminal_app_script(r#"echo "hi" \ it's"#);
    assert!(script.starts_with("tell application \"Terminal\""));
    assert!(script.contains(r#"do script "sh -c 'echo \"hi\" \\ it'\\''s'""#));
}

#[test]
fn empty_words_stay_separate_arguments() {
    assert_eq!(quote(""), "''");
    assert_eq!(
        command_line(&[arg("echo"), arg(""), arg("$HOME")]),
        "'echo' '' '$HOME'"
    );
    assert_eq!(command_line(&[]), "");
}

#[test]
fn an_empty_command_is_not_started() {
    assert_eq!(spawn_detached(&[]), Err("empty command".to_owned()));
}

#[test]
fn detached_children_are_reaped() {
    let child = Command::new("true").spawn().unwrap();
    let pid = rustix::process::Pid::from_raw(i32::try_from(child.id()).unwrap()).unwrap();
    reap(child).unwrap().join().unwrap();
    assert!(rustix::process::test_kill_process(pid).is_err());
}
