use super::*;

#[test]
fn userinfo_and_query_values_are_hidden() {
    assert_eq!(
        redact_urls("https://user:token@host.example/a/b?token=abc&x=1#frag"),
        "https://***@host.example/a/b?token=***&x=***#frag"
    );
    assert_eq!(
        redact_urls("git ls-remote ssh://git@host:22/repo.git"),
        "git ls-remote ssh://***@host:22/repo.git"
    );
}

#[test]
fn text_without_credentials_is_kept() {
    assert_eq!(
        redact_urls("github:NixOS/nixpkgs/nixos-unstable"),
        "github:NixOS/nixpkgs/nixos-unstable"
    );
    assert_eq!(
        redact_urls("https://example.org/a/b"),
        "https://example.org/a/b"
    );
    assert_eq!(redact_urls("a\n  b"), "a b");
}

#[test]
fn urls_inside_longer_arguments_are_found() {
    assert_eq!(
        redact_urls("fetchTarball \"https://u:p@h.example/x.tar.gz?token=s"),
        "fetchTarball \"https://***@h.example/x.tar.gz?token=***"
    );
}
