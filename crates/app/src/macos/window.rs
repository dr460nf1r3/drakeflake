#[cfg(target_os = "macos")]
#[expect(
    unsafe_code,
    reason = "declares the Qt functions of macos.mm (see the call sites)"
)]
unsafe extern "C" {
    fn drakeflake_blur_windows();
    fn drakeflake_has_glass() -> bool;
    fn drakeflake_set_window_dirty(dirty: bool);
    fn drakeflake_set_dock_badge(count: i32);
    fn drakeflake_request_attention();
}

#[cfg(target_os = "macos")]
pub fn blur_windows() {
    #[expect(unsafe_code, reason = "Qt calls without arguments on the GUI thread")]
    // SAFETY: only probes a class and queues a call on the application
    // object, from the GUI thread.
    let glass = unsafe { drakeflake_has_glass() };
    log::debug!(
        "system blur through the {}",
        if glass {
            "glass material"
        } else {
            "sidebar material"
        }
    );

    #[expect(
        unsafe_code,
        reason = "one Qt call without arguments on the GUI thread"
    )]
    // SAFETY: only queues a call on the application object, from the GUI thread.
    unsafe {
        drakeflake_blur_windows();
    }
}

#[cfg(not(target_os = "macos"))]
pub const fn blur_windows() {}

#[cfg(target_os = "macos")]
pub fn set_window_dirty(dirty: bool) {
    #[expect(unsafe_code, reason = "one AppKit flag on the GUI thread")]
    // SAFETY: only sets a flag on the existing windows, from the GUI thread.
    unsafe {
        drakeflake_set_window_dirty(dirty);
    }
}

#[cfg(target_os = "macos")]
pub fn set_dock_badge(count: i32) {
    #[expect(unsafe_code, reason = "one AppKit label on the GUI thread")]
    // SAFETY: only sets the Dock tile's label, from the GUI thread.
    unsafe {
        drakeflake_set_dock_badge(count);
    }
}

#[cfg(target_os = "macos")]
pub fn dock_bounce() {
    #[expect(unsafe_code, reason = "one AppKit request on the GUI thread")]
    // SAFETY: only asks for attention, from the GUI thread.
    unsafe {
        drakeflake_request_attention();
    }
}

#[cfg(not(target_os = "macos"))]
#[expect(
    clippy::missing_const_for_fn,
    reason = "a const stub makes clippy ask for a const slot, which macOS cannot have"
)]
pub fn set_window_dirty(_dirty: bool) {}

#[cfg(not(target_os = "macos"))]
pub const fn set_dock_badge(_count: i32) {}

#[cfg(not(target_os = "macos"))]
pub const fn dock_bounce() {}
