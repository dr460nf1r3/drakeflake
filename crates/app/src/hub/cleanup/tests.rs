use super::*;
use crate::hub::test_support::texts;

#[test]
fn nh_clean_arguments() {
    assert_eq!(
        texts(&clean_args(0, 7, Optimise::Run, None, Confirm::Ask)).join(" "),
        "clean all --keep 1 --keep-since 7d --ask --optimise"
    );
    assert_eq!(
        texts(&clean_args(0, 7, Optimise::Run, None, Confirm::Skip)).join(" "),
        "clean all --keep 1 --keep-since 7d --optimise"
    );
    assert!(
        clean_args(
            5,
            3,
            Optimise::Skip,
            Some(Path::new("/run/wrappers/bin/pkexec")),
            Confirm::Ask
        )
        .ends_with(&[arg("--elevation-strategy"), arg("/run/wrappers/bin/pkexec")])
    );
    let usage = usage();
    assert!(usage.total >= usage.free);
}

#[test]
fn tried_links_are_removed_and_counted() {
    let dir = tempfile::tempdir().unwrap();
    let tried = dir.path().join("try");
    std::fs::create_dir_all(tried.join("folder")).unwrap();
    for name in ["hello", "xterm"] {
        std::os::unix::fs::symlink(format!("/nix/store/abc-{name}"), tried.join(name)).unwrap();
    }

    assert_eq!(links_in(&tried).map(Iterator::count), Some(3));
    assert_eq!(remove_links(&tried), 2, "a folder is not a link");
    assert!(tried.join("folder").is_dir());
    std::fs::symlink_metadata(tried.join("hello")).unwrap_err();
    assert_eq!(remove_links(&dir.path().join("missing")), 0);
}
