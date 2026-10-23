use super::is_headless_qpa;

#[test]
fn headless_platforms_have_no_global_menu() {
    assert!(is_headless_qpa("offscreen"));
    assert!(is_headless_qpa("minimal"));
}

#[test]
fn display_platforms_may_have_a_global_menu() {
    for platform in ["", "wayland", "xcb", "cocoa"] {
        assert!(!is_headless_qpa(platform), "{platform:?} is not headless");
    }
}
