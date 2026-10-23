//! Bare `u64`s mix at every boundary (a console session passed where a load
//! generation belongs compiles).

use std::fmt;

macro_rules! counter {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
        pub struct $name(pub u64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

counter!(
    Generation,
    "One load of one target: stale results carry an older one."
);
counter!(
    HomeEpoch,
    "Home Manager user selections: answers for a previous user are dropped."
);
counter!(
    ConsoleSession,
    "One program run's block of console rows: redraws of an older run are dropped."
);

impl Generation {
    #[must_use = "the next counter value is the only result"]
    pub const fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

impl HomeEpoch {
    #[must_use = "the next counter value is the only result"]
    pub const fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}
