//! Real-git integration tests, ignored because the offline flake check has no git or network.

#[cfg(test)]
mod tests;

/// Keep the `TempDir` alive while the copy is used.
fn fixture_copy() -> Option<(std::path::PathBuf, tempfile::TempDir)> {
    let source = std::env::var("DRAKEFLAKE_TEST_FLAKE").ok()?;
    let parent = tempfile::TempDir::new().expect("temp dir for the fixture copy");
    let dir = parent.path().join("flake");

    let copy = std::process::Command::new("cp")
        .args(["-r", &source, &dir.to_string_lossy()])
        .status()
        .expect("cp -r the fixture");

    assert!(copy.success(), "cp -r {source} failed");

    let writable = std::process::Command::new("chmod")
        .args(["-R", "u+w", &dir.to_string_lossy()])
        .status()
        .expect("chmod the fixture copy");

    assert!(writable.success(), "chmod the fixture copy failed");

    Some((dir, parent))
}
