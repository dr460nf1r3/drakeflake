use std::path::{Path, PathBuf};

/// Matches what Nix itself uses for a path inside a flake.
#[must_use = "the root is only looked up"]
pub fn find_flake_root(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|candidate| candidate.join("flake.nix").is_file())
        .map(Path::to_path_buf)
}
