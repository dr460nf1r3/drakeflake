use std::os::unix::fs::PermissionsExt as _;

use super::*;

fn env(temp: &Path) -> Env {
    Env {
        home: Some("/home/ada".into()),
        config_home: None,
        state_home: None,
        cache_home: None,
        runtime_dir: None,
        temp: temp.to_path_buf(),
        uid: rustix::process::getuid().as_raw(),
    }
}

#[test]
fn defaults_live_below_home() {
    let temp = tempfile::tempdir().unwrap();
    let env = env(temp.path());

    assert_eq!(
        env.app_dir(Kind::Config),
        Ok(PathBuf::from("/home/ada/.config/drakeflake"))
    );
    assert_eq!(
        env.app_dir(Kind::State),
        Ok(PathBuf::from("/home/ada/.local/state/drakeflake"))
    );
    assert_eq!(
        env.app_dir(Kind::Cache),
        Ok(PathBuf::from("/home/ada/.cache/drakeflake"))
    );
    assert_eq!(env.runtime(), env.app_dir(Kind::State));
}

#[test]
fn absolute_xdg_variables_win() {
    let temp = tempfile::tempdir().unwrap();

    let env = Env {
        config_home: Some("/xdg/config".into()),
        state_home: Some("/xdg/state".into()),
        cache_home: Some("/xdg/cache".into()),
        runtime_dir: Some("/run/user/7".into()),
        ..env(temp.path())
    };

    assert_eq!(
        env.app_dir(Kind::Config),
        Ok("/xdg/config/drakeflake".into())
    );
    assert_eq!(env.app_dir(Kind::State), Ok("/xdg/state/drakeflake".into()));
    assert_eq!(env.app_dir(Kind::Cache), Ok("/xdg/cache/drakeflake".into()));
    assert_eq!(env.runtime(), Ok("/run/user/7".into()));
}

#[test]
fn relative_or_empty_variables_are_ignored() {
    let temp = tempfile::tempdir().unwrap();

    let env = Env {
        config_home: Some("relative/config".into()),
        state_home: Some("".into()),
        runtime_dir: Some("run".into()),
        ..env(temp.path())
    };

    assert_eq!(
        env.app_dir(Kind::Config),
        Ok("/home/ada/.config/drakeflake".into())
    );
    assert_eq!(
        env.app_dir(Kind::State),
        Ok("/home/ada/.local/state/drakeflake".into())
    );
    assert_eq!(env.runtime(), env.app_dir(Kind::State));
}

#[test]
fn a_relative_or_empty_home_is_not_usable() {
    let temp = tempfile::tempdir().unwrap();

    for home in [Some(OsString::new()), Some("home/ada".into()), None] {
        let env = Env {
            home,
            ..env(temp.path())
        };

        assert_eq!(env.home(), None);
        assert_eq!(env.base(Kind::Config), None);
    }
}

#[test]
fn without_home_the_private_folder_is_used() {
    let temp = tempfile::tempdir().unwrap();

    let env = Env {
        home: None,
        ..env(temp.path())
    };

    let private = temp.path().join(format!("drakeflake-{}", env.uid));

    assert_eq!(env.app_dir(Kind::State), Ok(private.join("state")));
    assert_eq!(env.app_dir(Kind::Cache), Ok(private.join("cache")));
    assert_eq!(env.runtime(), Ok(private.join("state")));

    let mode = std::fs::metadata(&private).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o700);
}

#[test]
fn an_unsafe_private_folder_is_an_error_not_a_fallback() {
    let temp = tempfile::tempdir().unwrap();

    let env = Env {
        home: None,
        ..env(temp.path())
    };

    let private = temp.path().join(format!("drakeflake-{}", env.uid));
    std::fs::create_dir_all(&private).unwrap();
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o755)).unwrap();

    let err = env.app_dir(Kind::State).unwrap_err();
    assert_eq!(err.path, private);
    assert!(err.to_string().contains("other users"), "{err}");
}
