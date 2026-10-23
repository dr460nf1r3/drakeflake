use std::path::{Path, PathBuf};

use ::notify::event::{AccessKind, CreateKind, Event, EventKind};

use super::{in_skipped_folder, is_change};

fn roots() -> [PathBuf; 2] {
    [
        PathBuf::from("/var/flake"),
        PathBuf::from("/private/var/flake"),
    ]
}

#[test]
fn hidden_folders_and_build_outputs_are_skipped_below_either_root() {
    for path in [
        "/var/flake/.git/index",
        "/private/var/flake/.direnv/bin/nix",
        "/var/flake/target/debug/build",
        "/var/flake/hosts/result/bin",
    ] {
        assert!(in_skipped_folder(Path::new(path), &roots()), "{path}");
    }
}

#[test]
fn files_of_the_flake_count_even_when_hidden() {
    for path in [
        "/var/flake/flake.nix",
        "/private/var/flake/modules/desktop.nix",
        "/var/flake/.envrc",
        "/elsewhere/.git/index",
    ] {
        assert!(!in_skipped_folder(Path::new(path), &roots()), "{path}");
    }
}

#[test]
fn reads_and_skipped_paths_are_no_change() {
    let created = EventKind::Create(CreateKind::File);
    let flake_file = Event::new(created).add_path(PathBuf::from("/var/flake/flake.nix"));
    let git_file = Event::new(created).add_path(PathBuf::from("/var/flake/.git/index"));
    let read = Event::new(EventKind::Access(AccessKind::Read))
        .add_path(PathBuf::from("/var/flake/flake.nix"));

    assert!(is_change(&flake_file, &roots()));
    assert!(!is_change(&git_file, &roots()));
    assert!(!is_change(&read, &roots()));
}
