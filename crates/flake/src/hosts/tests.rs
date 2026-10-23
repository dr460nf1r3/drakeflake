use drakeflake_exec::testing::{ScriptedRunner, failure, success};

use super::*;
use crate::values::eval_value;

#[test]
fn host_choice() {
    let hosts = vec![
        Host {
            name: "a".to_owned(),
            kind: SystemKind::NixOs,
        },
        Host {
            name: "b".to_owned(),
            kind: SystemKind::Darwin,
        },
    ];

    let host = |name: &str| Host {
        name: name.into(),
        kind: SystemKind::NixOs,
    };

    assert_eq!(
        choose_host(&hosts, Some("b"), None).unwrap(),
        Host {
            name: "b".into(),
            kind: SystemKind::Darwin,
        }
    );
    choose_host(&hosts, Some("c"), None).unwrap_err();
    assert_eq!(choose_host(&hosts, None, Some("a")).unwrap(), host("a"));
    choose_host(&hosts, None, Some("z")).unwrap_err();
    assert_eq!(choose_host(&hosts[..1], None, None).unwrap(), host("a"));
    choose_host(&[], None, None).unwrap_err();
}

#[test]
fn hosts_of_this_machines_kind_come_first() {
    let names = || HostNames {
        nixos: vec!["box".to_owned(), "both".to_owned()],
        darwin: vec!["mac".to_owned(), "both".to_owned()],
    };

    let listed = |native| {
        names()
            .into_hosts(native)
            .into_iter()
            .map(|host| format!("{}:{}", host.kind.as_str(), host.name))
            .collect::<Vec<_>>()
    };

    assert_eq!(
        listed(SystemKind::NixOs),
        ["nixos:box", "nixos:both", "darwin:mac", "darwin:both"]
    );
    assert_eq!(
        listed(SystemKind::Darwin),
        ["darwin:mac", "darwin:both", "nixos:box", "nixos:both"]
    );

    let hosts = names().into_hosts(SystemKind::Darwin);
    assert_eq!(
        choose_host(&hosts, Some("both"), None).unwrap().kind,
        SystemKind::Darwin
    );
    assert_eq!(
        HostNames::default().into_hosts(SystemKind::NixOs),
        Vec::<Host>::new()
    );
}

#[test]
fn an_empty_preference_falls_back_and_ambiguity_names_the_hosts() {
    let hosts = ["a", "b"].map(|name| Host {
        name: name.to_owned(),
        kind: SystemKind::NixOs,
    });

    assert_eq!(choose_host(&hosts, Some(""), Some("b")).unwrap().name, "b");
    assert!(matches!(
        choose_host(&hosts, Some(""), None),
        Err(FlakeError::SeveralHosts(names)) if names == "a, b"
    ));
    let missing = choose_host(&hosts, Some("c"), Some("a")).unwrap_err();
    assert_eq!(
        missing.to_string(),
        "Host \"c\" is not declared in this flake (found: a, b)"
    );
}

#[test]
fn fingerprints_tell_hosts_and_kinds_of_one_flake_apart() {
    let metadata = serde_json::json!({ "locked": { "narHash": "sha256-abc" } });
    let target = |host: &str, kind| Target {
        flake: "/etc/nixos".to_owned(),
        host: host.into(),
        kind,
    };

    let print =
        |host, kind| fingerprint_of(&metadata, &Nix::system(), &target(host, kind)).unwrap();

    let box_nixos = print("box", SystemKind::NixOs);
    assert!(box_nixos.starts_with("sha256-abc "), "{box_nixos}");
    assert!(
        box_nixos.ends_with(&format!(" v{EVAL_VERSION}")),
        "{box_nixos}"
    );
    assert_ne!(box_nixos, print("other", SystemKind::NixOs));
    assert_ne!(box_nixos, print("box", SystemKind::Darwin));

    for unhashed in [
        serde_json::json!({ "locked": {} }),
        serde_json::json!({ "narHash": "sha256-abc" }),
    ] {
        assert!(
            matches!(
                fingerprint_of(&unhashed, &Nix::system(), &target("box", SystemKind::NixOs)),
                Err(FlakeError::NoContentHash)
            ),
            "{unhashed}"
        );
    }
}

#[test]
fn a_missing_output_is_told_from_a_failing_evaluation() {
    let cpp = "error: flake 'path:/x' does not provide attribute \
        'packages.x86_64-linux.darwinConfigurations', \
        'legacyPackages.x86_64-linux.darwinConfigurations' or 'darwinConfigurations'";

    assert!(is_missing_output(cpp, "darwinConfigurations"));
    assert!(is_missing_output(
        "error: attribute 'darwinConfigurations' missing",
        "darwinConfigurations"
    ));
    assert!(!is_missing_output(cpp, "nixosConfigurations"));

    let broken = "error:\n  \u{2026} while calling the 'throw' builtin\n  error: boom";
    assert!(!is_missing_output(broken, "darwinConfigurations"));
    assert!(!is_missing_output(
        "error: path '/x' does not exist",
        "nixosConfigurations"
    ));
    assert!(!is_missing_output(
        "error: attribute 'darwinConfigurations' has a missing option 'x'",
        "darwinConfigurations"
    ));
}

#[test]
fn hosts_of_a_flake_without_darwin_configurations_are_listed_purely() {
    let dir = tempfile::tempdir().unwrap();
    let flake = "{ outputs = _: { nixosConfigurations = { b = { }; a = { }; }; }; }";
    std::fs::write(dir.path().join("flake.nix"), flake).unwrap();

    let path = dir.path().canonicalize().unwrap();
    let hosts = list_hosts(&Nix::system(), &format!("path:{}", path.display())).unwrap();
    let names: Vec<_> = hosts.iter().map(|host| host.name.as_str()).collect();

    assert_eq!(names, ["a", "b"]);
    assert!(hosts.iter().all(|host| host.kind == SystemKind::NixOs));
}

#[test]
fn a_failing_evaluation_is_not_taken_for_a_missing_output() {
    let dir = tempfile::tempdir().unwrap();
    let flake = "{ outputs = _: { nixosConfigurations = throw \"boom\"; }; }";
    std::fs::write(dir.path().join("flake.nix"), flake).unwrap();

    let path = dir.path().canonicalize().unwrap();
    let err = list_hosts(&Nix::system(), &format!("path:{}", path.display())).unwrap_err();

    assert!(err.to_string().contains("boom"), "{err}");
}

#[test]
fn missing_flake_is_an_error() {
    list_hosts(&Nix::system(), "/nonexistent-flake-path").unwrap_err();
    let target = Target {
        flake: "/nonexistent-flake-path".to_owned(),
        host: "host".into(),
        ..Default::default()
    };

    eval_value(&Nix::system(), &target, &"services.printing.enable".into()).unwrap_err();
}

fn scripted_listing(
    nixos: &'static str,
    darwin_stderr: &'static str,
) -> (std::sync::Arc<ScriptedRunner>, Nix) {
    let (runner, exec) = ScriptedRunner::start(move |call| {
        if call.has_arg("path:/flake#nixosConfigurations") {
            Ok(success(nixos))
        } else {
            Ok(failure(darwin_stderr))
        }
    });

    (runner, Nix::new(exec))
}

#[test]
fn a_flake_with_only_nixos_configurations_lists_those_hosts() {
    let missing = "error: flake 'path:/flake' does not provide attribute 'darwinConfigurations'";
    let (runner, nix) = scripted_listing(r#"["b","a"]"#, missing);

    let hosts = list_hosts(&nix, "path:/flake").unwrap();

    assert_eq!(
        hosts
            .iter()
            .map(|host| host.name.as_str())
            .collect::<Vec<_>>(),
        ["b", "a"]
    );
    assert!(hosts.iter().all(|host| host.kind == SystemKind::NixOs));

    let calls = runner.calls();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|call| call.program == "nix"));
    assert!(
        calls
            .iter()
            .all(|call| call.cwd.as_deref() == Some(Path::new("/flake")))
    );
    assert!(calls.iter().all(|call| call.timeout == SHORT_TIMEOUT));
}

#[test]
fn an_evaluation_error_in_one_output_fails_the_listing() {
    let (_, nix) = scripted_listing(r#"["a"]"#, "error: boom");

    let error = list_hosts(&nix, "path:/flake").unwrap_err();

    assert!(error.to_string().contains("boom"), "{error}");
}
