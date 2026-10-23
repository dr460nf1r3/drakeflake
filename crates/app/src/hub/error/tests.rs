use super::*;

#[test]
fn the_messages_are_the_ones_the_slots_always_returned() {
    assert_eq!(
        HubError::Busy.to_string(),
        "Another operation is still running"
    );
    assert_eq!(HubError::NoTarget.to_string(), "No system is selected yet");
    assert_eq!(HubError::invalid("Bad name").to_string(), "Bad name");
}

#[test]
fn a_missing_local_flake_keeps_its_own_variant() {
    assert!(matches!(
        HubError::from(FileError::NoLocalFlake),
        HubError::NoLocalFlake
    ));
    assert!(matches!(
        HubError::from(FileError::OutputNotNix("x".to_owned())),
        HubError::Library(_)
    ));
}
