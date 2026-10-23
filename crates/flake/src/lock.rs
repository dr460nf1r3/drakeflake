mod check;
mod kind;
mod parse;
mod plan;
mod remote;
mod urls;

pub use check::check_updates;
pub use kind::InputKind;
pub use parse::{parse_lock, read_lock};
pub use plan::UpdatePlan;
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct LockedInput {
    pub name: String,
    pub kind: InputKind,
    pub source: String,
    pub reference: Option<String>,
    pub rev: Option<String>,
    pub last_modified: Option<i64>,
    pub web_url: Option<String>,
    pub commit_url: Option<String>,
    #[serde(skip)]
    pub remote: Option<String>,
    #[serde(skip)]
    pub locked_url: Option<String>,
    #[serde(skip)]
    pub pin_base: Option<String>,
    pub pinned: bool,
    pub follows: Option<String>,
    pub nested: Vec<NestedInput>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct NestedInput {
    pub name: String,
    pub follows: Option<String>,
    pub follows_root: bool,
    pub source: Option<String>,
    pub rev: Option<String>,
    pub last_modified: Option<i64>,
    pub same_as: Option<String>,
    pub could_follow: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum UpdateCheck {
    UpToDate,
    Available {
        rev: String,
        compare_url: Option<String>,
        /// Locking it needs no rate-limited GitHub API call and no channel cache (`tarball-ttl`).
        #[serde(skip)]
        lock_url: Option<String>,
    },
    NotCheckable {
        reason: String,
    },
    Failed {
        error: String,
    },
}

#[must_use = "only checks the name"]
pub fn is_input_name(name: &str) -> bool {
    let mut chars = name.chars();

    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}

#[cfg(test)]
mod tests;
