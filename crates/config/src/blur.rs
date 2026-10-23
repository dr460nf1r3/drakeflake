use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

/// The QML getters call `blur_env_override` on every read, so warn once.
static WARNED_UNKNOWN_BLUR: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlurMode {
    #[default]
    Auto,
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownBlurMode;

impl BlurMode {
    #[must_use = "the name is only computed, nothing is stored"]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::On => "on",
            Self::Off => "off",
        }
    }

    #[must_use = "the decision has no other effect"]
    pub const fn resolve(self, wayland: bool) -> bool {
        match self {
            Self::Auto => wayland,
            Self::On => true,
            Self::Off => false,
        }
    }
}

impl FromStr for BlurMode {
    type Err = UnknownBlurMode;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let name = text.trim();
        let is_any = |names: &[&str]| names.iter().any(|known| name.eq_ignore_ascii_case(known));

        if is_any(&["auto"]) {
            Ok(Self::Auto)
        } else if is_any(&["on", "true", "yes", "1"]) {
            Ok(Self::On)
        } else if is_any(&["off", "false", "no", "0"]) {
            Ok(Self::Off)
        } else {
            Err(UnknownBlurMode)
        }
    }
}

impl fmt::Display for UnknownBlurMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown blur mode")
    }
}

impl std::error::Error for UnknownBlurMode {}

#[must_use = "the override is lost otherwise"]
pub fn blur_env_override() -> Option<BlurMode> {
    let value = std::env::var("DRAKEFLAKE_BLUR").ok();

    override_from(value.as_deref()).unwrap_or_else(|unknown| {
        if !WARNED_UNKNOWN_BLUR.swap(true, Ordering::Relaxed) {
            log::warn!("ignoring unknown DRAKEFLAKE_BLUR={unknown:?}");
        }

        None
    })
}

fn override_from(value: Option<&str>) -> Result<Option<BlurMode>, &str> {
    match value {
        None => Ok(None),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => value.parse().map(Some).map_err(|UnknownBlurMode| value),
    }
}

/// A hint only, because the request itself probes.
#[must_use = "the decision has no other effect"]
pub fn effective_blur(saved: BlurMode) -> bool {
    blur_env_override()
        .unwrap_or(saved)
        .resolve(wayland_session())
}

#[must_use = "reading the session has no other effect"]
pub fn wayland_session() -> bool {
    wayland_session_from(
        std::env::var("QT_QPA_PLATFORM").ok().as_deref(),
        std::env::var_os("WAYLAND_DISPLAY").is_some(),
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
    )
}

/// An explicit Qt platform wins since xcb or offscreen draw no Wayland surface.
fn wayland_session_from(platform: Option<&str>, display: bool, session: Option<&str>) -> bool {
    platform
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map_or_else(
            || display || session.is_some_and(|kind| kind.eq_ignore_ascii_case("wayland")),
            |names| {
                names
                    .split(';')
                    .next()
                    .is_some_and(|first| first.trim().starts_with("wayland"))
            },
        )
}

#[cfg(test)]
mod tests;
