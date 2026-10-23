use super::count;
use crate::source::Lang;

const RULE: &str = "no-restating-comment";

#[test]
fn restating_comments_are_reported() {
    for code in [
        "// Set loading to true\nthis.loading = true;",
        "// Get the user from the session\nconst user = session.user;",
        "// Loop over all lines\nfor (const line of lines) {}",
        "// Check if the user is an admin\nif (user.isAdmin) {}",
    ] {
        assert_eq!(
            count(Lang::Js, RULE, &format!("function f() {{\n{code}\n}}\n")),
            1,
            "{code}"
        );
    }

    let rust = "fn f() {\n    // Load the lock file\n    let lock = load_lock_file();\n}\n";
    assert_eq!(count(Lang::Rust, RULE, rust), 1);
}

#[test]
fn trailing_restatements_are_reported() {
    assert_eq!(
        count(
            Lang::Js,
            RULE,
            "let count = 1;\ncount = 0; // reset count\n"
        ),
        1
    );
}

#[test]
fn reasons_and_information_pass() {
    let text = "function f() {\n// The backend recalculates the price after a PATCH\nconst price = line.price;\n// Set to Y, otherwise the server ignores the line\nflag = 'Y';\n}\n";
    assert_eq!(count(Lang::Js, RULE, text), 0);
}
