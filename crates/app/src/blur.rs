#[cfg(test)]
mod tests;
#[cfg(target_os = "linux")]
mod wayland;

#[cfg(target_os = "linux")]
use std::ffi::c_void;

use drakeflake_config::BlurMode;

#[cfg(target_os = "linux")]
type ApplyFn = extern "C" fn(*mut c_void, *mut c_void, bool) -> bool;
#[cfg(target_os = "linux")]
type ForgetFn = extern "C" fn(*mut c_void);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlurRequest {
    Off,
    Probe,
    Require,
}

impl BlurRequest {
    #[cfg(any(test, target_os = "linux"))]
    const fn enabled(self) -> bool {
        match self {
            Self::Off => false,
            Self::Probe | Self::Require => true,
        }
    }

    #[cfg(any(test, target_os = "linux"))]
    const fn silent(self) -> bool {
        match self {
            Self::Probe => true,
            Self::Off | Self::Require => false,
        }
    }
}

impl From<BlurMode> for BlurRequest {
    fn from(mode: BlurMode) -> Self {
        match mode {
            BlurMode::Auto => Self::Probe,
            BlurMode::On => Self::Require,
            BlurMode::Off => Self::Off,
        }
    }
}

#[cfg(target_os = "linux")]
#[expect(
    unsafe_code,
    reason = "declares the Qt functions of linux_blur.cpp (see the call sites)"
)]
unsafe extern "C" {
    fn drakeflake_window_alpha(apply: ApplyFn, forget: ForgetFn);
    fn drakeflake_blur_apply(enabled: bool, silent: bool) -> bool;
    fn drakeflake_blur_windows(enabled: bool, silent: bool);
}

#[cfg(target_os = "linux")]
extern "C" fn apply_surface(display: *mut c_void, surface: *mut c_void, enabled: bool) -> bool {
    wayland::apply(display, surface, enabled)
}

#[cfg(target_os = "linux")]
extern "C" fn forget_surface(surface: *mut c_void) {
    wayland::forget(surface);
}

/// Windows need an alpha channel before the application exists so the blur can show.
#[cfg(target_os = "linux")]
pub fn prepare() {
    #[expect(unsafe_code, reason = "one Qt call at startup, before threads exist")]
    // SAFETY: sets Qt's default surface format and stores fn pointers, all before threads exist.
    unsafe {
        drakeflake_window_alpha(apply_surface, forget_surface);
    }
}

#[cfg(not(target_os = "linux"))]
pub const fn prepare() {}

#[cfg(target_os = "linux")]
pub fn refresh(request: BlurRequest) -> bool {
    #[expect(unsafe_code, reason = "one Qt call with flags on the GUI thread")]
    // SAFETY: only touches the application's own windows, from the GUI thread.
    let active = unsafe { drakeflake_blur_apply(request.enabled(), request.silent()) };
    log::debug!("window blur {}", if active { "on" } else { "off" });
    active
}

#[cfg(not(target_os = "linux"))]
pub const fn refresh(_request: BlurRequest) -> bool {
    false
}

/// Queued so it runs once the event loop is up, a single shot is still too early on a slow start.
#[cfg(target_os = "linux")]
pub fn apply_startup(request: BlurRequest) {
    #[expect(unsafe_code, reason = "one Qt call with flags on the GUI thread")]
    // SAFETY: only queues a call on the application object, from the GUI thread.
    unsafe {
        drakeflake_blur_windows(request.enabled(), request.silent());
    }
}

#[cfg(not(target_os = "linux"))]
pub const fn apply_startup(_request: BlurRequest) {}
