use drakeflake_render::NixValue;
use serde_json::json;

use super::*;

const PORT: &str = "16 bit unsigned integer; between 0 and 65535 (both inclusive)";

#[test]
fn kinds() {
    assert_eq!(parse_type("boolean").kind(), "bool");
    assert_eq!(parse_type("signed integer").kind(), "int");
    assert_eq!(
        parse_type(PORT).bounds(),
        (Some(0), Some(i64::from(u16::MAX)))
    );
    assert_eq!(
        parse_type("positive integer, meaning >0").bounds(),
        (Some(1), None)
    );
    assert_eq!(
        parse_type("unsigned integer, meaning >=0").bounds(),
        (Some(0), None)
    );
    assert_eq!(parse_type("list of string").kind(), "list");
    assert_eq!(parse_type("attribute set of (submodule)").kind(), "raw");
    assert_eq!(parse_type("package").kind(), "package");
    assert_eq!(parse_type("absolute path").kind(), "path");
    assert_eq!(
        parse_type("strings concatenated with \"\\n\"").kind(),
        "text"
    );
    assert_eq!(
        parse_type("string matching the pattern [a-z]+").kind(),
        "string"
    );
    assert_eq!(parse_type("string or signed integer").kind(), "raw");
    assert_eq!(parse_type("something exotic").kind(), "raw");
    assert_eq!(parse_type("list of (submodule)").kind(), "raw");
    assert_eq!(parse_type("floating point number").kind(), "float");
}

#[test]
fn nullable_types() {
    let ty = parse_type("null or boolean");
    assert!(ty.nullable);
    assert_eq!(ty.kind(), "bool");
    assert_eq!(parse_input(&ty, "null"), Ok(NixValue::Null));

    let list = parse_type("null or (list of string)");
    assert!(list.nullable);
    assert_eq!(list.kind(), "list");

    parse_input(&parse_type("boolean"), "null").unwrap_err();
}

#[test]
fn enums() {
    let ty = parse_type(r#"one of "auto", "manual", 3"#);
    assert_eq!(ty.choices(), vec!["auto", "manual", "3"]);
    assert_eq!(
        parse_input(&ty, "manual"),
        Ok(NixValue::Str("manual".into()))
    );
    assert_eq!(parse_input(&ty, "3"), Ok(NixValue::Int(3)));
    parse_input(&ty, "other").unwrap_err();

    let tricky = parse_type(r#"one of "a or b", "c\"d""#);
    assert_eq!(tricky.choices(), vec!["a or b", "c\"d"]);
    assert_eq!(parse_type("one of \"unterminated").kind(), "raw");
}

#[test]
fn bool_words() {
    let ty = parse_type("boolean");
    for word in ["true", "yes", "on", "1", " TRUE "] {
        assert_eq!(parse_input(&ty, word), Ok(NixValue::Bool(true)));
    }

    for word in ["false", "no", "off", "0"] {
        assert_eq!(parse_input(&ty, word), Ok(NixValue::Bool(false)));
    }

    parse_input(&ty, "maybe").unwrap_err();
}

#[test]
fn int_and_port_range() {
    let ty = parse_type("signed integer");
    assert_eq!(parse_input(&ty, "8080"), Ok(NixValue::Int(8080)));
    assert_eq!(parse_input(&ty, "-5"), Ok(NixValue::Int(-5)));
    parse_input(&ty, "abc").unwrap_err();
    parse_input(&ty, "99999999999999999999999").unwrap_err();

    let port = parse_type(PORT);
    assert_eq!(parse_input(&port, "443"), Ok(NixValue::Int(443)));
    parse_input(&port, "99999").unwrap_err();
    parse_input(&port, "-1").unwrap_err();
}

#[test]
fn lists() {
    assert_eq!(
        parse_input(&parse_type("list of string"), "a b\n\nc, d\n"),
        Ok(NixValue::List(vec![
            NixValue::Str("a b".into()),
            NixValue::Str("c, d".into()),
        ]))
    );
    assert_eq!(
        parse_input(&parse_type(&format!("list of {PORT}")), "22, 80 443"),
        Ok(NixValue::List(vec![
            NixValue::Int(22),
            NixValue::Int(80),
            NixValue::Int(443)
        ]))
    );
    let err = parse_input(&parse_type(&format!("list of {PORT}")), "22 nope");
    assert_eq!(
        err,
        Err(InputError::Item {
            number: 2,
            cause: Box::new(InputError::NotWholeNumber("nope".to_owned()))
        })
    );
    assert_eq!(
        parse_input(&parse_type("list of package"), "git, kdePackages.kate"),
        Ok(NixValue::List(vec![
            NixValue::Raw("pkgs.git".into()),
            NixValue::Raw("pkgs.kdePackages.kate".into()),
        ]))
    );
}

#[test]
fn packages_paths_raw() {
    let pkg = parse_type("package");
    assert_eq!(
        parse_input(&pkg, "pkgs.firefox"),
        Ok(NixValue::Raw("pkgs.firefox".into()))
    );
    parse_input(&pkg, "rm -rf /").unwrap_err();
    parse_input(&pkg, "").unwrap_err();

    let path = parse_type("absolute path");
    parse_input(&path, "relative").unwrap_err();
    assert_eq!(
        parse_input(&path, " /etc/x "),
        Ok(NixValue::Str("/etc/x".into()))
    );

    let raw = parse_type("attribute set of string");
    parse_input(&raw, "{ a = \"}\"; }").unwrap();
    parse_input(&raw, "{ a = 1;").unwrap_err();
    parse_input(&raw, "[ ) ").unwrap_err();
    parse_input(&raw, "\"open").unwrap_err();
    parse_input(&raw, "   ").unwrap_err();
}

#[test]
fn values_to_editor_text() {
    assert_eq!(
        value_to_input(&parse_type("boolean"), &json!(true)),
        Some("true".into())
    );
    assert_eq!(
        value_to_input(&parse_type("list of string"), &json!(["a", "b"])),
        Some("a\nb".into())
    );
    assert_eq!(
        value_to_input(
            &parse_type(&format!("list of {PORT}")),
            &json!([22_u16, 80_u16])
        ),
        Some("22 80".into())
    );
    assert_eq!(value_to_input(&parse_type("boolean"), &json!(null)), None);
    assert_eq!(
        value_to_input(&parse_type("null or string"), &json!(null)),
        Some("null".into())
    );
    assert_eq!(
        value_to_input(&parse_type("attribute set of string"), &json!({"a": "b"})),
        Some("{\n  a = \"b\";\n}".into())
    );
    assert_eq!(
        value_to_input(
            &parse_type("attribute set of package"),
            &json!({"a": {"_drakeflakeDrv": "x"}})
        ),
        None
    );
}

#[test]
fn display() {
    assert_eq!(display_value(&json!(true)), "true");
    assert_eq!(display_value(&json!([])), "[ ]");
    assert_eq!(display_value(&json!([1_u8, "a"])), "[\n  1\n  \"a\"\n]");
    assert_eq!(
        display_value(&json!({"_drakeflakeDrv": "firefox-1.0"})),
        "«derivation firefox-1.0»"
    );
    assert_eq!(
        display_value(&json!({"_drakeflakeOpaque": "lambda"})),
        "«lambda»"
    );
}

#[test]
fn shapes() {
    assert_eq!(value_shape("boolean"), ValueShape::Scalar);
    assert_eq!(value_shape(PORT), ValueShape::Scalar);
    assert_eq!(value_shape("list of (submodule)"), ValueShape::List);
    assert_eq!(value_shape("null or (list of string)"), ValueShape::List);
    assert_eq!(value_shape("non-empty (list of string)"), ValueShape::List);
    assert_eq!(value_shape("lines"), ValueShape::List);
    assert_eq!(
        value_shape("null or strings concatenated with \"\\n\""),
        ValueShape::List
    );
    assert_eq!(
        value_shape("attribute set of (submodule)"),
        ValueShape::Attrs
    );
    assert_eq!(
        value_shape("lazy attribute set of string"),
        ValueShape::Attrs
    );
    assert_eq!(value_shape("submodule"), ValueShape::Attrs);
    assert_eq!(value_shape("string or signed integer"), ValueShape::Scalar);
    assert_eq!(value_shape(""), ValueShape::Scalar);
}

#[test]
fn never_panics_on_garbage() {
    for ty in [
        "",
        "(",
        ")",
        "null or ",
        "one of ",
        "list of ",
        "list of list of list of list of list of list of string",
        "between",
        "integer between and",
        "non-empty (",
        "((((",
        "null or null or null or boolean",
    ] {
        let parsed = parse_type(ty);
        drop(parse_input(&parsed, ""));
        drop(parse_input(&parsed, "\u{0}\"[{("));
    }
}

#[test]
fn input_errors_keep_their_wording() {
    let item = InputError::Item {
        number: 3,
        cause: Box::new(InputError::BelowMinimum {
            value: -1,
            minimum: 0,
        }),
    };

    assert_eq!(item.to_string(), "Item 3: -1 is below the minimum of 0");
    assert_eq!(
        InputError::NotOneOf(vec!["a".to_owned(), "b".to_owned()]).to_string(),
        "Must be one of: a, b"
    );
    assert_eq!(InputError::Unclosed('{').to_string(), "Unclosed \"{\"");
}
