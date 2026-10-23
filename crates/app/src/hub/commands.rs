use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use drakeflake_exec::Context;
use drakeflake_flake::Nix;

use super::services::Services;
use super::{Cache, CommandLookupError, Hub, PackagesMsg, find_program, try_package};

const LOCATE_TIMEOUT: Duration = Duration::from_secs(60);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

const OUTPUTS: &[&str] = &["out", "bin", "lib", "dev", "man", "doc", "info", "debug"];

pub fn find_command(owner: &Arc<Hub>, command: String) {
    owner.spawn_background("find-command", move |hub, nix| {
        let result = locate(hub.services(), nix, &command);
        match &result {
            Ok(found) => log::debug!("command {command}: {} packages", found.len()),
            Err(CommandLookupError::MissingIndex) => {
                log::debug!("command {command}: no nix-index database");
            }
            Err(CommandLookupError::Failed(error)) => {
                log::warn!("command {command}: lookup failed: {error}");
            }
        }

        hub.mail.packages.post(PackagesMsg::Commands {
            command,
            result: result.map(|attrs| {
                attrs
                    .into_iter()
                    .map(drakeflake_render::PackageAttr::from)
                    .collect()
            }),
        });
    });
}

fn locate(
    services: Services<'_>,
    nix: &Nix,
    command: &str,
) -> Result<Vec<String>, CommandLookupError> {
    if command.is_empty() || command.contains('/') || command.chars().any(char::is_whitespace) {
        return Err("Type the name of a program, e.g. rg".into());
    }

    let database = database_dir().ok_or(CommandLookupError::MissingIndex)?;
    let locate = nix_locate(services, nix)?;
    log::debug!(
        "looking up {command} with {} in {}",
        locate.display(),
        database.display()
    );
    let pattern = format!("/bin/{command}");
    let args = [
        OsStr::new("--db"),
        database.as_os_str(),
        OsStr::new("--minimal"),
        OsStr::new("--whole-name"),
        OsStr::new("--at-root"),
        OsStr::new(&pattern),
    ];

    let captured = nix.context().command(&locate, args, LOCATE_TIMEOUT)?;
    if !captured.success {
        return Err(captured.stderr.trim().into());
    }

    Ok(attributes(&captured.stdout))
}

pub fn ready() -> bool {
    database_dir().is_some() && find_program("nix-locate").is_some()
}

fn nix_locate(services: Services<'_>, nix: &Nix) -> Result<PathBuf, String> {
    if let Some(path) = find_program("nix-locate") {
        return Ok(path);
    }

    let target = services
        .session
        .target()
        .ok_or("No system is selected yet")?;

    log::debug!("nix-locate not on PATH: building nix-index");
    let built = {
        let _activity = services
            .console
            .activity("find-command", "Getting nix-locate…");

        try_package::build_tool(nix, &target, "nix-index")
    };

    let bin = built?
        .into_iter()
        .map(|path| path.join("bin/nix-locate"))
        .find(|path| path.is_file())
        .ok_or("nix-index has no nix-locate")?;

    Ok(bin)
}

pub fn download_command_index(owner: &Arc<Hub>) {
    owner.spawn_background("command-index", |hub, nix| {
        let result = {
            let _activity = hub
                .console()
                .activity("command-index", "Downloading the command index…");

            download_database(nix.context())
        };

        match result {
            Ok(()) => {
                hub.log("Command index downloaded.");
                hub.mail.packages.post(PackagesMsg::CommandIndexReady);
            }
            Err(error) => {
                hub.log_error(format!("Could not download the command index: {error}"));
            }
        }
    });
}

fn database_url() -> String {
    let os = match drakeflake_flake::SystemKind::native() {
        drakeflake_flake::SystemKind::NixOs => "linux",
        drakeflake_flake::SystemKind::Darwin => "darwin",
    };

    format!(
        "https://github.com/nix-community/nix-index-database/releases/latest/download/index-{}-{os}",
        std::env::consts::ARCH
    )
}

fn own_database_dir() -> Result<PathBuf, String> {
    Ok(Cache::root()
        .map_err(|err| err.to_string())?
        .join("nix-index"))
}

fn database_dir() -> Option<PathBuf> {
    let home_default =
        std::env::var_os("HOME").map(|home| Path::new(&home).join(".cache/nix-index"));

    [own_database_dir().ok(), home_default]
        .into_iter()
        .flatten()
        .find(|dir| dir.join("files").is_file())
}

fn download_database(exec: &Context) -> Result<(), String> {
    let dir = own_database_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("cannot create {}: {err}", dir.display()))?;
    let partial = dir.join("files.part");
    let url = database_url();
    log::debug!("downloading {url} to {}", dir.display());

    let captured = exec
        .command(
            "curl",
            [
                OsStr::new("-sSfL"),
                OsStr::new("--proto"),
                OsStr::new("=https"),
                OsStr::new("--proto-redir"),
                OsStr::new("=https"),
                OsStr::new("--retry"),
                OsStr::new("2"),
                OsStr::new("-o"),
                partial.as_os_str(),
                OsStr::new("--"),
                OsStr::new(&url),
            ],
            DOWNLOAD_TIMEOUT,
        )
        .map_err(|error| error.to_string())?;

    if !captured.success {
        if let Err(err) = std::fs::remove_file(&partial)
            && err.kind() != ErrorKind::NotFound
        {
            log::warn!("cannot remove {}: {err}", partial.display());
        }

        return Err(captured.stderr.trim().to_owned());
    }

    let database = dir.join("files");
    std::fs::rename(&partial, &database).map_err(|err| err.to_string())?;
    log::debug!(
        "command index downloaded: {} ({} bytes)",
        database.display(),
        std::fs::metadata(&database).map_or(0, |meta| meta.len())
    );
    Ok(())
}

fn attributes(listing: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();

    for line in listing
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let attr = match line.rsplit_once('.') {
            Some((attr, output)) if OUTPUTS.contains(&output) => attr,
            _ => line,
        };

        if !found.iter().any(|known| known == attr) {
            found.push(attr.to_owned());
        }
    }

    found
}

#[cfg(test)]
mod tests;
