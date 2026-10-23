use std::path::{Path, PathBuf};

use drakeflake_config::{DEFAULT_DARWIN_FLAKE_PATH, DEFAULT_FLAKE_PATH};
use drakeflake_plan::find_flake_root;
use drakeflake_render::HostName;

use crate::error::{FlakeError, FlakeProblem, HostProblem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemKind {
    #[default]
    NixOs,
    Darwin,
}

impl SystemKind {
    #[must_use = "only returns the attribute name"]
    pub const fn attr(self) -> &'static str {
        match self {
            Self::NixOs => "nixosConfigurations",
            Self::Darwin => "darwinConfigurations",
        }
    }

    #[must_use = "only returns the name"]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NixOs => "nixos",
            Self::Darwin => "darwin",
        }
    }

    #[must_use = "only returns this machine's kind"]
    pub const fn native() -> Self {
        if cfg!(target_os = "macos") {
            Self::Darwin
        } else {
            Self::NixOs
        }
    }

    #[must_use = "only returns the path"]
    pub const fn default_flake_path(self) -> &'static str {
        match self {
            Self::NixOs => DEFAULT_FLAKE_PATH,
            Self::Darwin => DEFAULT_DARWIN_FLAKE_PATH,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Target {
    pub flake: String,
    pub host: HostName,
    pub kind: SystemKind,
}

impl Target {
    pub fn config_ref(&self) -> Result<String, FlakeError> {
        validate_flake(&self.flake)?;
        validate_host(&self.host)?;

        Ok(format!(
            "{}#{}.\"{}\"",
            self.flake.trim(),
            self.kind.attr(),
            self.host
        ))
    }

    #[must_use = "the path is only computed"]
    pub fn local_dir(&self) -> Option<PathBuf> {
        local_flake_dir(&self.flake)
    }
}

/// Local folders canonicalized: `getFlake` rejects symlink components.
#[must_use = "the reference is only computed"]
pub fn get_flake_ref(flake: &str) -> String {
    local_flake_dir(flake).map_or_else(
        || flake.trim().to_owned(),
        |dir| reference_text(&dir.canonicalize().unwrap_or(dir)),
    )
}

fn reference_text(dir: &Path) -> String {
    dir.to_string_lossy().into_owned()
}

#[must_use = "only compares the references"]
pub fn same_flake(loaded: &str, next: &str) -> bool {
    let loaded = loaded.trim();
    let next = next.trim();
    if loaded == next {
        return true;
    }

    match (local_flake_dir(loaded), local_flake_dir(next)) {
        (Some(first), Some(second)) => {
            let first = first.canonicalize().unwrap_or(first);
            let second = second.canonicalize().unwrap_or(second);
            first == second
        }
        _ => false,
    }
}

#[must_use = "the input is not changed in place"]
pub fn normalize_flake(flake: &str) -> String {
    let trimmed = flake.trim();

    let Some(dir) = local_flake_dir(trimmed) else {
        return trimmed.to_owned();
    };

    if dir.join("flake.nix").is_file() {
        return trimmed.to_owned();
    }

    let Some(root) = find_flake_root(&dir) else {
        log::debug!("{} has no flake.nix and no enclosing flake", dir.display());
        return trimmed.to_owned();
    };

    log::info!(
        "{} has no flake.nix; using the enclosing flake {}",
        dir.display(),
        root.display()
    );
    reference_text(&root)
}

#[must_use = "the path is only computed"]
pub fn local_flake_dir(flake: &str) -> Option<PathBuf> {
    let trimmed = flake.trim();
    let path = trimmed
        .strip_prefix("path:")
        .or_else(|| trimmed.strip_prefix("git+file://"))
        .unwrap_or(trimmed);

    let path = path.split(['?', '#']).next().unwrap_or(path);

    if path.starts_with('/') {
        Some(PathBuf::from(path))
    } else if let Some(rest) = path.strip_prefix("~/") {
        std::env::var_os("HOME").map(|home| Path::new(&home).join(rest))
    } else {
        None
    }
}

pub fn validate_flake(flake: &str) -> Result<(), FlakeError> {
    let trimmed = flake.trim();

    let problem = if trimmed.is_empty() {
        FlakeProblem::Empty
    } else if trimmed.contains('#') {
        FlakeProblem::HasFragment
    } else if trimmed.starts_with('-') {
        FlakeProblem::StartsWithDash
    } else if trimmed.chars().any(char::is_control) {
        FlakeProblem::ControlCharacters
    } else if names_non_utf8_dir(trimmed) {
        FlakeProblem::NotUtf8
    } else if is_relative_local(trimmed) {
        FlakeProblem::RelativePath
    } else {
        return Ok(());
    };

    Err(FlakeError::InvalidFlake(problem))
}

fn is_relative_local(flake: &str) -> bool {
    for prefix in ["path:", "git+file://"] {
        if let Some(path) = flake.strip_prefix(prefix) {
            let path = path.split(['?', '#']).next().unwrap_or(path);
            return !(path.starts_with('/') || path.starts_with("~/"));
        }
    }

    let path = flake.split(['?', '#']).next().unwrap_or(flake);

    if path.starts_with('/') || path.starts_with("~/") || path.contains(':') {
        return false;
    }

    if path == "." || path == ".." || path.starts_with("./") || path.starts_with("../") {
        return true;
    }

    path.contains('/')
}

fn names_non_utf8_dir(flake: &str) -> bool {
    local_flake_dir(flake).is_some_and(|dir| {
        let resolved = dir.canonicalize().ok();

        lacks_text_form(&dir, resolved.as_deref())
    })
}

fn lacks_text_form(dir: &Path, resolved: Option<&Path>) -> bool {
    dir.to_str().is_none() || resolved.is_some_and(|resolved| resolved.to_str().is_none())
}

/// Host names are interpolated into a quoted attribute, so anything odd is rejected.
pub fn validate_host(host: &str) -> Result<(), FlakeError> {
    if host.is_empty() {
        return Err(FlakeError::InvalidHost(HostProblem::Empty));
    }

    if host
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        Ok(())
    } else {
        Err(FlakeError::InvalidHost(HostProblem::Unsupported(
            host.to_owned(),
        )))
    }
}

#[cfg(test)]
mod tests;
