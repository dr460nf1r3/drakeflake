mod boot;
mod build;
mod save;

use std::sync::Arc;

pub use boot::start_image;
pub use build::build_image;
use drakeflake_flake::{FlakeError, Nix, SystemKind, Target};
pub use save::save_image;

use super::session::Session;
use super::{Capabilities, Hub, HubError, ImagesMsg};

pub(super) fn image_target(session: &Session, host: &str) -> Result<Target, HubError> {
    if !Capabilities::selected(session).images {
        return Err(HubError::invalid(
            "Creating images needs a NixOS host on a NixOS machine.",
        ));
    }

    let target = session.target().ok_or(HubError::NoTarget)?;

    Ok(if host.is_empty() {
        target
    } else {
        Target {
            flake: target.flake,
            host: host.into(),
            kind: SystemKind::NixOs,
        }
    })
}

pub fn load_image_formats(owner: &Arc<Hub>, host: String) {
    owner.spawn_background("image-formats", move |hub, nix| {
        let result = image_target(hub.session(), &host)
            .map_err(|error| error.to_string())
            .and_then(|target| image_formats(nix, &target));

        match &result {
            Ok(formats) => log::debug!("image formats of {host:?}: {}", formats.len()),
            Err(error) => log::warn!("image formats of {host:?}: {error}"),
        }

        hub.mail.images.post(ImagesMsg::Formats(result));
    });
}

pub fn image_formats(nix: &Nix, target: &Target) -> Result<Vec<String>, String> {
    let installable = format!(
        "{}.config.system.build.images",
        target.config_ref().map_err(|error| error.to_string())?
    );

    let output = drakeflake_flake::run_nix(
        nix,
        target.local_dir().as_deref(),
        [
            "eval",
            "--json",
            &installable,
            "--apply",
            "builtins.attrNames",
        ],
        drakeflake_exec::SHORT_TIMEOUT,
    )
    .map_err(|error| image_list_error(&error))?;

    serde_json::from_str(&output).map_err(|err| format!("Unexpected image list: {err}"))
}

fn image_list_error(error: &FlakeError) -> String {
    if error.missing_attribute() == Some("images") {
        "This NixOS has no image builder (system.build.images needs NixOS 25.05 or newer)."
            .to_owned()
    } else {
        error.to_string()
    }
}

fn is_format_name(format: &str) -> bool {
    !format.is_empty()
        && format
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
}

#[cfg(test)]
mod tests;
