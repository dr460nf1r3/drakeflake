use super::*;

fn lock(nodes: &str, root_inputs: &str) -> String {
    format!(
        r#"{{"nodes":{{{nodes}"root":{{"inputs":{{{root_inputs}}}}}}},"root":"root","version":7}}"#
    )
}

fn only(text: &str) -> LockedInput {
    let mut inputs = parse_lock(text).unwrap();
    assert_eq!(inputs.len(), 1, "{inputs:?}");
    inputs.remove(0)
}

#[test]
fn malformed_lock_files_are_errors() {
    let cases = [
        ("", "flake.lock is not valid JSON"),
        (r#"{"version":7}"#, "flake.lock has no nodes"),
        (
            r#"{"nodes":{"a":{}},"root":"main","version":7}"#,
            "flake.lock has no root node",
        ),
        (
            r#"{"nodes":{"root":{}},"root":"root"}"#,
            "flake.lock has no version",
        ),
        (
            r#"{"nodes":{"root":{}},"root":"root","version":6}"#,
            "flake.lock has version 6, version 7 or newer is supported",
        ),
        (
            r#"{"nodes":{"root":{"inputs":{"a":3}}},"root":"root","version":7}"#,
            "flake.lock is malformed",
        ),
        (
            r#"{"nodes":{"root":{"inputs":{"ghost":"ghost"}}},"root":"root","version":7}"#,
            "flake.lock: input ghost points to the missing node ghost",
        ),
    ];

    for (text, expected) in cases {
        let error = parse_lock(text).unwrap_err().to_string();
        assert!(error.starts_with(expected), "{text:?}: {error}");
    }
}

#[test]
fn a_flake_without_inputs_has_none() {
    let text = r#"{"nodes":{"root":{}},"root":"root","version":7}"#;
    assert_eq!(parse_lock(text).unwrap(), Vec::new());
    let unnamed = r#"{"version":7,"nodes":{"root":{"inputs":{"a":"a"}},"a":{"locked":{"type":"path","path":"/x"},"original":{"type":"path","path":"/x"}}}}"#;
    assert_eq!(only(unnamed).source, "/x");
}

#[test]
fn an_input_without_a_fetcher_type_keeps_what_the_lock_says() {
    let input = only(&lock(r#""x":{"locked":{"rev":"1"}},"#, r#""x":"x""#));

    assert_eq!(input.name, "x");
    assert_eq!(input.kind, InputKind::default());
    assert_eq!(input.rev.as_deref(), Some("1"));
    assert_eq!(input.remote, None);
    assert!(!input.pinned);

    let other = only(&lock(
        r#""x":{"locked":{"type":"mercurial"}},"#,
        r#""x":"x""#,
    ));

    assert_eq!(other.kind, InputKind::Other("mercurial".to_owned()));
}

#[test]
fn sources_are_written_like_flake_references() {
    let node = |locked: &str, original: &str| {
        lock(
            &format!(r#""x":{{"locked":{locked},"original":{original}}},"#),
            r#""x":"x""#,
        )
    };

    let cases = [
        (
            node(
                r#"{"type":"git","url":"https://example.org/r.git","rev":"1"}"#,
                r#"{"type":"git","url":"https://example.org/r.git","ref":"dev"}"#,
            ),
            "https://example.org/r.git?ref=dev",
        ),
        (
            node(
                r#"{"type":"indirect","id":"nixpkgs"}"#,
                r#"{"type":"indirect","id":"nixpkgs"}"#,
            ),
            "flake:nixpkgs",
        ),
        (
            node(r#"{"type":"mercurial"}"#, r#"{"type":"mercurial"}"#),
            "mercurial",
        ),
        (
            node(
                r#"{"type":"sourcehut","owner":"~me","repo":"cfg","rev":"1"}"#,
                r#"{"type":"sourcehut","owner":"~me","repo":"cfg","ref":"main"}"#,
            ),
            "sourcehut:~me/cfg/main",
        ),
    ];

    for (text, expected) in cases {
        assert_eq!(only(&text).source, expected);
    }
}

#[test]
fn only_plain_forge_references_get_a_pin_base() {
    let base = |original: &str| {
        let text = lock(
            &format!(
                r#""x":{{"locked":{{"type":"github","owner":"o","repo":"r","rev":"1"}},"original":{original}}},"#
            ),
            r#""x":"x""#,
        );

        only(&text).pin_base
    };

    assert_eq!(
        base(r#"{"type":"github","owner":"o","repo":"r","ref":"main"}"#).as_deref(),
        Some("github:o/r")
    );
    assert_eq!(
        base(r#"{"type":"github","owner":"o","repo":"r","dir":"sub"}"#),
        None
    );
    assert_eq!(
        base(r#"{"type":"github","owner":"o","repo":"r","host":"git.example.com"}"#),
        None
    );

    let git = lock(
        r#""x":{"locked":{"type":"git","url":"https://e.org/r","rev":"1"},"original":{"type":"git","url":"https://e.org/r"}},"#,
        r#""x":"x""#,
    );

    assert_eq!(only(&git).pin_base, None);
}

#[test]
fn nested_inputs_relate_to_the_flakes_own_inputs() {
    let nodes = r#"
      "nixpkgs":{"locked":{"type":"github","owner":"NixOS","repo":"nixpkgs","rev":"1"},"original":{"type":"github","owner":"NixOS","repo":"nixpkgs"}},
      "nixpkgs_2":{"locked":{"type":"github","owner":"nixos","repo":"NIXPKGS","rev":"2"},"original":{"type":"github","owner":"nixos","repo":"NIXPKGS"}},
      "flake-utils":{"locked":{"type":"github","owner":"numtide","repo":"flake-utils","rev":"3"},"original":{"type":"github","owner":"numtide","repo":"flake-utils"}},
      "tool":{"inputs":{"nixpkgs":"nixpkgs_2","utils":"flake-utils","systems":["tool","utils","systems"],"pkgs":["nixpkgs"]},
              "locked":{"type":"github","owner":"o","repo":"tool","rev":"4"},"original":{"type":"github","owner":"o","repo":"tool"}},
    "#;

    let inputs = parse_lock(&lock(
        nodes,
        r#""nixpkgs":"nixpkgs","tool":"tool","stable":["nixpkgs"]"#,
    ))
    .unwrap();

    let tool = inputs.iter().find(|input| input.name == "tool").unwrap();
    let nested = |name: &str| {
        tool.nested
            .iter()
            .find(|nested| nested.name == name)
            .unwrap()
    };

    assert_eq!(nested("nixpkgs").could_follow.as_deref(), Some("nixpkgs"));
    assert_eq!(nested("nixpkgs").same_as, None);
    assert_eq!(nested("nixpkgs").rev.as_deref(), Some("2"));

    assert_eq!(nested("utils").could_follow, None);
    assert_eq!(nested("utils").same_as, None);

    assert_eq!(
        nested("systems").follows.as_deref(),
        Some("tool/utils/systems")
    );
    assert!(nested("systems").follows_root);
    assert!(nested("pkgs").follows_root);

    let stable = inputs.iter().find(|input| input.name == "stable").unwrap();
    assert_eq!(stable.kind, InputKind::Follows);
    assert_eq!(stable.nested, Vec::new());
}

#[test]
fn a_follows_path_outside_the_flakes_inputs_is_not_rooted() {
    let nodes = r#"
      "tool":{"inputs":{"nixpkgs":["elsewhere","nixpkgs"]},"locked":{"type":"github","owner":"o","repo":"tool","rev":"4"},"original":{"type":"github","owner":"o","repo":"tool"}},
    "#;

    let tool = only(&lock(nodes, r#""tool":"tool""#));

    assert_eq!(tool.nested[0].follows.as_deref(), Some("elsewhere/nixpkgs"));
    assert!(!tool.nested[0].follows_root);
}

#[test]
fn reading_a_lock_file_names_the_file_on_failure() {
    let dir = tempfile::tempdir().unwrap();

    let missing = read_lock(dir.path()).unwrap_err().to_string();
    assert!(missing.starts_with("Cannot read "), "{missing}");
    assert!(missing.contains("flake.lock"), "{missing}");

    std::fs::write(dir.path().join("flake.lock"), "{").unwrap();
    read_lock(dir.path()).unwrap_err();

    std::fs::write(
        dir.path().join("flake.lock"),
        lock(
            r#""a":{"locked":{"type":"path","path":"/a"},"original":{"type":"path","path":"/a"}},"#,
            r#""a":"a""#,
        ),
    )
    .unwrap();
    assert_eq!(read_lock(dir.path()).unwrap()[0].name, "a");
}
