use std::path::Path;

use crate::paths::is_inside;

#[test]
fn missing_folders_count_by_their_parents() {
    let flake = tempfile::tempdir().unwrap();
    let missing = flake.path().join("modules").join("default.nix");

    assert!(is_inside(flake.path(), &missing));
    assert!(!is_inside(&flake.path().join("elsewhere"), &missing));
    assert!(!is_inside(Path::new("/nonexistent-dir-xyz"), &missing));
}

#[cfg(unix)]
#[test]
fn symlinked_parents_resolve_before_comparing() {
    let flake = tempfile::tempdir().unwrap();
    let link = flake.path().join("link");
    std::os::unix::fs::symlink(flake.path(), &link).unwrap();

    // Without resolving the existing parents first, the raw link path
    // never matches the canonical flake dir (macOS /tmp does exactly this).
    assert!(is_inside(
        flake.path(),
        &link.join("modules").join("default.nix")
    ));
    assert!(is_inside(&link, &link.join("modules").join("default.nix")));
    assert!(!is_inside(
        &link.join("elsewhere"),
        &link.join("modules").join("default.nix")
    ));
}
