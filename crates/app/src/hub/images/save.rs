use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::super::{Hub, ImagesMsg};

pub fn save_image(owner: &Arc<Hub>, file: String, destination: PathBuf) {
    owner.spawn_background("image-save", move |hub, _| {
        match copy_image(&file, &destination) {
            Ok(()) => hub.log(format!("Saved the image to {}", destination.display())),
            Err(error) => hub.log_error(error),
        }

        hub.mail.images.post(ImagesMsg::Saved);
    });
}

fn copy_image(file: &str, destination: &Path) -> Result<(), String> {
    let bytes = std::fs::copy(file, destination).map_err(|err| {
        let error = format!("cannot copy to {}: {err}", destination.display());
        log::error!("cannot save image {file}: {error}");
        error
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if let Err(err) =
            std::fs::set_permissions(destination, std::fs::Permissions::from_mode(0o644))
        {
            log::warn!("cannot make {} writable: {err}", destination.display());
        }
    }

    log::info!("image saved: {} ({bytes} bytes)", destination.display());
    Ok(())
}

#[cfg(test)]
mod tests;
