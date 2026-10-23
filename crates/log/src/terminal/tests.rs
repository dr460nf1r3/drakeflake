use super::*;

#[test]
fn only_a_terminal_gets_colour() {
    assert!(!wants_color(false));
}

#[test]
fn echo_works_without_a_terminal_set() {
    echo(SystemTime::now(), log::Level::Info, "test", "line");
    assert!(!COLOR.load(Ordering::Relaxed) || std::io::stderr().is_terminal());
}
