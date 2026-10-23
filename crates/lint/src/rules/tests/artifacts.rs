use super::count;
use crate::source::Lang;

const RULE: &str = "no-llm-artifacts";

#[test]
fn placeholder_comments_are_reported() {
    for comment in [
        "// ... existing code",
        "// rest of the function stays the same",
        "// TODO: implement",
        "// your logic goes here",
        "// omitted for brevity",
        "// same as above",
    ] {
        assert_eq!(
            count(Lang::Rust, RULE, &format!("{comment}\nfn f() {{}}\n")),
            1,
            "{comment}"
        );
        assert_eq!(
            count(Lang::Qml, RULE, &format!("{comment}\nItem {{}}\n")),
            1,
            "{comment}"
        );
    }
}

#[test]
fn ordinary_comments_pass() {
    let text = "// The rest of an escape sequence after ESC.\nfn f() {}\n";
    assert_eq!(count(Lang::Rust, RULE, text), 0);
}

#[test]
fn javascript_stubs_are_reported() {
    let text = "function load() {\n    throw new Error(\"Not implemented yet\");\n}\n";
    assert_eq!(count(Lang::Js, RULE, text), 1);
    let real = "function load() {\n    throw new Error(\"No flake\");\n}\n";
    assert_eq!(count(Lang::Js, RULE, real), 0);
}
