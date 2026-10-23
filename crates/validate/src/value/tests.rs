use serde_json::json;

use super::*;
use crate::types::parse_type;

#[test]
fn editor_text_follows_the_editor_of_the_type() {
    let cases = [
        ("floating point number", json!(0.5_f64), Some("0.5")),
        ("signed integer", json!(1.5_f64), None),
        ("absolute path", json!("/etc/hosts"), Some("/etc/hosts")),
        (r#"one of "a", 3"#, json!(3_i32), Some("3")),
        (r#"one of "a", 3"#, json!("a"), Some("a")),
        ("list of absolute path", json!(["/a", "/b"]), Some("/a\n/b")),
        ("list of boolean", json!([true, "no"]), None),
        ("package", json!({"_drakeflakeDrv": "hello-2.12"}), None),
        ("boolean", json!("true"), None),
    ];

    for (type_name, value, expected) in cases {
        assert_eq!(
            value_to_input(&parse_type(type_name), &value).as_deref(),
            expected,
            "{type_name}: {value}"
        );
    }
}

#[test]
fn placeholders_anywhere_make_a_value_unrepresentable() {
    assert_eq!(
        json_to_nix(&json!([1_i32, {"_drakeflakeOpaque": "lambda"}])),
        None
    );
    assert_eq!(
        json_to_nix(&json!({"a": {"b": {"_drakeflakeError": "x"}}})),
        None
    );
    assert_eq!(
        json_to_nix(&json!({"n": 1.25_f64, "list": [null]})),
        Some(NixValue::Attrs(vec![
            ("list".to_owned(), NixValue::List(vec![NixValue::Null])),
            ("n".to_owned(), NixValue::Float(1.25)),
        ]))
    );
}

#[test]
fn display_shows_placeholders_and_nests_sets() {
    assert_eq!(
        display_value(&json!({"_drakeflakeError": "boom"})),
        "«evaluation error»"
    );
    assert_eq!(display_value(&json!({})), "{ }");
    assert_eq!(
        display_value(&json!({"users": {"root": {"_drakeflakeOpaque": "function"}}, "x.y": []})),
        "{\n  users = {\n    root = «function»;\n  };\n  \"x.y\" = [ ];\n}"
    );
}
