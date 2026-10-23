use super::count;
use crate::source::Lang;

const RULE: &str = "comment-style";

#[test]
fn block_comments_are_reported() {
    assert_eq!(count(Lang::Rust, RULE, "/* Why. */\nfn f() {}\n"), 1);
    assert_eq!(count(Lang::Rust, RULE, "/** Docs. */\nfn f() {}\n"), 1);
    assert_eq!(count(Lang::Qml, RULE, "Item {\n    /* Why. */\n}\n"), 1);
}

#[test]
fn line_comments_and_jsdoc_pass() {
    assert_eq!(
        count(Lang::Rust, RULE, "// Why.\n/// Docs.\nfn f() {}\n"),
        0
    );
    assert_eq!(count(Lang::Js, RULE, "/** Docs. */\nfunction f() {}\n"), 0);
}
