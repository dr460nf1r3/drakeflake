use std::time::Instant;

use serde_json::Value;

use crate::{OptionIndex, OptionName, PackageAttr, PackageIndex, ValueSnapshot, strip_roles};

fn fixture() -> OptionIndex {
    OptionIndex::from_json_str(
        r#"{
            "services.printing.enable": {"description": "Whether to enable printing support.\n\nMore text.", "type": "boolean", "default": {"_type": "literalExpression", "text": "false"}, "declarations": ["nixos/modules/services/printing/cupsd.nix"], "readOnly": false},
            "services.printing.drivers": {"description": "Printer drivers", "type": "list of package"},
            "networking.hostName": {"description": "The hostname", "type": "string", "example": "laptop"},
            "services.openssh.enable": {"description": "Whether to enable the OpenSSH secure shell daemon.", "type": "boolean"},
            "services.openssh.settings.PermitRootLogin": {"description": "Whether root can login using ssh.", "type": "one of \"yes\", \"no\""},
            "users.users.<name>.isNormalUser": {"description": "normal", "type": "boolean"},
            "_module.args": {"description": "internal", "type": "lazy attribute set"},
            "broken": "not an object"
        }"#,
    )
    .unwrap()
}

fn names(index: &OptionIndex, hits: Vec<usize>) -> Vec<String> {
    hits.into_iter()
        .filter_map(|hit| index.entry(hit).map(|entry| entry.name.to_string()))
        .collect()
}

fn option_name(name: &str) -> OptionName {
    OptionName::from(name)
}

fn package_attr(attr: &str) -> PackageAttr {
    PackageAttr::from(attr)
}

#[test]
fn parses_and_skips_malformed() {
    let index = fixture();

    assert_eq!(index.len(), 6);
    let printing = index.get(&option_name("services.printing.enable")).unwrap();
    assert_eq!(printing.default_text.as_deref(), Some("false"));
    assert_eq!(printing.summary(), "Whether to enable printing support.");
    assert_eq!(printing.namespace(), "services");
    assert!(
        index
            .get(&option_name("users.users.<name>.isNormalUser"))
            .unwrap()
            .templated
    );
    assert_eq!(
        index
            .get(&option_name("networking.hostName"))
            .unwrap()
            .example_text
            .as_deref(),
        Some("laptop")
    );
}

#[test]
fn search_ranks_name_hits_above_description_hits() {
    let index = fixture();

    let hits = names(&index, index.search("ssh", |_| true));
    assert_eq!(hits[0], "services.openssh.enable");
    assert_eq!(hits.len(), 2);

    let printing = names(&index, index.search("printing enable", |_| true));
    assert_eq!(printing, vec!["services.printing.enable"]);

    assert_eq!(index.search("no such thing", |_| true), Vec::<usize>::new());
    assert_eq!(index.search("", |_| true).len(), 6);
    assert_eq!(index.search("", |entry| !entry.templated).len(), 5);
}

#[test]
fn dotted_prefix_query() {
    let index = fixture();
    let hits = names(&index, index.search("services.openssh", |_| true));

    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|name| name.starts_with("services.openssh")));
}

#[test]
fn roles_are_stripped() {
    assert_eq!(
        strip_roles("use {command}`sftp` and {option}`a.b`."),
        "use sftp and a.b."
    );
    assert_eq!(strip_roles("{ a = 1; } {x}"), "{ a = 1; } {x}");
    assert_eq!(strip_roles("{file}`unterminated"), "unterminated");
    assert_eq!(strip_roles("ünï {var}`ç`"), "ünï ç");
}

#[test]
fn namespaces_counted() {
    let index = fixture();
    assert_eq!(index.namespaces()[0], ("services".to_owned(), 4));
}

#[test]
fn option_fields_are_read_in_all_their_shapes() {
    let index = OptionIndex::from_json_str(
        r#"{
            "a.list": {"default": [1, 2], "example": {"_type": "literalMD"}, "readOnly": true,
                       "declarations": ["/nix/store/x/a.nix", {"name": "<hm/b.nix>", "url": "https://u"}, {"url": "https://only-url"}, 3]},
            "": {"type": "boolean"}
        }"#,
    )
    .unwrap();

    assert_eq!(index.len(), 1);
    let option = index.get(&option_name("a.list")).unwrap();
    assert_eq!(option.type_name, "unspecified");
    assert_eq!(option.default_text.as_deref(), Some("[\n  1,\n  2\n]"));
    assert_eq!(option.example_text, None);
    assert!(option.read_only);
    assert_eq!(
        option.declarations,
        ["/nix/store/x/a.nix", "<hm/b.nix>", "https://only-url"]
    );
}

#[test]
fn invalid_json_is_an_error_not_a_panic() {
    OptionIndex::from_json_str("").unwrap_err();
    OptionIndex::from_json_str("[1,2]").unwrap_err();
    PackageIndex::from_json_str("{").unwrap_err();
    ValueSnapshot::from_json_str("null").unwrap_err();
}

#[test]
fn packages() {
    let index = PackageIndex::from_json_str(
        r#"{"firefox": {"v": "1.0", "d": "Web browser"}, "firefox-esr": {"v": "0.9", "d": "Web browser (ESR)"}, "git": {"d": "Version control"}, "chromium": {"v": "2", "d": "Another web browser"}, "lib": null, "broken": null}"#,
    )
    .unwrap();

    let hits: Vec<&str> = index
        .search("firefox")
        .into_iter()
        .filter_map(|hit| index.entry(hit).map(|package| package.attr.as_str()))
        .collect();

    assert_eq!(hits, vec!["firefox", "firefox-esr"]);

    assert_eq!(index.len(), 4);

    let browsers = index.search("browser");
    assert_eq!(browsers.len(), 3);
    assert!(index.contains(&package_attr("git")));
    assert!(!index.contains(&package_attr("gi")));
    assert_eq!(
        index.entry(3).map(|package| package.version.as_str()),
        Some("")
    );
}

#[test]
fn snapshot() {
    let snapshot = ValueSnapshot::from_json_str(
        r#"{"a.b": {"v": true, "u": ["/etc/nixos/configuration.nix"]}, "c": {"v": 3, "u": []}}"#,
    )
    .unwrap();

    assert_eq!(
        snapshot.value(&option_name("a.b")),
        Some(&Value::Bool(true))
    );
    assert!(snapshot.is_user_set(&option_name("a.b")));
    assert!(!snapshot.is_user_set(&option_name("c")));
    assert_eq!(
        snapshot.user_files(&option_name("missing")),
        Vec::<String>::new()
    );
}

#[test]
fn snapshot_without_values_of_undefined_options() {
    let snapshot = ValueSnapshot::from_json_str(
        r#"{"a.b": {"v": false, "u": ["/flake/drakeflake.nix"]}, "c.d": {"u": []}, "e": {"v": null, "u": ["/x.nix"]}}"#,
    )
    .unwrap();

    assert_eq!(snapshot.options(), 3);
    assert_eq!(snapshot.len(), 2);
    assert_eq!(
        snapshot.value(&option_name("a.b")),
        Some(&Value::Bool(false))
    );

    assert_eq!(snapshot.value(&option_name("c.d")), None);
    assert!(!snapshot.is_user_set(&option_name("c.d")));

    assert_eq!(snapshot.value(&option_name("e")), Some(&Value::Null));
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_OPTIONS_JSON"]
fn real_options_json_search_speed() {
    let Ok(path) = std::env::var("DRAKEFLAKE_TEST_OPTIONS_JSON") else {
        return;
    };

    let started = Instant::now();
    let index = OptionIndex::from_json_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    println!("loaded {} options in {:?}", index.len(), started.elapsed());

    for query in [
        "ssh",
        "services openssh enable",
        "firewall port",
        "e",
        "networking.",
    ] {
        let query_started = Instant::now();
        let hits = index.search(query, |entry| !entry.templated);
        println!(
            "{query:?}: {} hits in {:?}; top: {:?}",
            hits.len(),
            query_started.elapsed(),
            hits.iter()
                .take(3)
                .filter_map(|hit| index.entry(*hit).map(|entry| entry.name.as_str()))
                .collect::<Vec<_>>()
        );
    }
}
#[test]
#[ignore = "needs DRAKEFLAKE_TEST_PACKAGES_JSON"]
fn real_packages_json_search() {
    let Ok(path) = std::env::var("DRAKEFLAKE_TEST_PACKAGES_JSON") else {
        return;
    };

    let index = PackageIndex::from_json_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let hits = index.search("firefox");
    println!("{} packages, {} hits for firefox", index.len(), hits.len());
    assert_ne!(hits, Vec::<usize>::new());
}
