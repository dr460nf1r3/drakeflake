use std::path::PathBuf;

use drakeflake_config::Tray;

use super::{TrayEdit, autostart_file_in, edit_tray, set_autostart, wants_start};

#[test]
#[cfg(target_os = "linux")]
fn the_autostart_entry_goes_to_the_xdg_config_folder() {
    let home = Some(PathBuf::from("/home/me"));

    assert_eq!(
        autostart_file_in(home.clone(), Some(PathBuf::from("/cfg"))),
        Some(PathBuf::from("/cfg/autostart/drakeflake-tray.desktop"))
    );
    assert_eq!(autostart_file_in(home, None), None);
}

#[test]
#[cfg(target_os = "macos")]
fn the_launch_agent_goes_to_the_library_folder() {
    let home = Some(PathBuf::from("/Users/me"));

    assert_eq!(
        autostart_file_in(home, Some(PathBuf::from("/cfg"))),
        Some(PathBuf::from(
            "/Users/me/Library/LaunchAgents/org.dr460nf1r3.drakeflake.tray.plist"
        ))
    );
    assert_eq!(autostart_file_in(None, Some(PathBuf::from("/cfg"))), None);
}

#[test]
fn autostart_is_written_and_removed() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("autostart/drakeflake-tray.desktop");

    set_autostart(&file, true).unwrap();
    let entry = std::fs::read_to_string(&file).unwrap();
    assert!(entry.contains("drakeflake-tray"), "{entry}");

    set_autostart(&file, false).unwrap();
    assert!(!file.exists());

    set_autostart(&file, false).unwrap();
}

#[test]
fn autostart_failures_name_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("autostart");
    std::fs::write(&blocker, "a file, not a folder").unwrap();
    let file = blocker.join("drakeflake-tray.desktop");

    let error = set_autostart(&file, true).unwrap_err();
    assert!(
        error.starts_with(&format!("Cannot change {}: ", file.display())),
        "{error}"
    );
}

#[test]
fn only_a_real_change_is_saved() {
    let current = Tray::default();
    let on = current.updates;

    assert_eq!(
        edit_tray(current, |tray| tray.set("updates", on)),
        TrayEdit::Unchanged
    );
    assert_eq!(
        edit_tray(current, |tray| tray.set("nope", true)),
        TrayEdit::Refused
    );
    assert_eq!(
        edit_tray(current, |tray| tray.set_interval("check_hours", 0)),
        TrayEdit::Refused
    );

    let flipped = edit_tray(current, |tray| tray.set("updates", !on));
    assert!(matches!(flipped, TrayEdit::Changed(tray) if tray.updates != on));
}

#[test]
fn the_app_starts_the_tray_only_when_it_is_wanted_and_missing() {
    assert!(wants_start(true, false));
    assert!(!wants_start(true, true));
    assert!(!wants_start(false, false));
}
