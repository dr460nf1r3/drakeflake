use super::*;

#[test]
fn blur_mode_parses_and_resolves() {
    assert_eq!("auto".parse(), Ok(BlurMode::Auto));
    assert_eq!("ON".parse(), Ok(BlurMode::On));
    assert_eq!(" 0 ".parse(), Ok(BlurMode::Off));
    assert_eq!("sometimes".parse::<BlurMode>(), Err(UnknownBlurMode));
    assert_eq!(BlurMode::Auto.as_str(), "auto");
    assert_eq!(BlurMode::default(), BlurMode::Auto);

    assert!(BlurMode::On.resolve(false));
    assert!(!BlurMode::Off.resolve(true));
    assert!(BlurMode::Auto.resolve(true));
    assert!(!BlurMode::Auto.resolve(false));
}

#[test]
fn blur_mode_from_str_ignores_case_and_spaces() {
    assert_eq!(" AUTO ".parse::<BlurMode>(), Ok(BlurMode::Auto));
    assert_eq!("True".parse::<BlurMode>(), Ok(BlurMode::On));
    assert_eq!("No".parse::<BlurMode>(), Ok(BlurMode::Off));
    assert_eq!("".parse::<BlurMode>(), Err(UnknownBlurMode));
    assert_eq!("onn".parse::<BlurMode>(), Err(UnknownBlurMode));
}

#[test]
fn only_a_known_override_replaces_the_saved_choice() {
    assert_eq!(override_from(None), Ok(None));
    assert_eq!(override_from(Some("  ")), Ok(None), "empty means unset");
    assert_eq!(override_from(Some("off")), Ok(Some(BlurMode::Off)));
    assert_eq!(override_from(Some(" Yes ")), Ok(Some(BlurMode::On)));
    assert_eq!(override_from(Some("blurry")), Err("blurry"));
}

#[test]
fn wayland_session_follows_the_qt_platform() {
    assert!(wayland_session_from(None, true, None));
    assert!(wayland_session_from(None, false, Some("Wayland")));
    assert!(!wayland_session_from(None, false, Some("x11")));
    assert!(!wayland_session_from(None, false, None));
    assert!(wayland_session_from(Some("wayland;xcb"), false, None));
    assert!(wayland_session_from(Some("wayland-egl"), false, None));
    assert!(!wayland_session_from(Some("xcb"), true, Some("wayland")));
    assert!(!wayland_session_from(Some("offscreen"), true, None));
    assert!(wayland_session_from(Some(" "), true, None));
}
