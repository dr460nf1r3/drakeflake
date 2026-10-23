use super::*;

#[test]
fn scalars() {
    assert_eq!(render_value(&NixValue::Bool(true)), "true");
    assert_eq!(render_value(&NixValue::Bool(false)), "false");
    assert_eq!(render_value(&NixValue::Null), "null");
    assert_eq!(render_value(&NixValue::Int(-42)), "-42");
    assert_eq!(render_value(&NixValue::Float(1.5)), "1.5");
    assert_eq!(render_value(&NixValue::Float(2.0)), "2.0");
    assert_eq!(render_value(&NixValue::Float(f64::NAN)), "0.0");
}

#[test]
fn string_escapes() {
    assert_eq!(render_string("plain"), "\"plain\"");
    assert_eq!(
        render_string("quote \" back\\slash ${interp}"),
        "\"quote \\\" back\\\\slash \\${interp}\""
    );
    assert_eq!(render_string("a\nb\tc"), "\"a\\nb\\tc\"");
}

#[test]
fn nested_list() {
    let value = NixValue::List(vec![
        NixValue::Str("a".to_owned()),
        NixValue::Int(1),
        NixValue::Bool(true),
    ]);

    assert_eq!(render_value(&value), "[ \"a\" 1 true ]");
    assert_eq!(render_value(&NixValue::List(Vec::new())), "[ ]");
}

#[test]
fn attrs_sorted_by_key() {
    let value = NixValue::Attrs(vec![
        ("b".to_owned(), NixValue::Int(2)),
        ("a".to_owned(), NixValue::Bool(false)),
        ("with space".to_owned(), NixValue::Null),
    ]);

    assert_eq!(
        render_value(&value),
        "{\n  a = false;\n  b = 2;\n  \"with space\" = null;\n}"
    );
}

#[test]
fn raw_expressions() {
    assert_eq!(
        render_value(&NixValue::Raw("pkgs.firefox".into())),
        "pkgs.firefox"
    );
    assert_eq!(
        render_value(&NixValue::Raw(" { a = 1; } ".into())),
        "({ a = 1; })"
    );
}

#[test]
fn attribute_names_are_quoted_when_needed() {
    assert_eq!(render_attr_name("enable"), "enable");
    assert_eq!(render_attr_name("my user"), "\"my user\"");
    assert_eq!(render_attr_name("in"), "\"in\"");
}

#[test]
fn negative_numbers_in_lists_are_parenthesised() {
    let value = NixValue::List(vec![
        NixValue::Int(-1),
        NixValue::Float(-1.5),
        NixValue::Int(2),
    ]);

    assert_eq!(render_value(&value), "[ (-1) (-1.5) 2 ]");
}

#[test]
fn smallest_integer_stays_parsable() {
    assert_eq!(
        render_value(&NixValue::Int(i64::MIN)),
        "(-9223372036854775807 - 1)"
    );
}

#[test]
fn raw_expression_ending_in_a_comment_closes_on_its_own_line() {
    assert_eq!(
        render_value(&NixValue::Raw("x # note".into())),
        "(\nx # note\n)"
    );
}

#[test]
fn option_name_splitting() {
    assert_eq!(
        split_option_name("services.printing.enable"),
        vec!["services", "printing", "enable"]
    );
    assert_eq!(
        split_option_name("boot.kernel.sysctl.\"net.ipv4.ip_forward\""),
        vec!["boot", "kernel", "sysctl", "net.ipv4.ip_forward"]
    );
    assert_eq!(split_option_name("a.\"b\\\"c\""), vec!["a", "b\"c"]);

    assert_eq!(split_option_name("a\\b.c"), vec!["a\\b", "c"]);
    assert_eq!(split_option_name(""), vec![""]);
}

#[test]
fn rendered_attribute_paths_split_back() {
    let parts = ["in", "a.b", "x\"y\\z", "${e}", "l\nm\tn\r", "plain-1"];
    let joined = parts.map(render_attr_name).join(".");

    assert_eq!(split_option_name(&joined), parts);
}
