use super::*;
use crate::expr::{SNAPSHOT, resolve};

#[test]
fn option_trees() {
    assert_eq!(OptionTree::System.prefix(), "");
    assert_eq!(
        OptionTree::HomeManager("nico".into()).prefix(),
        "home-manager.users.nico."
    );
    assert_eq!(
        OptionTree::HomeManager("first.last".into()).prefix(),
        "home-manager.users.\"first.last\"."
    );
    assert!(
        OptionTree::HomeManager("x".into())
            .config_expr()
            .contains("sys.config.home-manager.users.${\"x\"}")
    );
    assert_eq!(
        prefix_keys(
            serde_json::from_str(r#"{"programs.git.enable":1}"#).unwrap(),
            "home-manager.users.a."
        ),
        r#"{"home-manager.users.a.programs.git.enable":1}"#
    );
    assert!(!resolve(SNAPSHOT).contains("__"));
}

#[test]
fn absolute_names_are_not_prefixed_twice() {
    let map =
        serde_json::from_str(r#"{"home-manager.users.a.home.stateVersion":1,"home.username":2}"#)
            .unwrap();

    assert_eq!(
        prefix_keys(map, "home-manager.users.a."),
        r#"{"home-manager.users.a.home.stateVersion":1,"home-manager.users.a.home.username":2}"#
    );
}

#[test]
fn user_names_are_quoted_when_they_are_no_plain_attribute() {
    let cases = [
        ("nico", "nico"),
        ("_svc", "_svc"),
        ("o'brien", "o'brien"),
        ("first-last", "first-last"),
        ("1user", "\"1user\""),
        ("-dash", "\"-dash\""),
        ("a\"b", "\"a\\\"b\""),
        ("", "\"\""),
    ];

    for (name, quoted) in cases {
        assert_eq!(quote_name(name), quoted, "{name}");
    }
}

#[test]
fn walker_output_is_flattened() {
    let nested: Value = serde_json::from_str(
        r#"{
            "services": {
                "services.a": { "_drakeflakeOption": "services.a", "type": "boolean" },
                "services.b": null,
                "services.users": {
                    "_drakeflakeOption": "services.users",
                    "type": "attribute set of (submodule)",
                    "_drakeflakeSub": { "services.users.<name>": {
                        "services.users.<name>.home": { "_drakeflakeOption": "services.users.<name>.home", "type": "string" }
                    } }
                }
            }
        }"#,
    )
    .unwrap();

    let flat = flatten_options(nested);
    let mut names: Vec<&String> = flat.keys().collect();
    names.sort();
    assert_eq!(
        names,
        vec!["services.a", "services.users", "services.users.<name>.home"]
    );
    assert!(flat["services.users"].get("_drakeflakeSub").is_none());
    assert_eq!(flatten_options(Value::Null), Map::default());
}
