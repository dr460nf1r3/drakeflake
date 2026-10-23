use drakeflake_plan::{InputMode, ManagedOption, Priority};

use super::*;

const PORTS: &str = "list of 16 bit unsigned integer; between 0 and 65535 (both inclusive)";

#[test]
fn implicit_priorities_match_the_old_output() {
    assert_eq!(
        ManagedOption::typed("boolean", "true").implicit_priority(),
        Priority::Override
    );
    assert_eq!(
        ManagedOption::typed(PORTS, "22").implicit_priority(),
        Priority::Merge
    );
    assert_eq!(
        ManagedOption::typed("null or (list of string)", "null").implicit_priority(),
        Priority::Override
    );
    assert_eq!(
        ManagedOption::typed("string or signed integer", "1").implicit_priority(),
        Priority::Normal
    );
    assert_eq!(
        ManagedOption::typed("attribute set of string", "{ }").implicit_priority(),
        Priority::Merge
    );
}

#[test]
fn priorities_render_their_wrappers() {
    let mut option = ManagedOption::typed("signed integer", "-3");
    assert_eq!(option.render("a").unwrap().text, "lib.mkOverride 90 (-3)");

    for (priority, text) in [
        (Priority::Default, "lib.mkDefault (-3)"),
        (Priority::Normal, "-3"),
        (Priority::Force, "lib.mkForce (-3)"),
    ] {
        option.priority = Some(priority);
        assert_eq!(option.render("a").unwrap().text, text);
    }

    let mut list = ManagedOption::typed(PORTS, "2222");
    for (priority, text) in [
        (Priority::Merge, "[ 2222 ]"),
        (Priority::Before, "lib.mkBefore [ 2222 ]"),
        (Priority::After, "lib.mkAfter [ 2222 ]"),
        (Priority::Replace, "lib.mkForce [ 2222 ]"),
    ] {
        list.priority = Some(priority);
        assert_eq!(list.render("a").unwrap().text, text);
    }

    list.priority = Some(Priority::Default);
    assert!(list.render("a").is_err(), "scalar priority on a list");
}

#[test]
fn choices_and_normalisation() {
    let option = ManagedOption::typed("boolean", "true");
    assert_eq!(
        option.priority_choices(),
        vec![
            Priority::Override,
            Priority::Default,
            Priority::Normal,
            Priority::Force
        ]
    );

    let null_list = ManagedOption::typed("null or (list of string)", "null");
    assert_eq!(
        null_list.priority_choices().first(),
        Some(&Priority::Override)
    );

    let chosen = ManagedOption {
        priority: Some(Priority::Override),
        ..option
    };

    assert_eq!(chosen.normalized().priority, None);
}

#[test]
fn expressions_are_parenthesised() {
    let option = ManagedOption {
        mode: InputMode::Expression,
        priority: Some(Priority::Force),
        ..ManagedOption::typed("string", "config.networking.hostName + \"-vm\"")
    };

    assert_eq!(
        option.render("networking.domain").unwrap(),
        RenderedValue {
            text: "lib.mkForce (config.networking.hostName + \"-vm\")".to_owned(),
            verbatim: true,
        }
    );
    assert!(
        option
            .check_source()
            .unwrap()
            .ends_with("__drakeflakeUnwrap (config.networking.hostName + \"-vm\"))")
    );

    let breakout = ManagedOption {
        mode: InputMode::Expression,
        ..ManagedOption::typed("string", "1); system.evil = true; x = (2")
    };

    breakout.render("a").unwrap_err();
}

#[test]
fn host_scope_rules() {
    let scoped = ManagedOption {
        host: Some("box".into()),
        ..ManagedOption::typed("string", "x")
    };

    scoped.check_settings("networking.domain").unwrap();
    assert!(scoped.check_settings("networking.hostName").is_err());
    assert!(scoped.check_settings("_module.args.x").is_err());
    assert!(!host_scope_allowed("networking.hostName"));

    let empty = ManagedOption {
        host: Some(" ".into()),
        ..ManagedOption::typed("string", "x")
    };

    assert!(empty.check_settings("networking.domain").is_err());
}
