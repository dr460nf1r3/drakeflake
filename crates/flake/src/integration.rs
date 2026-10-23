use drakeflake_render::HostName;

use crate::{Nix, SystemKind, Target};

#[must_use = "the context is only built"]
fn nix() -> Nix {
    Nix::system()
}

#[cfg(test)]
mod tests;

#[must_use = "the target is only read, nothing runs yet"]
fn target() -> Option<Target> {
    Some(Target {
        flake: std::env::var("DRAKEFLAKE_TEST_FLAKE").ok()?,
        host: std::env::var("DRAKEFLAKE_TEST_HOST")
            .ok()
            .map(HostName::from)?,
        kind: SystemKind::NixOs,
    })
}

fn copied_target() -> Option<(Target, tempfile::TempDir)> {
    let source = std::env::var("DRAKEFLAKE_TEST_FLAKE").ok()?;
    let host = std::env::var("DRAKEFLAKE_TEST_HOST").ok()?;
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

    Some((
        Target {
            flake: dir.to_string_lossy().into_owned(),
            host: host.into(),
            kind: SystemKind::NixOs,
        },
        parent,
    ))
}
