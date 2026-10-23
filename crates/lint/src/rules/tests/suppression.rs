use super::{count, rules};
use crate::source::Lang;

#[test]
fn allow_directives_need_a_reason_and_cover_the_next_line() {
    let allowed =
        "// drakeflake-lint: allow(no-comment-semicolon) quoting a log line\n// a; b\nfn f() {}\n";

    assert_eq!(rules(Lang::Rust, allowed), Vec::<&str>::new());

    let bare = "// drakeflake-lint: allow(no-comment-semicolon)\n// a; b\nfn f() {}\n";
    assert_eq!(rules(Lang::Rust, bare), vec!["allow-without-reason"]);
    assert_eq!(
        count(Lang::Rust, "no-comment-semicolon", "// a; b\nfn f() {}\n"),
        1
    );
}
