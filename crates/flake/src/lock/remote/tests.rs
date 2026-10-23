use super::*;

fn captured(success: bool, stdout: &str, stderr: &str) -> Captured {
    Captured {
        code: Some(i32::from(!success)),
        success,
        stdout: stdout.to_owned(),
        stderr: stderr.to_owned(),
    }
}

#[test]
fn ls_remote_failures_show_the_last_line_of_git() {
    let cases = [
        (
            captured(
                false,
                "",
                "remote: Not Found\nfatal: repository 'x' not found\n",
            ),
            Err("fatal: repository 'x' not found".to_owned()),
        ),
        (
            captured(false, "", "  \n"),
            Err("git ls-remote failed".to_owned()),
        ),
        (
            captured(false, "", ""),
            Err("git ls-remote failed".to_owned()),
        ),
        (
            captured(true, "", ""),
            Err("main not found upstream".to_owned()),
        ),
        (
            captured(true, "1111\trefs/heads/main\n", "warning: redirecting"),
            Ok("1111".to_owned()),
        ),
    ];

    for (run, expected) in cases {
        let found = ls_remote_rev(&run, Some("main")).map_err(|error| error.to_string());
        assert_eq!(found, expected, "{run:?}");
    }

    assert_eq!(
        ls_remote_rev(&captured(true, "", ""), None).map_err(|error| error.to_string()),
        Err("HEAD not found upstream".to_owned())
    );
}

#[test]
fn refs_are_matched_branch_tag_then_literal() {
    let output = "1111\trefs/tags/v2\n2222\trefs/pull/1/head\nnot a ref line\n";

    assert_eq!(pick_rev(output, Some("v2")).as_deref(), Some("1111"));

    assert_eq!(
        pick_rev(output, Some("refs/pull/1/head")).as_deref(),
        Some("2222")
    );

    let both = "3333\trefs/tags/main\n4444\trefs/heads/main\n";
    assert_eq!(pick_rev(both, Some("main")).as_deref(), Some("4444"));
}

#[test]
fn an_unlisted_ref_has_no_revision() {
    let output = "5555\trefs/remotes/origin/main\n6666\trefs/remotes/fork/main\n";
    assert_eq!(pick_rev(output, Some("main")), None);
    assert_eq!(pick_rev("garbage without tabs\n", Some("main")), None);
    assert_eq!(
        ls_remote_rev(&captured(true, output, ""), Some("main")).map_err(|error| error.to_string()),
        Err("main not found upstream".to_owned())
    );
}

#[test]
fn the_immutable_link_after_the_last_redirect_wins() {
    let headers = "HTTP/2 301\r\n\
        Link: <https://a.example/first.tar.gz?rev=1>; rel=\"immutable\"\r\n\
        location: https://b.example\r\n\
        \r\n\
        HTTP/2 200\r\n\
        LINK: <https://b.example/docs>; rel=\"help\", <https://b.example/second.tar.gz?rev=2>; rel=\"immutable\"\r\n";

    assert_eq!(
        parse_immutable_link(headers).as_deref(),
        Some("https://b.example/second.tar.gz?rev=2")
    );
    assert_eq!(
        parse_immutable_link("link: <https://x.example/help>; rel=\"help\"\r\n"),
        None
    );
    assert_eq!(
        parse_immutable_link("link: https://x.example/no-brackets; rel=\"immutable\"\r\n"),
        None
    );
    assert_eq!(
        parse_immutable_link("x-link: <https://x.example/a>; rel=\"immutable\"\r\n"),
        None
    );
}

#[test]
fn commas_inside_link_urls_do_not_split_entries() {
    let headers = "HTTP/2 200\r\n\
        Link: <https://x.example/a,b.tar.gz?rev=1>; rel=\"immutable\", <https://x.example/help>; rel=\"help\"\r\n";

    assert_eq!(
        parse_immutable_link(headers).as_deref(),
        Some("https://x.example/a,b.tar.gz?rev=1")
    );
}

#[test]
fn url_parts() {
    let url = "https://releases.nixos.org/nixos/unstable/nixos-26.11pre1.c593/nixexprs.tar.zst?rev=c593&lastModified=17";

    assert_eq!(
        without_query(url),
        "https://releases.nixos.org/nixos/unstable/nixos-26.11pre1.c593/nixexprs.tar.zst"
    );
    assert_eq!(without_query("https://x.example/a"), "https://x.example/a");
    assert_eq!(query_param(url, "lastModified").as_deref(), Some("17"));
    assert_eq!(query_param(url, "missing"), None);
    assert_eq!(query_param("https://x.example/a", "rev"), None);
    assert_eq!(query_param("https://x.example/a?revision=1", "rev"), None);
    assert_eq!(release_name("nixexprs.tar.zst"), "nixexprs.tar.zst");
}
