use std::time::Instant;

use drakeflake_render::{HostName, OptionName, PackageAttr, UserName};
use serde_json::{Map, Value};

use crate::*;

fn nix() -> Nix {
    Nix::system()
}

fn report(what: &'static str) -> impl Fn(&str, usize) + Sync {
    move |name, count| eprintln!("{what} skip #{count}: {name}")
}

fn target() -> Option<Target> {
    Some(Target {
        flake: std::env::var("DRAKEFLAKE_TEST_FLAKE").ok()?,
        host: std::env::var("DRAKEFLAKE_TEST_HOST")
            .ok()
            .map(HostName::from)?,
        ..Default::default()
    })
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST"]
fn live_docs() {
    let Some(target) = target() else {
        return;
    };

    let mut skip = Vec::new();
    let started = Instant::now();
    let docs = eval_options_doc(
        &nix(),
        &target,
        &OptionTree::System,
        &mut skip,
        &report("docs"),
    )
    .unwrap();

    let docs: Map<String, Value> = serde_json::from_str(&docs).unwrap();
    eprintln!(
        "docs: {} options in {:?}, skipped {:?}",
        docs.len(),
        started.elapsed(),
        skip
    );
    assert!(docs.len() > 1000);
    assert!(docs.contains_key("networking.hostName"));
    let reduced = skip.iter().filter(|entry| entry.starts_with('~')).count();
    let dropped = skip.len() - reduced;
    eprintln!("docs: {reduced} options without descriptions, {dropped} dropped");
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST"]
fn live_fingerprint_is_stable() {
    let Some(target) = target() else {
        return;
    };

    let first = fingerprint(&nix(), &target).unwrap();
    assert!(first.starts_with("sha256-"), "{first}");
    assert_eq!(fingerprint(&nix(), &target).unwrap(), first);
    assert_ne!(
        fingerprint(
            &nix(),
            &Target {
                flake: target.flake,
                host: "other-host".into(),
                ..Default::default()
            }
        )
        .unwrap(),
        first
    );
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST"]
fn live_type_checks() {
    let Some(target) = target() else {
        return;
    };

    let check =
        |name: &str, value: &str| check_value(&nix(), &target, &OptionName::from(name), value);

    assert_eq!(
        check("networking.hostName", "\"good-host\"").unwrap(),
        TypeCheck::Valid
    );
    assert!(matches!(
        check("networking.hostName", "\"bad_host!\""),
        Ok(TypeCheck::Invalid(_))
    ));
    assert_eq!(
        check("services.openssh.enable", "true").unwrap(),
        TypeCheck::Valid
    );
    assert!(matches!(
        check("services.openssh.enable", "\"yes\""),
        Ok(TypeCheck::Invalid(_))
    ));
    assert_eq!(
        check("networking.firewall.allowedTCPPorts", "[ 22 80 ]").unwrap(),
        TypeCheck::Valid
    );
    assert!(matches!(
        check("networking.firewall.allowedTCPPorts", "[ 22 \"x\" ]"),
        Ok(TypeCheck::Invalid(_))
    ));
    assert!(matches!(
        check("networking.firewall.allowedTCPPorts", "[ 22 70000 ]"),
        Ok(TypeCheck::Invalid(_))
    ));
    assert_eq!(
        check("environment.systemPackages", "[ pkgs.hello ]").unwrap(),
        TypeCheck::Valid
    );
    assert!(matches!(
        check("services.openssh.enable", "(throw \"boom\")"),
        Ok(TypeCheck::Failed(_))
    ));
    check("services.openssh.enable", "undefinedVariable").unwrap_err();

    let hello = package_meta(&nix(), &target, &PackageAttr::from("hello")).unwrap();
    assert_eq!(hello["mainProgram"], "hello");
    assert!(hello["homepage"].is_string());
    assert!(
        package_meta(&nix(), &target, &PackageAttr::from("no-such-package-xyz"))
            .unwrap()
            .is_null()
    );
    assert_eq!(check("no.such.option", "1").unwrap(), TypeCheck::Unknown);
}

fn assert_home_manager_values(target: &Target, tree: &OptionTree) {
    let values_started = Instant::now();
    let values: Map<String, Value> = serde_json::from_str(
        &snapshot_values(&nix(), target, tree, &mut Vec::new(), &|_, _| {}).unwrap(),
    )
    .unwrap();

    eprintln!(
        "home-manager values: {} in {:?}",
        values.len(),
        values_started.elapsed()
    );
    assert_eq!(
        values["home-manager.users.alice.programs.git.enable"]["v"],
        true
    );

    let user_files = |name: &str| values[name]["u"].as_array().map_or(0, Vec::len);
    assert!(user_files("home-manager.users.alice.programs.git.enable") > 0);
    assert_eq!(
        user_files("home-manager.users.alice.programs.gitui.enable"),
        0
    );
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_HM_FLAKE and DRAKEFLAKE_TEST_HM_HOST"]
fn live_home_manager() {
    let nix = nix();
    let (Ok(flake), Ok(host)) = (
        std::env::var("DRAKEFLAKE_TEST_HM_FLAKE"),
        std::env::var("DRAKEFLAKE_TEST_HM_HOST"),
    ) else {
        return;
    };

    let target = Target {
        flake,
        host: host.into(),
        ..Default::default()
    };

    let tree = OptionTree::HomeManager("alice".into());

    assert_eq!(
        home_manager_users(&nix, &target).unwrap(),
        Some(vec![UserName::from("alice")])
    );

    let started = Instant::now();
    let docs: Map<String, Value> = serde_json::from_str(
        &eval_options_doc(&nix, &target, &tree, &mut Vec::new(), &|_, _| {}).unwrap(),
    )
    .unwrap();

    eprintln!(
        "home-manager docs: {} options in {:?}",
        docs.len(),
        started.elapsed()
    );
    assert!(docs.contains_key("home-manager.users.alice.programs.git.enable"));

    assert_home_manager_values(&target, &tree);

    let git = "home-manager.users.alice.programs.git.enable".to_owned();
    let check = |value: &str| check_value(&nix, &target, &OptionName::from(&git), value);

    assert_eq!(check("false").unwrap(), TypeCheck::Valid);
    assert!(matches!(check("\"no\""), Ok(TypeCheck::Invalid(_))));
    assert_eq!(
        eval_value(&nix, &target, &OptionName::from(&git)).unwrap(),
        Value::Bool(true)
    );

    let selected: Map<String, Value> = serde_json::from_str(
        &selected_values(&nix, &target, &tree, &[OptionName::from(&git)]).unwrap(),
    )
    .unwrap();

    assert_eq!(selected[&git]["v"], true, "{selected:?}");
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST"]
fn live_selected_values() {
    let Some(target) = target() else {
        return;
    };

    let names = [
        "networking.hostName",
        "users.users.root.shell",
        "no.such.option",
        "services.openssh.enable",
    ]
    .map(OptionName::from);

    let started = Instant::now();
    let selected: Map<String, Value> = serde_json::from_str(
        &selected_values(&nix(), &target, &OptionTree::System, &names).unwrap(),
    )
    .unwrap();

    eprintln!("selected values in {:?}: {selected:?}", started.elapsed());

    assert!(selected["networking.hostName"]["v"].is_string());
    assert_ne!(
        selected["networking.hostName"]["u"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    assert!(selected.contains_key("users.users"));
    assert!(!selected.contains_key("no.such.option"));

    let values = eval_values(&nix(), &target, &names[..2]).unwrap();
    assert!(values["networking.hostName"].is_string());
    assert_eq!(values.len(), 2);
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST"]
fn live_packages_snapshot() {
    let Some(target) = target() else {
        return;
    };

    let mut skip = Vec::new();
    let started = Instant::now();
    let packages = package_index(&nix(), &target, &mut skip, &report("packages")).unwrap();
    eprintln!(
        "packages: {} bytes in {:?}, skipped {:?}",
        packages.len(),
        started.elapsed(),
        skip
    );

    let mut values_skip = Vec::new();
    let values_started = Instant::now();
    let snapshot = snapshot_values(
        &nix(),
        &target,
        &OptionTree::System,
        &mut values_skip,
        &report("values"),
    )
    .unwrap();

    let snapshot: Map<String, Value> = serde_json::from_str(&snapshot).unwrap();
    eprintln!(
        "values: {} options in {:?}, skipped {} ",
        snapshot.len(),
        values_started.elapsed(),
        values_skip.len()
    );
    assert!(snapshot.contains_key("networking.hostName"));
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST"]
fn live_flake_roundtrip() {
    let Some(target) = target() else {
        return;
    };

    let hosts = list_hosts(&nix(), &target.flake).unwrap();
    assert!(hosts.iter().any(|host| host.name == target.host));

    let path = build_options_json(&nix(), &target).unwrap();
    assert!(path.is_file());

    let value = eval_value(&nix(), &target, &OptionName::from("networking.hostName")).unwrap();
    assert!(value.is_string());

    let installed = installed_packages(&nix(), &target).unwrap();
    assert_ne!(installed, Vec::<PackageAttr>::new());

    let mut skip = Vec::new();
    let packages = package_index(&nix(), &target, &mut skip, &|_, _| {}).unwrap();
    assert!(packages.len() > 1000);

    let mut values_skip = Vec::new();
    let started = Instant::now();
    let snapshot = snapshot_values(
        &nix(),
        &target,
        &OptionTree::System,
        &mut values_skip,
        &|name, count| {
            eprintln!("skip #{count}: {name}");
        },
    )
    .unwrap();

    eprintln!(
        "snapshot {} bytes in {:?}, skipped {}",
        snapshot.len(),
        started.elapsed(),
        values_skip.len()
    );
    assert!(snapshot.contains("networking.hostName"));
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST"]
fn live_package_sets() {
    let Some(target) = target() else {
        return;
    };

    let sets = package_sets(&nix(), &target, &mut Vec::new()).unwrap();
    assert!(
        sets.iter()
            .any(|set| set == "python3Packages" || set.starts_with("python3"))
    );

    let set = sets
        .iter()
        .find(|set| set.as_str() == "vimPlugins")
        .unwrap();

    let text = package_set_index(&nix(), &target, set, &mut Vec::new(), &|_, _| {}).unwrap();
    let index = serde_json::from_str::<Value>(&text).unwrap();
    let count = index.as_object().map_or(0, Map::len);
    println!("{set}: {count} packages");
    assert!(count > 100);
    package_set_index(&nix(), &target, "a.b", &mut Vec::new(), &|_, _| {}).unwrap_err();
}
