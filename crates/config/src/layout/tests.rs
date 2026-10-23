use super::*;

#[test]
fn rules_are_listed_back_as_text() {
    let rules = LayoutConfig::parse_rules(
        "# remote access\nservices.openssh = remote-access\nservices = services.nix\n",
    )
    .unwrap();

    let config = LayoutConfig {
        mode: LayoutMode::Single,
        rules,
    };

    assert_eq!(
        config.rules_text(),
        "services.openssh = remote-access.nix\nservices = services.nix"
    );
}

#[test]
fn bad_rules_are_rejected() {
    for bad in [
        "services.openssh",
        "= x.nix",
        "a..b = x.nix",
        "a = /etc/x.nix",
        "a = ../x.nix",
        "a = sub/../x.nix",
        "a = we ird.nix",
        "a = ",
    ] {
        assert!(LayoutConfig::parse_rules(bad).is_err(), "{bad}");
    }

    assert_eq!(
        LayoutConfig::parse_rules("  \n# only comments\n"),
        Ok(Vec::new())
    );
}

#[test]
fn a_rule_in_a_config_file_goes_through_the_same_checks() {
    let parse = |file: &str| {
        toml::from_str::<LayoutConfig>(&format!(
            "[[rules]]\nprefix = \"services\"\nfile = \"{file}\"\n"
        ))
        .unwrap()
    };

    for bad in ["/abs/x.nix", "../x.nix", "a/../x.nix", "we ird.nix", ""] {
        assert_eq!(parse(bad).rules, Vec::new(), "{bad}");
    }

    let empty_prefix = "[[rules]]\nprefix = \"a..b\"\nfile = \"x.nix\"\n";
    assert_eq!(
        toml::from_str::<LayoutConfig>(empty_prefix).unwrap().rules,
        Vec::new()
    );

    let good = parse("./net/x");
    assert_eq!(good.rules.first().map(LayoutRule::file), Some("net/x.nix"));
}

#[test]
fn rules_that_are_not_even_a_table_with_both_keys_are_dropped_too() {
    let text = "[[layout.rules]]\nprefix = \"a\"\n\n[[layout.rules]]\nfile = \"x.nix\"\n";
    let config = toml::from_str::<crate::Config>(text).unwrap();

    assert_eq!(config.layout.rules, Vec::new());
    assert_eq!(dropped_rules(text).len(), 2);
}

#[test]
fn the_messages_name_the_rule_and_the_reason() {
    let text = "[[layout.rules]]\nprefix = \"services\"\nfile = \"/etc/x.nix\"\n";

    assert_eq!(
        dropped_rules(text),
        [
            "Ignored layout rule 1 (\"services\") in config.toml: \"/etc/x.nix\" must be a relative file name"
        ]
    );
}

#[test]
fn a_rule_built_in_code_stays_strict() {
    LayoutRule::new("services", "/abs.nix").unwrap_err();
    LayoutRule::new("services", "../x.nix").unwrap_err();
}
