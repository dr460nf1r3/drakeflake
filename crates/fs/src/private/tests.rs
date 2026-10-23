use std::os::unix::fs::PermissionsExt as _;

use super::*;

fn me() -> u32 {
    rustix::process::getuid().as_raw()
}

#[test]
fn the_folder_is_created_private_and_reused() {
    let temp = tempfile::tempdir().unwrap();
    let path = private_dir(temp.path(), me()).unwrap();

    assert_eq!(path, temp.path().join(format!("drakeflake-{}", me())));
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o700);
    assert_eq!(private_dir(temp.path(), me()), Ok(path));
}

#[test]
fn a_symlink_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("elsewhere");
    fs::create_dir_all(&target).unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
    std::os::unix::fs::symlink(&target, temp.path().join(format!("drakeflake-{}", me()))).unwrap();

    let err = private_dir(temp.path(), me()).unwrap_err();
    assert!(err.reason.contains("not a real folder"), "{err}");
}

#[test]
fn a_plain_file_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join(format!("drakeflake-{}", me())), "").unwrap();

    private_dir(temp.path(), me()).unwrap_err();
}

#[test]
fn a_folder_of_another_user_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let other = me().wrapping_add(1);
    fs::create_dir_all(temp.path().join(format!("drakeflake-{other}"))).unwrap();

    let err = private_dir(temp.path(), other).unwrap_err();
    assert!(err.reason.contains("another user"), "{err}");
}

#[test]
fn a_folder_open_to_the_group_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(format!("drakeflake-{}", me()));
    fs::create_dir_all(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).unwrap();

    private_dir(temp.path(), me()).unwrap_err();
}

#[test]
fn an_unwritable_temp_dir_is_an_error() {
    private_dir(Path::new("/nonexistent-drakeflake-temp"), me()).unwrap_err();
}
