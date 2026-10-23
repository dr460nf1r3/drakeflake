use serde_json::{Value, json};

use super::*;
use crate::lock::kind::InputKind;

fn attrs(value: &Value) -> Attrs {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

fn urls(kind: &str, locked: &Value, original: &Value) -> [Option<String>; 3] {
    let found = Urls::for_node(&InputKind::from(kind), &attrs(locked), &attrs(original));
    [found.web, found.commit, found.remote]
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "the expected tables mix found and missing addresses"
)]
fn some(text: &str) -> Option<String> {
    Some(text.to_owned())
}

#[test]
fn forges_link_to_their_web_pages_and_git_remotes() {
    let cases = [
        (
            "sourcehut",
            json!({ "owner": "~me", "repo": "cfg", "rev": "1" }),
            [
                some("https://git.sr.ht/~me/cfg"),
                some("https://git.sr.ht/~me/cfg/commit/1"),
                some("https://git.sr.ht/~me/cfg"),
            ],
        ),
        (
            "github",
            json!({ "owner": "o", "repo": "r", "host": "github.example.com" }),
            [
                some("https://github.example.com/o/r"),
                None,
                some("https://github.example.com/o/r.git"),
            ],
        ),
        (
            "gitlab",
            json!({ "owner": "group%2fsub", "repo": "r", "rev": "2" }),
            [
                some("https://gitlab.com/group/sub/r"),
                some("https://gitlab.com/group/sub/r/-/commit/2"),
                some("https://gitlab.com/group/sub/r.git"),
            ],
        ),
    ];

    for (kind, locked, expected) in cases {
        assert_eq!(urls(kind, &locked, &Value::Null), expected, "{kind}");
    }
}

#[test]
fn git_inputs_are_checkable_only_when_remote() {
    let git = |original: Value, locked: Value| urls("git", &locked, &original);

    assert_eq!(
        git(
            json!({ "url": "git+https://example.org/r.git" }),
            Value::Null
        ),
        [
            some("https://example.org/r"),
            None,
            some("https://example.org/r.git")
        ]
    );
    assert_eq!(
        git(json!({ "url": "git+ssh://git@example.org/r" }), Value::Null),
        [None, None, some("ssh://git@example.org/r")]
    );

    assert_eq!(
        git(Value::Null, json!({ "url": "git://example.org/r" })),
        [None, None, some("git://example.org/r")]
    );
    assert_eq!(
        git(json!({ "url": "git+file:///home/me/cfg" }), Value::Null),
        [None, None, None]
    );
    assert_eq!(git(Value::Null, Value::Null), [None, None, None]);
}

#[test]
fn tarballs_link_to_nixpkgs_only_from_nixos_org() {
    let tarball = |url: &str, rev: Option<&str>| {
        let found = Urls::tarball(url, rev);
        [found.web, found.commit, found.remote]
    };

    assert_eq!(
        tarball("https://example.org/src.tar.gz", Some("abc")),
        [None, None, some("https://example.org/src.tar.gz")]
    );
    assert_eq!(
        tarball(
            "http://channels.nixos.org/nixos-unstable/nixexprs.tar.xz",
            None
        ),
        [
            some("https://github.com/NixOS/nixpkgs"),
            None,
            some("http://channels.nixos.org/nixos-unstable/nixexprs.tar.xz")
        ]
    );
    assert_eq!(tarball("file:///srv/src.tar.gz", None), [None, None, None]);
    assert_eq!(
        urls("path", &json!({ "path": "/x" }), &json!({ "path": "/x" })),
        [None, None, None]
    );
}
