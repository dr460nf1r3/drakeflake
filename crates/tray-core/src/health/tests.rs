use std::cell::RefCell;

use super::*;

#[test]
fn restart_needed_when_the_kernel_changed() {
    let root = tempfile::tempdir().unwrap();
    let system = |name: &str, kernel: &str| {
        let dir = root.path().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let target = root.path().join(kernel);
        std::fs::write(&target, kernel).unwrap();
        std::os::unix::fs::symlink(&target, dir.join("kernel")).unwrap();
        dir
    };

    let booted = system("booted", "kernel-a");
    assert!(!restart_needed(&booted, &system("same", "kernel-a")));
    assert!(restart_needed(&booted, &system("newer", "kernel-b")));

    assert!(!restart_needed(&root.path().join("none"), &booted));
}

#[test]
fn staged_edits_are_counted_over_all_systems() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.json"),
        r#"{"options":{"x":{},"y":{}},"packages":{"hello":{}}}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("b.json"), r#"{"options":{"z":{}}}"#).unwrap();
    std::fs::write(dir.path().join("broken.json"), "{").unwrap();
    std::fs::write(dir.path().join("notes.txt"), r#"{"options":{"n":{}}}"#).unwrap();

    assert_eq!(staged_edits(dir.path()), 4);
    assert_eq!(staged_edits(&dir.path().join("missing")), 0);
}

#[test]
fn failed_units_are_parsed() {
    let output = "nginx.service loaded failed failed nginx\n● backup.service loaded failed failed Backup\n\n";
    assert_eq!(failed_units_in(output), ["nginx.service", "backup.service"]);
    assert_eq!(failed_units_in(""), Vec::<String>::new());
}

#[test]
fn space_is_low_below_five_gib_or_a_tenth() {
    assert!(is_low(4 * GIB, 1000 * GIB));
    assert!(is_low(8 * GIB, 100 * GIB));
    assert!(!is_low(20 * GIB, 100 * GIB));
    assert_eq!(gib(3 * GIB + GIB / 4), "3.2 GiB");
}

#[test]
fn notes_and_notifications_for_new_findings() {
    let healthy = Health::default();
    let ill = Health {
        restart_needed: true,
        staged: 2,
        failed_units: vec!["nginx.service".to_owned()],
        low_space: Some(3 * GIB),
    };

    assert!(!healthy.needs_attention());
    assert!(
        !Health {
            staged: 3,
            ..Health::default()
        }
        .needs_attention()
    );
    assert!(ill.needs_attention());
    assert_eq!(
        ill.notes(),
        [
            "Restart to finish the update (new kernel)",
            "Failed services: nginx.service",
            "Low disk space: 3.0 GiB free for Nix",
            "2 changes staged in DrakeFlake, not applied",
        ]
    );

    let shown = RefCell::new(Vec::new());
    let notify = |summary: &str, _: &str| shown.borrow_mut().push(summary.to_owned());
    report_health(&healthy, &ill, notify);

    report_health(&ill, &ill, notify);
    assert_eq!(
        *shown.borrow(),
        ["Restart needed", "Services failed", "Low disk space"]
    );
}

#[test]
fn only_newly_failed_services_are_named() {
    let before = Health {
        failed_units: vec!["nginx.service".to_owned()],
        low_space: Some(GIB),
        ..Health::default()
    };

    let now = Health {
        failed_units: vec!["nginx.service".to_owned(), "backup.service".to_owned()],
        low_space: Some(GIB / 2),
        ..Health::default()
    };

    let shown = RefCell::new(Vec::new());
    report_health(&before, &now, |summary: &str, body: &str| {
        shown.borrow_mut().push(format!("{summary}: {body}"));
    });

    assert_eq!(
        *shown.borrow(),
        ["Services failed: backup.service failed. Check `systemctl status` for the reason."]
    );
}

#[test]
fn reports_that_are_turned_off_read_nothing() {
    let off = drakeflake_config::Tray {
        restart: false,
        failed_services: false,
        low_space: false,
        staged: false,
        ..drakeflake_config::Tray::default()
    };

    assert_eq!(Health::read(off), Health::default());
}
