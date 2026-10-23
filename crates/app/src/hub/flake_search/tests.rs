use std::fs;

use drakeflake_exec::Context;

use super::*;

fn flake() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    fs::create_dir_all(path.join("hosts")).unwrap();
    fs::write(
        path.join("flake.nix"),
        "{ outputs = { nixpkgs, ... }: { }; }\n",
    )
    .unwrap();
    fs::write(
        path.join("hosts/laptop.nix"),
        "{ pkgs, ... }:\n{\n  environment.systemPackages = [ pkgs.openssh ];\n  services.openssh.enable = true;\n}\n",
    )
    .unwrap();
    dir
}

#[test]
fn attributes_come_before_values() {
    let dir = flake();
    let (total, hits) = search_dir(&Context::system(), dir.path(), "openssh", || true).unwrap();

    assert_eq!(total, 2);
    assert_eq!(hits[0].kind, FlakeHitKind::Attribute);
    assert_eq!(hits[0].path, "services.openssh.enable");
    assert_eq!(hits[0].display, "hosts/laptop.nix");
    assert_eq!(hits[0].line, 4);
    assert_eq!(hits[1].kind, FlakeHitKind::Value);
    assert_eq!(hits[1].snippet, "pkgs.openssh");
    assert!(
        hits[1].file.ends_with("hosts/laptop.nix"),
        "{}",
        hits[1].file
    );
}

#[test]
fn nothing_matches() {
    let dir = flake();

    assert_eq!(
        search_dir(&Context::system(), dir.path(), "no-such-thing", || true),
        Some((0, Vec::new()))
    );
}

#[test]
fn a_newer_query_stops_the_search() {
    let dir = flake();

    assert_eq!(
        search_dir(&Context::system(), dir.path(), "openssh", || false),
        None
    );
}

#[test]
fn large_files_are_skipped() {
    let dir = flake();
    let big = format!("{{ openssh = \"{}\"; }}", "x".repeat(1024 * 1024 + 1));
    fs::write(dir.path().join("big.nix"), big).unwrap();

    let (total, _) = search_dir(&Context::system(), dir.path(), "openssh", || true).unwrap();

    assert_eq!(total, 2);
}

#[test]
fn hit_kinds_serialise_as_the_words_qml_compares() {
    assert_eq!(
        serde_json::to_value(FlakeHitKind::Attribute).unwrap(),
        "attribute"
    );
    assert_eq!(serde_json::to_value(FlakeHitKind::Value).unwrap(), "value");
    assert!(FlakeHitKind::Attribute < FlakeHitKind::Value);
}
