use std::path::Path;

use tree_sitter::Point;

use crate::finding::{Finding, Located};
use crate::source::Lang;

#[test]
fn only_rust_qml_and_javascript_files_are_linted() {
    for (path, lang) in [
        ("crates/app/src/main.rs", Some(Lang::Rust)),
        ("src/qml/Main.qml", Some(Lang::Qml)),
        ("src/qml/main/helpers.js", Some(Lang::Js)),
        ("README.md", None),
        ("src/qml/qmldir", None),
        ("flake.nix", None),
    ] {
        assert_eq!(Lang::of(Path::new(path)), lang, "{path}");
    }
}

#[test]
fn findings_print_one_based_positions_with_why_and_fix() {
    let finding = Finding::new(
        Point { row: 0, column: 4 },
        "semicolon",
        "A semicolon in a comment.".to_owned(),
        "Readers stumble.",
    )
    .fix("End the sentence.");

    let located = Located {
        path: Path::new("crates/x/src/lib.rs"),
        finding: &finding,
    };

    assert_eq!(
        located.to_string(),
        "crates/x/src/lib.rs:1:5: semicolon: A semicolon in a comment.\n    Why: Readers stumble.\n    How to fix: End the sentence."
    );
}
