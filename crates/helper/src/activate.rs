use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::error::HelperError;
use crate::exec::{resolve, run};
use crate::{SYSTEM_PROFILE, note};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Switch,
    Boot,
    Test,
}

impl Mode {
    fn parse(mode: &str) -> Result<Self, HelperError> {
        match mode {
            "switch" => Ok(Self::Switch),
            "boot" => Ok(Self::Boot),
            "test" => Ok(Self::Test),
            _ => Err(HelperError::UnknownMode(mode.to_owned())),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Switch => "switch",
            Self::Boot => "boot",
            Self::Test => "test",
        }
    }
}

/// Resolved first so only immutable store contents run as root, never a script the caller can change.
pub fn checked_system(system: &Path) -> Result<PathBuf, HelperError> {
    let resolved = resolve(system)?;

    if !resolved.starts_with("/nix/store/") {
        return Err(HelperError::NotInStore(resolved));
    }

    if !resolved.join("bin/switch-to-configuration").is_file()
        || !resolved.join("nixos-version").is_file()
    {
        return Err(HelperError::NotASystem(resolved));
    }

    Ok(resolved)
}

pub fn activate(mode: &str, system: &Path) -> Result<(), HelperError> {
    let mode = Mode::parse(mode)?;
    let system = checked_system(system)?;

    if mode != Mode::Test {
        run(
            "nix-env",
            [
                OsStr::new("-p"),
                OsStr::new(SYSTEM_PROFILE),
                OsStr::new("--set"),
                system.as_os_str(),
            ],
        )?;
    }

    let switch = system.join("bin/switch-to-configuration");

    // A transient unit survives the app and this helper being closed mid-switch.
    let in_unit = run(
        "systemd-run",
        [
            OsStr::new("--collect"),
            OsStr::new("--no-ask-password"),
            OsStr::new("--pipe"),
            OsStr::new("--quiet"),
            OsStr::new("--wait"),
            OsStr::new("--service-type=exec"),
            OsStr::new("--unit=drakeflake-switch-to-configuration"),
            OsStr::new("-E"),
            OsStr::new("LOCALE_ARCHIVE"),
            OsStr::new("-E"),
            OsStr::new("NIXOS_INSTALL_BOOTLOADER"),
            switch.as_os_str(),
            OsStr::new(mode.as_str()),
        ],
    );

    match in_unit {
        // Only a missing systemd-run falls back. A failed switch is final.
        Err(
            error @ HelperError::Spawn {
                program: _,
                source: _,
            },
        ) => {
            note(&format!(
                "{error}; running switch-to-configuration directly"
            ));
            run(&switch, [mode.as_str()])
        }
        other => other,
    }
}
