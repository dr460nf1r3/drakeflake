use super::count;
use crate::source::Lang;

const RULE: &str = "no-comment-semicolon";

#[test]
fn semicolons_in_prose_are_reported() {
    assert_eq!(
        count(
            Lang::Rust,
            RULE,
            "// Rounds per line; the total is derived.\nfn f() {}\n"
        ),
        1
    );
    assert_eq!(
        count(
            Lang::Rust,
            RULE,
            "/// Loads the draft; falls back; never throws.\nfn f() {}\n"
        ),
        2
    );
    assert_eq!(
        count(Lang::Js, RULE, "var a = 1; // cached; refreshed hourly\n"),
        1
    );
}

#[test]
fn code_in_comments_passes() {
    let text = "/// Call `load(); render();` in order, with &nbsp; a space.\n///\n/// ```\n/// a(); b();\n/// ```\nfn f() {}\n// let total = sum(&lines);\nfn g() {}\n";
    assert_eq!(count(Lang::Rust, RULE, text), 0);
    let example = "/**\n * Builds the clause.\n * @example\n * const sql = build(); run(sql);\n */\nfunction build() {}\n";
    assert_eq!(count(Lang::Js, RULE, example), 0);
}

#[test]
fn findings_point_at_the_semicolon() {
    let findings = crate::rules::lint_text(Lang::Rust, "fn f() {}\n    // One; two.\n");
    let semicolon = findings.iter().find(|finding| finding.rule == RULE);
    assert_eq!(
        semicolon.map(|finding| (finding.at.row, finding.at.column)),
        Some((1, 10))
    );
}
