use drakeflake_flake::SystemKind;
use serde_json::{Value, json};

use super::Mode;
use super::session::Session;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub platform: SystemKind,
    pub native: bool,
    pub apply: &'static [Mode],
    pub review: &'static [Mode],
    pub rebuild: &'static [Mode],
    pub generations: bool,
    pub generation_modes: &'static [Mode],
    pub images: bool,
}

impl Capabilities {
    #[must_use = "only describes what the host allows"]
    pub const fn of(kind: SystemKind, native: SystemKind) -> Self {
        match (kind, native) {
            (SystemKind::NixOs, SystemKind::NixOs) => Self {
                platform: native,
                native: true,
                apply: &[
                    Mode::Save,
                    Mode::Build,
                    Mode::Test,
                    Mode::Switch,
                    Mode::Boot,
                ],
                review: &[Mode::Switch, Mode::Boot, Mode::Test],
                rebuild: &[Mode::Build, Mode::Test, Mode::Switch, Mode::Boot],
                generations: true,
                generation_modes: &[Mode::Switch, Mode::Test, Mode::Boot],
                images: true,
            },
            (SystemKind::Darwin, SystemKind::Darwin) => Self {
                platform: native,
                native: true,
                apply: &[Mode::Save, Mode::Build, Mode::Switch],
                review: &[Mode::Switch],
                rebuild: &[Mode::Build, Mode::Switch],
                generations: true,
                generation_modes: &[Mode::Switch],
                images: false,
            },
            (SystemKind::NixOs | SystemKind::Darwin, _) => Self {
                platform: native,
                native: false,
                apply: &[Mode::Save],
                review: &[],
                rebuild: &[],
                generations: false,
                generation_modes: &[],
                images: false,
            },
        }
    }

    #[must_use = "only describes what the host allows"]
    pub const fn here(kind: SystemKind) -> Self {
        Self::of(kind, SystemKind::native())
    }

    #[must_use = "only describes what the host allows"]
    pub fn selected(session: &Session) -> Self {
        Self::here(
            session
                .target()
                .map_or_else(SystemKind::native, |target| target.kind),
        )
    }

    #[must_use = "the JSON is only built for QML"]
    pub fn to_json(self) -> Value {
        let names = |modes: &[Mode]| modes.iter().map(|mode| mode.verb()).collect::<Vec<_>>();

        json!({
            "platform": self.platform.as_str(),
            "default_flake": self.platform.default_flake_path(),
            "native": self.native,
            "apply": names(self.apply),
            "review": names(self.review),
            "rebuild": names(self.rebuild),
            "generations": self.generations,
            "generation_modes": names(self.generation_modes),
            "images": self.images,
        })
    }
}

#[cfg(test)]
mod tests;
