use super::count;
use crate::source::Lang;

const RULE: &str = "no-restating-doc";

#[test]
fn docs_that_repeat_the_name_are_reported() {
    assert_eq!(
        count(
            Lang::Rust,
            RULE,
            "/// Gets the invoice lines.\nfn get_invoice_lines() {}\n"
        ),
        1
    );
    assert_eq!(
        count(
            Lang::Rust,
            RULE,
            "/// The flake path.\n#[must_use]\npub struct FlakePath;\n"
        ),
        1
    );
    assert_eq!(
        count(
            Lang::Js,
            RULE,
            "/** Loads the user. */\nfunction loadUser() {}\n"
        ),
        1
    );
}

#[test]
fn generic_tags_are_reported() {
    let text = "/**\n * Parses the lock, newest entries first.\n * @param user The user.\n * @returns The result.\n */\nfunction parseLock(user) {}\n";
    assert_eq!(count(Lang::Js, RULE, text), 2);
}

#[test]
fn docs_with_information_pass() {
    let text = "/// Lines of the invoice, sorted by position. Empty for drafts.\nfn get_invoice_lines() {}\n";
    assert_eq!(count(Lang::Rust, RULE, text), 0);
}
