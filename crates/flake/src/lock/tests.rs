use std::path::Path;

use drakeflake_exec::Context;

use super::check::{check_update, compare_url};
use super::parse::parse_lock;
use super::remote::{parse_immutable_link, pick_rev, query_param, release_name};
use super::*;

const LOCK: &str = r#"{
  "nodes": {
    "home-manager": {
      "inputs": { "nixpkgs": ["nixpkgs"] },
      "locked": { "lastModified": 1700000000, "owner": "nix-community", "repo": "home-manager", "rev": "aaaa", "type": "github" },
      "original": { "owner": "nix-community", "repo": "home-manager", "type": "github" }
    },
    "nixpkgs": {
      "locked": { "lastModified": 1700000001, "owner": "NixOS", "repo": "nixpkgs", "rev": "bbbb", "type": "github" },
      "original": { "owner": "NixOS", "ref": "nixos-unstable", "repo": "nixpkgs", "type": "github" }
    },
    "pinned": {
      "locked": { "owner": "o", "repo": "r", "rev": "cccc", "type": "github" },
      "original": { "owner": "o", "repo": "r", "rev": "cccc", "type": "github" }
    },
    "subsystem": {
      "locked": { "type": "git", "url": "https://gitlab.com/dr460nf1r3/nix-subsystem", "rev": "dddd", "ref": "refs/heads/main" },
      "original": { "type": "git", "url": "https://gitlab.com/dr460nf1r3/nix-subsystem" }
    },
    "local": {
      "locked": { "path": "./sub", "type": "path" },
      "original": { "path": "./sub", "type": "path" }
    },
    "root": {
      "inputs": { "home-manager": "home-manager", "nixpkgs": "nixpkgs", "pinned": "pinned", "subsystem": "subsystem", "local": "local", "stable": ["nixpkgs"] }
    }
  },
  "root": "root",
  "version": 7
}"#;

fn input<'a>(inputs: &'a [LockedInput], name: &str) -> &'a LockedInput {
    inputs.iter().find(|input| input.name == name).unwrap()
}

#[test]
fn parses_direct_inputs() {
    let inputs = parse_lock(LOCK).unwrap();
    assert_eq!(inputs.len(), 6);

    let nixpkgs = input(&inputs, "nixpkgs");
    assert_eq!(nixpkgs.source, "github:NixOS/nixpkgs/nixos-unstable");
    assert_eq!(nixpkgs.reference.as_deref(), Some("nixos-unstable"));
    assert_eq!(nixpkgs.rev.as_deref(), Some("bbbb"));
    assert_eq!(
        nixpkgs.remote.as_deref(),
        Some("https://github.com/NixOS/nixpkgs.git")
    );
    assert_eq!(
        nixpkgs.commit_url.as_deref(),
        Some("https://github.com/NixOS/nixpkgs/commit/bbbb")
    );

    let home = input(&inputs, "home-manager");
    assert_eq!(home.nested.len(), 1);
    assert_eq!(home.nested[0].follows.as_deref(), Some("nixpkgs"));
    assert!(home.nested[0].follows_root);
    assert_eq!(home.last_modified, Some(1_700_000_000));

    assert!(input(&inputs, "pinned").pinned);
    assert_eq!(input(&inputs, "stable").follows.as_deref(), Some("nixpkgs"));

    let subsystem = input(&inputs, "subsystem");
    assert_eq!(
        subsystem.remote.as_deref(),
        Some("https://gitlab.com/dr460nf1r3/nix-subsystem")
    );
    assert_eq!(input(&inputs, "local").remote, None);
}

#[test]
fn nested_copies_and_compare_links() {
    let lock = r#"{"nodes":{
      "nixpkgs":{"locked":{"type":"github","owner":"NixOS","repo":"nixpkgs","rev":"aaaaaaa1"},"original":{"type":"github","owner":"NixOS","repo":"nixpkgs"}},
      "nixpkgs_2":{"locked":{"type":"github","owner":"NixOS","repo":"nixpkgs","rev":"bbbbbbb2"},"original":{"type":"github","owner":"NixOS","repo":"nixpkgs"}},
      "tool":{"inputs":{"nixpkgs":"nixpkgs_2","shared":"nixpkgs"},"locked":{"type":"github","owner":"o","repo":"tool","rev":"ccccccc3"},"original":{"type":"github","owner":"o","repo":"tool"}},
      "root":{"inputs":{"nixpkgs":"nixpkgs","tool":"tool"}}},"root":"root","version":7}"#;

    let inputs = parse_lock(lock).unwrap();
    let tool = input(&inputs, "tool");
    let own = tool
        .nested
        .iter()
        .find(|nested| nested.name == "nixpkgs")
        .unwrap();

    assert_eq!(own.could_follow.as_deref(), Some("nixpkgs"));
    assert_eq!(own.rev.as_deref(), Some("bbbbbbb2"));
    let shared = tool
        .nested
        .iter()
        .find(|nested| nested.name == "shared")
        .unwrap();

    assert_eq!(shared.same_as.as_deref(), Some("nixpkgs"));

    assert_eq!(
        compare_url(input(&inputs, "nixpkgs"), "ddddddd4").as_deref(),
        Some("https://github.com/NixOS/nixpkgs/compare/aaaaaaa1...ddddddd4")
    );
    assert_eq!(
        compare_url(input(&inputs, "nixpkgs"), "nixos-26.11pre1"),
        None
    );
}

#[test]
fn gitlab_subgroups() {
    let lock = r#"{"nodes":{
      "app":{"locked":{"type":"gitlab","owner":"dr460nf1r3%2Fmisc","repo":"drakeflake","rev":"aaaaaaa1"},"original":{"type":"gitlab","owner":"dr460nf1r3%2Fmisc","repo":"drakeflake"}},
      "root":{"inputs":{"app":"app"}}},"root":"root","version":7}"#;

    let inputs = parse_lock(lock).unwrap();
    let app = input(&inputs, "app");

    assert_eq!(app.source, "gitlab:dr460nf1r3%252Fmisc/drakeflake");
    assert_eq!(
        app.pin_base.as_deref(),
        Some("gitlab:dr460nf1r3%252Fmisc/drakeflake")
    );
    assert_eq!(
        app.remote.as_deref(),
        Some("https://gitlab.com/dr460nf1r3/misc/drakeflake.git")
    );
    assert_eq!(
        app.commit_url.as_deref(),
        Some("https://gitlab.com/dr460nf1r3/misc/drakeflake/-/commit/aaaaaaa1")
    );
    assert_eq!(
        compare_url(app, "bbbbbbb2").as_deref(),
        Some("https://gitlab.com/dr460nf1r3/misc/drakeflake/-/compare/aaaaaaa1...bbbbbbb2")
    );
}

#[test]
fn unchecked_inputs_say_why() {
    let inputs = parse_lock(LOCK).unwrap();

    for name in ["pinned", "local", "stable"] {
        assert!(
            matches!(
                check_update(&Context::system(), input(&inputs, name)),
                UpdateCheck::NotCheckable { reason: _ }
            ),
            "{name}"
        );
    }
}

#[test]
fn picks_branch_then_tag() {
    let output = "1111\trefs/heads/main\n2222\trefs/tags/v1\n3333\trefs/tags/v1^{}\n4444\tHEAD\n";
    assert_eq!(pick_rev(output, Some("main")).as_deref(), Some("1111"));
    assert_eq!(pick_rev(output, Some("v1")).as_deref(), Some("3333"));
    assert_eq!(pick_rev(output, None).as_deref(), Some("4444"));
    assert_eq!(pick_rev("", None), None);
}

#[test]
fn tarball_links() {
    let headers = "HTTP/2 302 \r\nlink: <https://releases.nixos.org/nixos/unstable/nixos-26.11pre1.c593/nixexprs.tar.zst?rev=c593&lastModified=1>; rel=\"immutable\"\r\nlocation: x\r\n";
    let link = parse_immutable_link(headers).unwrap();
    assert_eq!(query_param(&link, "rev").as_deref(), Some("c593"));
    assert_eq!(release_name(&link), "nixos-26.11pre1.c593");
    assert_eq!(
        parse_immutable_link("HTTP/2 200\r\ncontent-type: x\r\n"),
        None
    );

    let lock = r#"{"nodes":{"nixpkgs":{"locked":{"type":"tarball","url":"https://releases.nixos.org/x/nixexprs.tar.zst?rev=c593","rev":"c593"},"original":{"type":"tarball","url":"https://channels.nixos.org/nixos-unstable/nixexprs.tar.zst"}},"root":{"inputs":{"nixpkgs":"nixpkgs"}}},"root":"root","version":7}"#;
    let inputs = parse_lock(lock).unwrap();
    let nixpkgs = input(&inputs, "nixpkgs");
    assert_eq!(
        nixpkgs.source,
        "https://channels.nixos.org/nixos-unstable/nixexprs.tar.zst"
    );
    assert_eq!(
        nixpkgs.commit_url.as_deref(),
        Some("https://github.com/NixOS/nixpkgs/commit/c593")
    );
    assert_eq!(
        nixpkgs.remote.as_deref(),
        Some("https://channels.nixos.org/nixos-unstable/nixexprs.tar.zst")
    );
}

#[test]
fn input_names() {
    assert!(is_input_name("nixpkgs"));
    assert!(is_input_name("home-manager"));
    assert!(!is_input_name("--commit-lock-file"));
    assert!(!is_input_name("a/b"));
    assert!(!is_input_name(""));
    assert!(!is_input_name("-x"));
    assert!(!is_input_name("."));
    assert!(!is_input_name(".."));
    assert!(!is_input_name("a b"));
    assert!(!is_input_name("a\"b"));
    assert!(!is_input_name("h\u{f6}st"));
    assert!(is_input_name("_private.v2-x"));
    parse_lock("nope").unwrap_err();
}

#[test]
#[ignore = "needs network; DRAKEFLAKE_LOCK_DIR=<flake dir>"]
fn live_check_updates() {
    let dir = std::env::var("DRAKEFLAKE_LOCK_DIR").unwrap_or_else(|_| "../..".to_owned());
    let inputs = read_lock(Path::new(&dir)).unwrap();
    let checks = check_updates(&Context::system(), &inputs);

    assert_eq!(inputs.len(), checks.len());
    for (input, check) in &checks {
        println!(
            "{} {} {:?} -> {check:?}",
            input.name, input.source, input.rev
        );
    }

    assert!(
        checks
            .iter()
            .any(|(_, check)| !matches!(check, UpdateCheck::Failed { error: _ }))
    );
}
