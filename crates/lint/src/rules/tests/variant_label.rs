use super::count;
use crate::source::Lang;

const RULE: &str = "no-variant-label";

#[test]
fn backticked_variants_are_reported() {
    let text = "fn f() {\nmatch c {\n// Erase in line (`K`, `1K`): redrawn from its start.\n'K' => {},\n}\n}\n";
    assert_eq!(count(Lang::Rust, RULE, text), 1);
}

#[test]
fn reasons_and_outside_functions_pass() {
    let text = "fn f() {\nmatch c {\n// `K` must clear because the row is reused by nom.\n'K' => {},\n}\n}\n";
    assert_eq!(count(Lang::Rust, RULE, text), 0);

    assert_eq!(count(Lang::Rust, RULE, "// Use `K` for erase.\n"), 0);
}
