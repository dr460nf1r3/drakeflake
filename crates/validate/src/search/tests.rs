use super::*;

const MODULE: &str = r#"{ config, pkgs, ... }:
let
  sshPort = 22;
in
{
  # ssh only on the LAN
  services.openssh = {
    enable = true;
    ports = [ sshPort ];
  };
  users.users.alice.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAA alice@laptop"
  ];
  environment.systemPackages = with pkgs; [ firefox pkgs.sshfs ];
}
"#;

fn found(text: &str, query: &str) -> Vec<(HitKind, String, String, usize)> {
    search_source(text, query)
        .into_iter()
        .map(|hit| (hit.kind, hit.path, hit.snippet, hit.line))
        .collect()
}

#[test]
fn nested_attributes_are_found_by_their_full_path() {
    let hits = found(MODULE, "openssh");

    assert_eq!(
        hits,
        vec![
            (
                HitKind::Attribute,
                "services.openssh".to_owned(),
                "{ enable = true; ports = [ sshPort ]; }".to_owned(),
                7
            ),
            (
                HitKind::Attribute,
                "users.users.alice.openssh.authorizedKeys.keys".to_owned(),
                "[ \"ssh-ed25519 AAAA alice@laptop\" ]".to_owned(),
                11
            ),
        ]
    );
}

#[test]
fn children_of_a_matching_set_hit_only_with_their_own_name() {
    let hits = found(MODULE, "openssh.enable");

    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].1, "services.openssh.enable");
    assert_eq!(hits[0].3, 8);
}

#[test]
fn values_comments_and_arguments() {
    let hits = found(MODULE, "ssh");
    let lines: Vec<usize> = hits.iter().map(|hit| hit.3).collect();

    assert_eq!(lines, vec![3, 7, 9, 11, 12, 14], "{hits:?}");
    assert!(hits.iter().any(|hit| hit.0 == HitKind::Value
        && hit.1 == "users.users.alice.openssh.authorizedKeys.keys"
        && hit.2 == "\"ssh-ed25519 AAAA alice@laptop\""));
    assert!(
        hits.iter()
            .any(|hit| hit.0 == HitKind::Value && hit.2 == "pkgs.sshfs")
    );
}

#[test]
fn let_bindings_are_not_attributes_of_the_set() {
    let hits = found(MODULE, "sshport");

    assert_eq!(
        hits[0],
        (HitKind::Attribute, "sshPort".to_owned(), "22".to_owned(), 3)
    );
}

#[test]
fn module_arguments_are_not_hits() {
    let hits = found(MODULE, "config");

    assert_eq!(hits, Vec::new());
}

#[test]
fn inherited_names_and_quoted_attributes() {
    let text = "{ pkgs }: {\n  inherit (pkgs) firefox;\n  \"my-host\".enable = true;\n}\n";

    assert_eq!(found(text, "firefox")[0].0, HitKind::Attribute);
    assert_eq!(found(text, "firefox")[0].3, 2);
    assert_eq!(found(text, "my-host")[0].1, "my-host.enable");
}

#[test]
fn a_value_on_the_line_of_its_attribute_is_not_repeated() {
    let hits = found("{ networking.hostName = \"ssh-box\"; }", "ssh");

    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, HitKind::Value);

    let both = found("{ ssh.name = \"ssh-box\"; }", "ssh");

    assert_eq!(both.len(), 1, "{both:?}");
    assert_eq!(both[0].0, HitKind::Attribute);
}

#[test]
fn case_is_ignored_and_empty_queries_find_nothing() {
    assert_eq!(found(MODULE, "FIREFOX").len(), 1);
    assert_eq!(found(MODULE, "  "), Vec::new());
}

#[test]
fn broken_files_are_searched_as_far_as_they_parse() {
    let hits = found("{ services.openssh.enable = true;\n  broken = ", "openssh");

    assert_eq!(hits.len(), 1, "{hits:?}");
}

#[test]
fn long_values_are_shortened() {
    let long = format!("{{ x.ssh = \"{}\"; }}", "a".repeat(200));
    let hit = &found(&long, "ssh")[0];

    assert_eq!(hit.2.chars().count(), SNIPPET_LENGTH);
    assert!(hit.2.ends_with('…'));
}
