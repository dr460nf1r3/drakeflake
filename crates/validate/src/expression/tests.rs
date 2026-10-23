use drakeflake_render::wrap_expression;

use super::*;

#[test]
fn accepts_ordinary_expressions() {
    for text in [
        "config.networking.hostName + \"-vm\"",
        "pkgs.linuxPackages_latest",
        "lib.mkIf config.services.xserver.enable true",
        "{ a = 1; b = [ 1 2 ]; }",
        "\"${pkgs.hello}/bin/hello\"",
        "\"a ) string ( with brackets\"",
        "''\n  multi ${toString 1}\n  line ''$ ''' ''\\n )\n''",
        "''${ \"interpolated\" }''",
        "x: x + 1 # a comment with ) and (",
        "/* ) */ 1",
        "let f' = 1; in f'",
        "./foo/bar",
        "./${\"x\"}/y",
        "<nixpkgs>",
        "https://example.org/a'b?x=1",
        "\"$${not interpolated\"",
        "{ ${\"dyn\"} = 1; }",
        "(1) + (2)",
    ] {
        assert!(
            parse_expression(text).is_ok(),
            "{text}: {:?}",
            parse_expression(text)
        );
    }
}

#[test]
fn rejects_unbalanced_and_unterminated() {
    for text in [
        "",
        "  ",
        "(1",
        "[ 1 }",
        "\"open",
        "''open",
        "/* open",
        "\"${ 1 \"",
        "1 +",
    ] {
        assert!(parse_expression(text).is_err(), "{text} accepted");
    }
}

#[test]
fn cannot_break_out_of_the_parenthesis() {
    for text in [
        "1); evil = (2",
        "1); system.evil = true; x = (2",
        "1) (2",
        "1)",
        "x # \"\n); evil = true; z = (# \"",
        "x'' ); evil = true; y = ( ''",
        "a:'' ); evil = true; y = ( ''",
        "x:/* ); evil = true; y = ( */",
        "a/b'' \" ''); evil = true; z = (\"",
        "''${ ''); evil = true; z = (''",
    ] {
        assert!(parse_expression(text).is_err(), "{text:?} accepted");
    }
}

#[test]
fn errors_point_into_the_expression() {
    let error = parse_expression("{ a = 1 }").unwrap_err().to_string();
    assert!(error.contains("line 1, column"), "{error}");
    assert!(error.contains("expected"), "{error}");

    let multi_line = parse_expression("[\n  1\n  ;\n]").unwrap_err().to_string();
    assert!(multi_line.contains("line 3"), "{multi_line}");
}

#[test]
fn errors_are_described_in_the_users_terms() {
    for (text, expected) in [
        ("[ 1\n  2", "Unexpected \")\" at the end (line 2), expected"),
        (
            "{ a = 1 }",
            "Unexpected \"}\" at line 1, column 9, expected \";\"",
        ),
        (
            "{ a, b ? }: a",
            "Unexpected \"}\" at line 1, column 10, expected \"(\", rec, \"{\", \"[\", string start, ident",
        ),
        (
            "if 1 then 2",
            "Unexpected \")\" at the end (line 1), expected else",
        ),
    ] {
        assert_eq!(
            parse_expression(text)
                .unwrap_err()
                .to_string()
                .get(..expected.len()),
            Some(expected),
            "{text:?}"
        );
    }
}

#[test]
fn wrapping() {
    assert_eq!(wrap_expression("pkgs.hello"), "(pkgs.hello)");
    assert_eq!(wrap_expression("1 # one"), "(\n1 # one\n)");
}

#[test]
fn errors_keep_their_place_for_callers() {
    use crate::source::Site;

    assert_eq!(
        parse_expression("{ a = 1 }"),
        Err(ExpressionError::Syntax {
            message: "Unexpected \"}\" at line 1, column 9, expected \";\"".to_owned(),
            position: Some(Position::At(Site { line: 1, column: 9 })),
        })
    );
    assert!(
        parse_expression("if 1 then 2")
            .unwrap_err()
            .to_string()
            .contains("at the end (line 1)")
    );
    assert_eq!(parse_expression("  "), Err(ExpressionError::Empty));
}
