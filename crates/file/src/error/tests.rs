use super::*;

#[test]
fn messages_keep_their_wording() {
    let refused = FileError::Refused(Refusal::NotNixFile.in_file(std::path::Path::new("/x.txt")));

    assert_eq!(
        refused.to_string(),
        "cannot write the settings: /x.txt: the target must be a .nix file"
    );
    assert_eq!(
        FileError::HostScopeNotAllowed("networking.hostName".into()).to_string(),
        "networking.hostName cannot be limited to one host"
    );
    assert_eq!(
        FileError::Option {
            name: "a.b".into(),
            source: Box::new(FileError::EmptyHost),
        }
        .to_string(),
        "a.b: the host name is empty"
    );
}

#[test]
fn only_permission_errors_are_worth_elevating_for() {
    let denied = || io::Error::from(ErrorKind::PermissionDenied);
    let path = std::path::Path::new("/etc/x.nix");

    assert!(FileError::Write(denied()).is_permission_denied());
    assert!(!FileError::Write(io::Error::from(ErrorKind::NotFound)).is_permission_denied());
    let unreadable = Refusal::Io {
        action: "read",
        path: path.to_path_buf(),
        source: denied(),
    };

    assert!(FileError::Refused(unreadable.in_file(path)).is_permission_denied());
    assert!(!FileError::Refused(Refusal::NotModule.in_file(path)).is_permission_denied());
}

#[test]
fn a_cancelled_run_stays_recognisable() {
    assert!(FileError::from(ExecError::Cancelled(None)).is_cancelled());
    assert!(!FileError::EmptyHost.is_cancelled());
}
