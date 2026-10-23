use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use drakeflake_flake::{Nix, Target};

use super::super::argv::{arg, echo};
use super::super::operations::{Exclusive, Outcome, Reporting};
use super::super::{Cache, Hub, HubError, ImagesMsg};
use super::{image_target, is_format_name};

impl Exclusive {
    fn build_image(&self, target: &Target, format: String) -> Outcome {
        let reference = match target.config_ref() {
            Ok(reference) => reference,
            Err(error) => {
                self.console().error(error.to_string());
                return Outcome::Failed;
            }
        };

        let installable = format!("{reference}.config.system.build.images.{format}");
        let root = match Cache::root() {
            Ok(root) => root,
            Err(error) => {
                self.console().error(error.to_string());
                return Outcome::Failed;
            }
        };

        let dir = root.join("images");
        if let Err(err) = std::fs::create_dir_all(&dir) {
            log::warn!("cannot create {}: {err}", dir.display());
        }

        let link = dir.join(&format);
        let Some(cwd) = target.local_dir().or_else(|| self.work_dir()) else {
            return Outcome::Failed;
        };

        let args = build_args(&installable, &link);

        self.console()
            .log(format!("Building the {format} image of {}…", target.host));
        self.console().log(echo(OsStr::new("nix"), &args));
        let built = self.stream_pty(OsStr::new("nix"), &args, &cwd);
        if !built.is_success() {
            return built;
        }

        let Some(file) = image_file(self.nix(), target, &installable, &link) else {
            self.console()
                .error("The build finished but produced no image file.");
            return Outcome::Failed;
        };

        self.console()
            .log(format!("Image ready: {}", file.display()));
        log::debug!(
            "image size: {} bytes",
            std::fs::metadata(&file).map_or(0, |meta| meta.len())
        );
        self.mail().images.post(ImagesMsg::Built {
            format,
            file: file.to_string_lossy().into_owned(),
        });
        Outcome::Succeeded
    }
}

pub fn build_image(hub: &Arc<Hub>, host: &str, format: String) -> Result<(), HubError> {
    if !is_format_name(&format) {
        return Err(HubError::invalid(format!(
            "\"{format}\" is not an image format"
        )));
    }

    let target = image_target(hub.session(), host)?;

    hub.spawn_exclusive("image", Reporting::Shown, move |run| {
        run.build_image(&target, format)
    })
}

fn image_file(nix: &Nix, target: &Target, installable: &str, link: &Path) -> Option<PathBuf> {
    let declared = drakeflake_flake::run_nix(
        nix,
        target.local_dir().as_deref(),
        ["eval", "--raw", &format!("{installable}.passthru.filePath")],
        drakeflake_exec::SHORT_TIMEOUT,
    )
    .inspect_err(|error| log::debug!("image has no declared filePath: {error}"))
    .ok()
    .and_then(|relative| inside_output(link, relative.trim()));

    declared.or_else(|| {
        log::debug!("using the largest file of {}", link.display());
        largest_file(&canonical_or_log(link)?)
    })
}

fn inside_output(link: &Path, relative: &str) -> Option<PathBuf> {
    let output = canonical_or_log(link)?;
    let file = output.join(relative).canonicalize().ok()?;

    if file.starts_with(&output) && file.is_file() {
        Some(file)
    } else {
        log::warn!("ignoring the image's filePath {relative}: not a file in the image");
        None
    }
}

fn canonical_or_log(path: &Path) -> Option<PathBuf> {
    path.canonicalize()
        .inspect_err(|err| log::warn!("cannot resolve {}: {err}", path.display()))
        .ok()
}

fn build_args(installable: &str, link: &Path) -> Vec<OsString> {
    drakeflake_flake::NIX_FEATURES
        .iter()
        .map(arg)
        .chain([
            arg("build"),
            arg(installable),
            arg("--out-link"),
            arg(link),
            arg("-L"),
        ])
        .collect()
}

fn largest_file(dir: &Path) -> Option<PathBuf> {
    largest_file_at(dir, 0)
}

fn largest_file_at(dir: &Path, depth: usize) -> Option<PathBuf> {
    const MAX_DEPTH: usize = 32;

    if depth > MAX_DEPTH {
        return None;
    }

    if dir.is_file() && !dir.is_symlink() {
        return Some(dir.to_path_buf());
    }

    if dir.is_symlink() {
        return std::fs::symlink_metadata(dir)
            .ok()
            .filter(std::fs::Metadata::is_file)
            .map(|_| dir.to_path_buf());
    }

    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter_map(|path| {
            if path.is_dir() && !path.is_symlink() {
                largest_file_at(&path, depth + 1)
            } else {
                Some(path)
            }
        })
        .max_by_key(|path| std::fs::metadata(path).map_or(0, |meta| meta.len()))
}

#[cfg(test)]
mod tests;
