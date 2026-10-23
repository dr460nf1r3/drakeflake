use super::InputKind;

#[test]
fn kinds_keep_the_strings_of_the_lock_file() {
    for name in [
        "github",
        "gitlab",
        "sourcehut",
        "git",
        "path",
        "tarball",
        "file",
        "indirect",
        "follows",
        "mercurial",
        "",
    ] {
        let kind = InputKind::from(name);
        assert_eq!(kind.as_str(), name);
        assert_eq!(kind.to_string(), name);
        assert_eq!(serde_json::to_value(&kind).unwrap(), name);
    }

    assert_eq!(
        InputKind::from("mercurial"),
        InputKind::Other("mercurial".to_owned())
    );
    assert_eq!(InputKind::from(""), InputKind::default());
    assert_eq!(InputKind::default().as_str(), "");
    assert!(InputKind::Sourcehut.is_forge());
    assert!(!InputKind::Git.is_forge());
}
