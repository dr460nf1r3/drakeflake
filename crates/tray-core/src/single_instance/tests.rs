use super::*;

#[test]
fn a_second_tray_finds_the_lock_taken() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("run/user/drakeflake-tray.lock");

    let first = claim(&path);
    assert!(first.is_ok(), "the first tray must get the lock");
    assert!(matches!(claim(&path), Err(NotStarted::AlreadyRunning)));

    drop(first);
    assert!(
        claim(&path).is_ok(),
        "the lock is free again once the first tray is gone"
    );
}

#[test]
fn an_unusable_lock_path_is_not_mistaken_for_a_running_tray() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("file");
    std::fs::write(&blocker, "").unwrap();

    assert!(matches!(
        claim(&blocker.join("drakeflake-tray.lock")),
        Err(NotStarted::NoLock(reason)) if reason.starts_with("cannot create")
    ));
    assert!(matches!(
        claim(dir.path()),
        Err(NotStarted::NoLock(reason)) if reason.starts_with("cannot open")
    ));
}
