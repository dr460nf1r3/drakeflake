use drakeflake_plan::{HEADER, ManagedOption};
use drakeflake_render::{OptionName, PackageAttr};

use super::*;

fn state(options: &[(&str, &str)], packages: &[&str]) -> ManagedState {
    ManagedState {
        options: options
            .iter()
            .map(|(name, input)| {
                (
                    OptionName::from(*name),
                    ManagedOption {
                        type_name: "string".to_owned(),
                        input: (*input).to_owned(),
                        ..ManagedOption::default()
                    },
                )
            })
            .collect(),
        packages: packages
            .iter()
            .map(|attr| PackageAttr::from(*attr))
            .collect(),
    }
}

fn backup(dir: &Path, name: &str, state: &ManagedState) {
    let json = serde_json::to_string(state).unwrap();
    std::fs::write(
        dir.join(name),
        format!("{HEADER}# drakeflake-state: {json}\n{{ }}\n"),
    )
    .unwrap();
}

#[test]
fn differences_read_as_what_restoring_does() {
    let current = state(&[("a", "1"), ("b", "2")], &["hello"]);
    let earlier = state(&[("a", "0"), ("c", "3")], &["git"]);

    assert_eq!(
        differences(&current, &earlier),
        vec![
            "a: 1 → 0",
            "c = 3",
            "b: no longer set",
            "+ package git",
            "− package hello"
        ]
    );
    assert_eq!(differences(&current, &current), Vec::<String>::new());
}

#[test]
fn backup_names_carry_the_save_time() {
    for (name, seconds) in [
        ("/x/drakeflake.nix.1790822859.3.bak", 1_790_822_859),
        ("/x/default.nix.1790822859.12.bak", 1_790_822_859),
        ("/x/drakeflake.nix.12345.3.bak", 0),
        ("/x/drakeflake.nix.17908x2859.3.bak", 0),
        ("/x/other", 0),
    ] {
        assert_eq!(saved_at(Path::new(name)), seconds, "{name}");
    }
}

#[test]
fn backups_are_listed_newest_first_and_unreadable_ones_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let entry = Path::new("/etc/nixos/drakeflake.nix");
    let current = state(&[("a", "2")], &["git"]);

    backup(
        dir.path(),
        "drakeflake.nix.1790000000.1.bak",
        &state(&[("a", "1")], &[]),
    );
    backup(
        dir.path(),
        "drakeflake.nix.1790000100.2.bak",
        &state(&[("a", "2"), ("b", "x")], &["git", "vim"]),
    );

    backup(dir.path(), "notes.txt", &current);
    std::fs::write(dir.path().join("plain.1790000200.3.bak"), "{ }\n").unwrap();
    std::fs::write(
        dir.path().join("corrupt.1790000300.4.bak"),
        format!("{HEADER}# drakeflake-state: {{\"options\":\n"),
    )
    .unwrap();

    let entries = list_backups(dir.path(), entry, &current);

    assert_eq!(
        entries.iter().map(|found| found.saved).collect::<Vec<_>>(),
        vec![1_790_000_100, 1_790_000_000]
    );
    assert_eq!((entries[0].options, entries[0].packages), (2, 2));
    assert_eq!(entries[0].changes, vec!["b = x", "+ package vim"]);
    assert_eq!(entries[1].changes, vec!["a: 2 → 1", "− package git"]);
    assert!(entries[0].file.ends_with("drakeflake.nix.1790000100.2.bak"));
}

#[test]
fn no_backup_folder_means_no_history() {
    let dir = tempfile::tempdir().unwrap();

    let entries = list_backups(
        &dir.path().join("missing"),
        Path::new("/etc/nixos/drakeflake.nix"),
        &ManagedState::default(),
    );

    assert_eq!(
        entries.iter().map(|found| found.saved).collect::<Vec<_>>(),
        Vec::<u64>::new()
    );
}

#[test]
fn each_generated_file_has_its_own_history() {
    assert_ne!(
        history_dir(Path::new("/etc/nixos/drakeflake.nix")).unwrap(),
        history_dir(Path::new("/etc/nixos/drakeflake/default.nix")).unwrap()
    );
}
