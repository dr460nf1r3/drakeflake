use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use crate::flake_target::SystemKind;

const USUAL_FLAKE_DIRS: [&str; 11] = [
    "nixos",
    ".nixos",
    "nixos-config",
    ".config/nixos",
    "nix-config",
    "dotfiles",
    ".dotfiles",
    "flake",
    ".config/nix",
    "nix-darwin",
    ".config/nix-darwin",
];
const SCAN_DEPTH: usize = 3;
const SCAN_LIMIT: usize = 4000;
const SCAN_SKIP: [&str; 6] = [
    "node_modules",
    "target",
    "result",
    ".git",
    ".cache",
    ".local",
];
const SEARCHED_HIDDEN: [&str; 2] = [".config", ".dotfiles"];

#[must_use = "the flakes are only found, nothing is loaded"]
pub fn find_system_flakes(home: Option<&Path>) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut add = |dir: &Path| {
        if defines_systems(dir) {
            let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
            if !found.contains(&dir) {
                found.push(dir);
            }
        }
    };

    let native = SystemKind::native();
    let other = match native {
        SystemKind::NixOs => SystemKind::Darwin,
        SystemKind::Darwin => SystemKind::NixOs,
    };

    add(Path::new(native.default_flake_path()));
    add(Path::new(other.default_flake_path()));

    if let Some(home) = home {
        for dir in USUAL_FLAKE_DIRS {
            add(&home.join(dir));
        }

        scan(home, &mut add);
    }

    log::debug!("system flakes found: {found:?}");
    found
}

fn scan(home: &Path, visit: &mut impl FnMut(&Path)) {
    let mut queue = VecDeque::from([(home.to_path_buf(), 0)]);

    let mut read = 0;

    while let Some((dir, depth)) = queue.pop_front() {
        read += 1;
        if read > SCAN_LIMIT {
            log::debug!("flake scan stopped after {SCAN_LIMIT} directories");
            break;
        }

        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());

            if !is_dir || SCAN_SKIP.contains(&name.as_ref()) {
                continue;
            }

            let path = entry.path();
            visit(&path);

            let searchable = !name.starts_with('.') || SEARCHED_HIDDEN.contains(&name.as_ref());
            if depth + 1 < SCAN_DEPTH && searchable {
                queue.push_back((path, depth + 1));
            }
        }
    }
}

fn defines_systems(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("flake.nix")).is_ok_and(|text| {
        text.contains("nixosConfigurations") || text.contains("darwinConfigurations")
    })
}

#[cfg(test)]
mod tests;
