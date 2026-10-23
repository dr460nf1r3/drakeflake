use super::*;

const SYSTEM_FLAKE: &str = "{ outputs = _: { darwinConfigurations = { }; }; }";

fn below(home: &Path, found: &[PathBuf]) -> Vec<String> {
    let home = home.canonicalize().unwrap();
    found
        .iter()
        .filter_map(|dir| dir.strip_prefix(&home).ok())
        .map(|dir| dir.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_scan_stops_at_its_depth_and_skips_build_folders() {
    let home = tempfile::tempdir().unwrap();
    for dir in [
        "a/b/c",
        "a/b/c/d",
        "node_modules/pkg",
        "target",
        "code/result",
        ".config/sys",
        ".dotfiles/mac",
    ] {
        let dir = home.path().join(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("flake.nix"), SYSTEM_FLAKE).unwrap();
    }

    let mut found = below(home.path(), &find_system_flakes(Some(home.path())));
    found.sort();

    assert_eq!(found, [".config/sys", ".dotfiles/mac", "a/b/c"]);
}

#[test]
fn usual_folders_come_first_and_only_once() {
    let home = tempfile::tempdir().unwrap();
    for dir in ["aaa", "nixos", ".config/nixos"] {
        let dir = home.path().join(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("flake.nix"), SYSTEM_FLAKE).unwrap();
    }

    let found = below(home.path(), &find_system_flakes(Some(home.path())));

    assert_eq!(found, ["nixos", ".config/nixos", "aaa"]);
}

#[test]
fn system_flakes_are_found_below_home() {
    let home = tempfile::tempdir().unwrap();
    let system = home.path().join("code/infra");
    let other = home.path().join("code/website");
    let hidden = home.path().join(".secret/nixos");
    for dir in [&system, &other, &hidden] {
        std::fs::create_dir_all(dir).unwrap();
    }

    std::fs::write(
        system.join("flake.nix"),
        "{ outputs = _: { nixosConfigurations = { }; }; }",
    )
    .unwrap();
    std::fs::write(
        other.join("flake.nix"),
        "{ outputs = _: { packages = { }; }; }",
    )
    .unwrap();
    std::fs::write(
        hidden.join("flake.nix"),
        "{ outputs = _: { nixosConfigurations = { }; }; }",
    )
    .unwrap();

    let found = find_system_flakes(Some(home.path()));

    assert!(found.contains(&system.canonicalize().unwrap()));
    assert!(!found.iter().any(|dir| dir.ends_with("website")));
    assert!(!found.iter().any(|dir| dir.ends_with(".secret/nixos")));
}
