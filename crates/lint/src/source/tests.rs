use tree_sitter::Point;

use super::{Kind, Lang, Source, looks_like_code, parse};

fn comments(lang: Lang, text: &str) -> Vec<(Kind, Vec<String>, Point, bool)> {
    let prepared = lang.prepare(text);
    let tree = parse(lang, &prepared).unwrap();
    let source = Source::new(lang, &prepared, &tree);
    source
        .comments
        .iter()
        .map(|comment| {
            (
                comment.kind,
                comment.lines.iter().map(|line| line.text.clone()).collect(),
                comment.lines[0].at,
                comment.alone,
            )
        })
        .collect()
}

#[test]
fn rust_comments_lose_their_markers() {
    let found = comments(
        Lang::Rust,
        "//! Module.\n/// Item.\nfn f() { g(); // trailing\n}\n/*\n * Block\n */\n",
    );

    assert_eq!(
        found[0],
        (
            Kind::InnerDoc,
            vec!["Module.".to_owned()],
            Point::new(0, 4),
            true
        )
    );
    assert_eq!(
        found[1],
        (Kind::Doc, vec!["Item.".to_owned()], Point::new(1, 4), true)
    );
    assert_eq!(
        found[2],
        (
            Kind::Line,
            vec!["trailing".to_owned()],
            Point::new(2, 17),
            false
        )
    );
    assert_eq!(
        found[3],
        (
            Kind::Block,
            vec!["Block".to_owned()],
            Point::new(5, 3),
            true
        )
    );
}

#[test]
fn qml_javascript_headers_are_blanked() {
    let found = comments(Lang::Js, ".pragma library\n/** Doc. */\nfunction f() {}\n");
    assert_eq!(
        found,
        vec![(Kind::Doc, vec!["Doc.".to_owned()], Point::new(1, 4), true)]
    );
}

#[test]
fn code_is_told_from_prose() {
    assert!(looks_like_code(Lang::Rust, "let x = foo(1);"));
    assert!(looks_like_code(Lang::Rust, "x.y = 3"));
    assert!(looks_like_code(Lang::Qml, "text: page.title + \" \""));
    assert!(looks_like_code(Lang::Js, "page.reload()"));
    for prose in ["Read the file", "Done", "Note: the width", "TODO: later"] {
        assert!(!looks_like_code(Lang::Rust, prose), "{prose}");
        assert!(!looks_like_code(Lang::Qml, prose), "{prose}");
    }
}
