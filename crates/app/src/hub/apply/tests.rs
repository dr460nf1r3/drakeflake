use drakeflake_config::{LayoutConfig, LayoutMode};
use drakeflake_plan::{HEADER, WritePlan};

use super::super::test_support::{console_lines, hub_with, local_target};
use super::*;

#[test]
fn nothing_renders_without_a_system() {
    assert_eq!(
        hub_with(None)
            .render_plan(&ManagedState::default())
            .map(|(path, _)| path),
        Err(vec!["No system is selected yet".to_owned()])
    );
}

#[test]
fn unchanged_files_are_neither_backed_up_nor_written() {
    let flake = tempfile::tempdir().unwrap();
    let hub = hub_with(Some(local_target(flake.path())));
    hub.set_layout(LayoutConfig {
        mode: LayoutMode::Single,
        ..LayoutConfig::default()
    });
    let mut state = ManagedState::default();
    state.packages.insert("hello".into());

    let (path, rendered) = hub.render_plan(&state).unwrap();
    for (file, text) in WritePlan::for_rendered(&path, &rendered, None, Path::new("")).write {
        fs::write(file, text).unwrap();
    }

    let run = Exclusive::detached(&hub);
    assert!(run.save_plan(&path, &rendered).is_ok());
    assert!(
        run.backup_for_test().is_none(),
        "an unchanged file was backed up"
    );
    assert_eq!(
        console_lines(&hub),
        vec![format!("{} is already up to date.", path.display())]
    );
}

#[test]
fn a_new_entry_removes_the_one_it_supersedes() {
    let dir = tempfile::tempdir().unwrap();
    let old_entry = dir.path().join(drakeflake_plan::FILE_NAME);
    fs::write(
        &old_entry,
        format!("{HEADER}# drakeflake-files: [\"services.nix\",\"boot.nix\"]\n{{ }}\n"),
    )
    .unwrap();
    let entry = dir.path().join(drakeflake_file::FOLDER_ENTRY);
    let services = dir.path().join("services.nix");
    let boot = dir.path().join("boot.nix");

    let hub = hub_with(None);
    let run = Exclusive::detached(&hub);
    let mut plan = WritePlan::default();
    plan.write.insert(entry.clone(), String::new());
    plan.write.insert(boot, String::new());
    plan.remove.push(services.clone());
    run.drop_superseded_entry(&entry, &mut plan);

    assert_eq!(plan.remove, vec![services, old_entry]);
    let lines = console_lines(&hub);
    assert_eq!(lines.len(), 1);
    assert!(
        lines
            .iter()
            .all(|line| line.ends_with(&format!("Change your import to {}.", dir.path().display()))),
        "the folder is imported, not its default.nix: {lines:?}"
    );

    let empty = tempfile::tempdir().unwrap();
    let mut untouched = WritePlan::default();
    run.drop_superseded_entry(
        &empty.path().join(drakeflake_file::FOLDER_ENTRY),
        &mut untouched,
    );
    assert_eq!(untouched, WritePlan::default());
    assert_eq!(console_lines(&hub), Vec::<String>::new());
}

#[test]
fn unusable_backups_are_not_restored() {
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, text: &str| {
        let path = dir.path().join(name);
        fs::write(&path, text).unwrap();
        path
    };

    let cases = [
        (dir.path().join("absent.bak"), "Cannot read"),
        (write("plain.bak", "{ }\n"), "holds no settings"),
        (
            write(
                "corrupt.bak",
                &format!("{HEADER}# drakeflake-state: {{oops\n{{ }}\n"),
            ),
            "corrupt state line",
        ),
        (
            write("valid.bak", &ManagedState::default().render().unwrap()),
            "Invalid value in the backup: No system is selected yet",
        ),
    ];

    for (backup, error) in cases {
        let hub = hub_with(None);
        assert_eq!(
            Exclusive::detached(&hub).run_restore(&backup),
            Outcome::Failed,
            "{}",
            backup.display()
        );
        let lines = console_lines(&hub);
        assert!(
            lines.iter().any(|line| line.contains(error)),
            "{}: {lines:?}",
            backup.display()
        );
        assert_eq!(hub.mail.changes.take().len(), 0, "a failed restore reloads");
    }
}
