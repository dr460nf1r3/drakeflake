use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use drakeflake_flake::Nix;

use super::super::argv::{arg, echo};
use super::super::services::Services;
use super::super::session::Session;
use super::super::{Hub, find_program, terminal, try_package};

const VM_MEMORY_MIB: &str = "4096";
const VM_CPUS: &str = "2";

pub fn start_image(owner: &Arc<Hub>, file: String, format: String) {
    owner.spawn_background("image-start", move |hub, nix| {
        match boot_image(hub.services(), nix, &file, &format) {
            Ok(()) => log::info!("starting image {file} in a VM"),
            Err(error) => {
                log::error!("cannot start image {file}: {error}");
                hub.log_error(format!("Could not start the image: {error}"));
            }
        }
    });
}

fn boot_image(services: Services<'_>, nix: &Nix, file: &str, format: &str) -> Result<(), String> {
    let mut command = vm_command(&qemu(services, nix)?, file);
    if is_efi_format(format) {
        if let Some(firmware) = ovmf(services.session, nix) {
            command.extend([arg("-bios"), arg(firmware)]);
        } else {
            log::warn!("no UEFI firmware (OVMF) found: booting {file} without it");
        }
    }

    if let Some((program, args)) = command.split_first() {
        services.console.log(echo(program, args));
    }

    terminal::spawn_detached(&command)
}

fn qemu(services: Services<'_>, nix: &Nix) -> Result<PathBuf, String> {
    let name = format!("qemu-system-{}", std::env::consts::ARCH);
    if let Some(path) = find_program(&name) {
        log::debug!("qemu: {}", path.display());
        return Ok(path);
    }

    let target = services
        .session
        .target()
        .ok_or("No system is selected yet")?;

    log::debug!("{name} not on PATH: building qemu_kvm");
    let built = {
        let _activity = services.console.activity("qemu", "Getting QEMU…");
        try_package::build_tool(nix, &target, "qemu_kvm")
    };

    built?
        .into_iter()
        .map(|path| path.join("bin").join(&name))
        .find(|path| path.is_file())
        .ok_or_else(|| format!("QEMU has no {name}"))
}

fn ovmf(session: &Session, nix: &Nix) -> Option<PathBuf> {
    let target = session.target()?;
    try_package::build_tool(nix, &target, "OVMF.fd")
        .inspect_err(|error| log::warn!("cannot build OVMF: {error}"))
        .ok()?
        .into_iter()
        .flat_map(|path| [path.join("FV/OVMF.fd"), path.join("OVMF.fd")])
        .find(|path| path.is_file())
}

fn is_efi_format(format: &str) -> bool {
    format == "efi" || format.ends_with("-efi")
}

fn vm_command(qemu: &Path, file: &str) -> Vec<OsString> {
    let mut command = vec![
        arg(qemu),
        arg("-enable-kvm"),
        arg("-m"),
        arg(VM_MEMORY_MIB),
        arg("-smp"),
        arg(VM_CPUS),
        arg("-name"),
        arg("DrakeFlake image"),
    ];

    let is_iso = Path::new(file)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("iso"));

    if is_iso {
        command.extend([arg("-cdrom"), arg(file)]);
    } else {
        command.extend([
            arg("-drive"),
            arg(format!("file={},snapshot=on", qemu_value(file))),
        ]);
    }

    command
}

fn qemu_value(text: &str) -> String {
    text.replace(',', ",,")
}

#[cfg(test)]
mod tests;
