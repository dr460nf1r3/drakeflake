use super::*;

fn size_of(path: &Path) -> u64 {
    fs::metadata(path).map_or(0, |meta| meta.len())
}

#[test]
fn a_full_file_is_rotated_before_the_next_line() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state/drakeflake/app.log");
    let rotated = dir.path().join("state/drakeflake/app.log.1");
    fs::create_dir_all(path.parent().unwrap()).unwrap();

    let almost_full = usize::try_from(MAX_SIZE - 1).unwrap();
    fs::write(&path, "x".repeat(almost_full)).unwrap();

    let logger = FileLogger::open(path.clone());
    logger.write_line("first\n");
    assert!(
        !rotated.exists(),
        "rotation waits until the limit is reached"
    );
    assert_eq!(size_of(&path), MAX_SIZE + 5);

    logger.write_line("second\n");
    assert_eq!(size_of(&rotated), MAX_SIZE + 5);
    assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");
}

#[test]
fn a_failed_rotation_keeps_appending_and_retries_later() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.log");

    for name in ["app.log.1", "app.log.2", "app.log.3"] {
        fs::create_dir_all(dir.path().join(name).join("keep")).unwrap();
    }

    let logger = FileLogger::open_with_limit(path.clone(), 10);
    logger.write_line("0123456789\n");
    logger.write_line("second\n");
    logger.write_line("third\n");

    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "0123456789\nsecond\nthird\n"
    );
    assert!(dir.path().join("app.log.1").is_dir());

    fs::remove_dir_all(dir.path().join("app.log.1")).unwrap();
    logger.write_line(&"x".repeat(usize::try_from(RETRY_BYTES).unwrap()));
    logger.write_line("after\n");

    assert!(dir.path().join("app.log.1").is_file());
    assert_eq!(fs::read_to_string(&path).unwrap(), "after\n");
}

#[test]
fn the_log_is_private_to_the_user() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state/drakeflake/app.log");
    FileLogger::open(path.clone()).write_line("line\n");

    let mode = |file: &Path| fs::metadata(file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(path.parent().unwrap()), 0o700);
}

#[test]
fn an_existing_directory_keeps_its_mode() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
    FileLogger::open(dir.path().join("app.log")).write_line("line\n");

    let mode = fs::metadata(dir.path()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o755);
}

#[test]
fn lines_are_appended_to_an_existing_log() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new/folder/app.log");

    FileLogger::open(path.clone()).write_line("one\n");
    FileLogger::open(path.clone()).write_line("two\n");

    assert_eq!(fs::read_to_string(&path).unwrap(), "one\ntwo\n");
}

#[test]
fn an_unwritable_log_drops_lines_without_failing() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("file");
    fs::write(&blocker, "").unwrap();

    let logger = FileLogger::open(blocker.join("app.log"));
    assert!(logger.failure().is_some());
    logger.write_line("lost\n");
    log::Log::flush(&logger);

    assert_eq!(fs::read_to_string(&blocker).unwrap(), "");
}

#[test]
fn a_logger_without_a_folder_reports_why_and_drops_lines() {
    let logger = FileLogger::unavailable("no safe place".to_owned());

    assert_eq!(logger.failure(), Some("no safe place"));
    logger.write_line("lost\n");
    log::Log::flush(&logger);
}
