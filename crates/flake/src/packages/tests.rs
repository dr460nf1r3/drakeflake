use super::*;

#[test]
fn only_plain_attribute_names_are_spliced_as_package_sets() {
    for name in [
        "python3Packages",
        "haskellPackages",
        "kdePackages",
        "_x",
        "a-b",
    ] {
        assert!(is_package_set_name(name), "{name}");
    }

    for name in ["", "a.b", "a b", "x\"", "${x}", "pkgs; builtins.exec", "ä"] {
        assert!(!is_package_set_name(name), "{name}");
    }
}

#[test]
fn a_bad_set_name_is_refused_before_anything_is_evaluated() {
    let target = Target {
        flake: "/etc/nixos".to_owned(),
        host: "box".into(),
        ..Target::default()
    };

    let refused = package_set_index(
        &Nix::system(),
        &target,
        "x or (import ./evil.nix)",
        &mut Vec::new(),
        &|_, _| {
            panic!("nothing may be evaluated");
        },
    );

    assert!(matches!(
        refused,
        Err(FlakeError::NotPackageSet(set)) if set == "x or (import ./evil.nix)"
    ));
}

#[test]
fn package_sets_are_the_marked_attributes_sorted() {
    let merged: Map<String, Value> = serde_json::from_str(
        r#"{ "xorg": true, "hello": false, "aspellDicts": true, "broken": null, "lib": "yes" }"#,
    )
    .unwrap();

    assert_eq!(set_names(merged), ["aspellDicts", "xorg"]);
}
