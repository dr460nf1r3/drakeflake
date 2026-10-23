use std::path::Path;

use drakeflake_exec::SHORT_TIMEOUT;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::context::Nix;
use crate::error::FlakeError;
use crate::expr::EVAL_VERSION;
use crate::flake_target::{SystemKind, Target, local_flake_dir, validate_flake, validate_host};
use crate::nix::{failure, parse_json, run_nix, run_nix_raw};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Host {
    pub name: String,
    pub kind: SystemKind,
}

#[derive(Debug, Default)]
struct HostNames {
    nixos: Vec<String>,
    darwin: Vec<String>,
}

impl HostNames {
    /// The `native` kind first: a name declared under both attributes then
    /// resolves to the host that can be built and activated here.
    fn into_hosts(self, native: SystemKind) -> Vec<Host> {
        let nixos = self.nixos.into_iter().map(|name| Host {
            name,
            kind: SystemKind::NixOs,
        });

        let darwin = self.darwin.into_iter().map(|name| Host {
            name,
            kind: SystemKind::Darwin,
        });

        match native {
            SystemKind::NixOs => nixos.chain(darwin).collect(),
            SystemKind::Darwin => darwin.chain(nixos).collect(),
        }
    }
}

pub fn fingerprint(nix: &Nix, target: &Target) -> Result<String, FlakeError> {
    validate_flake(&target.flake)?;
    validate_host(&target.host)?;

    let stdout = run_nix(
        nix,
        target.local_dir().as_deref(),
        ["flake", "metadata", "--json", target.flake.trim()],
        SHORT_TIMEOUT,
    )?;

    let metadata: Value = parse_json(&stdout, "flake metadata")?;
    let fingerprint = fingerprint_of(&metadata, nix, target)?;

    log::debug!("fingerprint of {}: {fingerprint}", target.flake.trim());
    Ok(fingerprint)
}

fn fingerprint_of(metadata: &Value, nix: &Nix, target: &Target) -> Result<String, FlakeError> {
    let nar_hash = metadata
        .pointer("/locked/narHash")
        .and_then(Value::as_str)
        .ok_or(FlakeError::NoContentHash)?;

    let worktree = worktree_state(nix, target);

    Ok(format!(
        "{nar_hash} {worktree} {}:{} v{EVAL_VERSION}",
        target.kind.as_str(),
        target.host
    ))
}

/// `narHash` ignores uncommitted edits, so the worktree state is hashed too.
fn worktree_state(nix: &Nix, target: &Target) -> String {
    let Some(dir) = target.local_dir() else {
        return String::new();
    };

    let captured = nix
        .context()
        .probe(
            "git",
            [
                std::ffi::OsStr::new("-C"),
                dir.as_os_str(),
                std::ffi::OsStr::new("status"),
                std::ffi::OsStr::new("--porcelain"),
            ],
            SHORT_TIMEOUT,
        )
        .ok()
        .filter(|captured| captured.success);

    let Some(captured) = captured else {
        return String::new();
    };

    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in captured.stdout.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }

    format!("{hash:016x}")
}

pub fn list_hosts(nix: &Nix, flake: &str) -> Result<Vec<Host>, FlakeError> {
    validate_flake(flake)?;

    let cwd = local_flake_dir(flake);
    let (nixos, darwin) = std::thread::scope(|scope| {
        let names = |kind: SystemKind| attr_names(nix, flake, kind.attr(), cwd.as_deref());

        let darwin = scope.spawn(move || names(SystemKind::Darwin));
        let nixos = names(SystemKind::NixOs);

        (
            nixos,
            darwin
                .join()
                .unwrap_or(Err(FlakeError::ThreadCrashed("the host listing thread"))),
        )
    });

    let names = HostNames {
        nixos: nixos?,
        darwin: darwin?,
    };

    let hosts = names.into_hosts(SystemKind::native());

    let summary = hosts
        .iter()
        .map(|host| format!("{}:{}", host.kind.as_str(), host.name))
        .collect::<Vec<_>>()
        .join(", ");

    log::debug!("{} declares hosts: {summary}", flake.trim());
    Ok(hosts)
}

fn attr_names(
    nix: &Nix,
    flake: &str,
    attr: &str,
    cwd: Option<&Path>,
) -> Result<Vec<String>, FlakeError> {
    let installable = format!("{}#{attr}", flake.trim());

    let captured = run_nix_raw(
        nix,
        cwd,
        [
            "eval",
            "--json",
            &installable,
            "--apply",
            "builtins.attrNames",
        ],
        SHORT_TIMEOUT,
        log::Level::Debug,
    )?;

    if captured.success {
        return parse_json(&captured.stdout, "host list");
    }

    if is_missing_output(&captured.stderr, attr) {
        log::debug!("{} has no {attr}", flake.trim());
        return Ok(Vec::new());
    }

    let error = failure(&captured.stderr);
    log::warn!("listing {attr} of {} failed: {error}", flake.trim());
    Err(error)
}

/// Nix has no exit status for this, so only exact complaints count.
fn is_missing_output(stderr: &str, attr: &str) -> bool {
    let quoted = format!("'{attr}'");

    stderr.lines().any(|line| {
        line.contains(&quoted)
            && (line.contains("does not provide attribute")
                || line.contains(&format!("{quoted} missing")))
    })
}

#[must_use = "only reads the host name"]
pub fn local_hostname() -> Option<String> {
    let clean = |text: String| Some(text.trim().to_owned()).filter(|host| !host.is_empty());

    std::fs::read_to_string("/etc/hostname")
        .ok()
        .and_then(clean)
        .or_else(|| std::env::var("HOSTNAME").ok().and_then(clean))
        .or_else(|| macos_hostname().and_then(clean))
}

pub fn choose_host(
    hosts: &[Host],
    preferred: Option<&str>,
    local: Option<&str>,
) -> Result<Host, FlakeError> {
    let declared = |name: &str| hosts.iter().find(|host| host.name == name).cloned();

    if let Some(wanted) = preferred.filter(|name| !name.is_empty()) {
        return declared(wanted).ok_or_else(|| FlakeError::HostNotDeclared {
            host: wanted.into(),
            found: host_names(hosts),
        });
    }

    if let Some(local) = local.and_then(declared) {
        log::debug!(
            "no host configured; using this machine's host {}",
            local.name
        );
        return Ok(local);
    }

    match hosts {
        [] => Err(FlakeError::NoHostsDeclared),
        [only] => {
            log::debug!(
                "no host configured; using the only declared host {}",
                only.name
            );
            Ok(only.clone())
        }
        _ => Err(FlakeError::SeveralHosts(host_names(hosts))),
    }
}

fn host_names(hosts: &[Host]) -> String {
    hosts
        .iter()
        .map(|host| host.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// macOS has neither `/etc/hostname` nor `HOSTNAME` set by default.
#[cfg(target_os = "macos")]
fn macos_hostname() -> Option<String> {
    let out = std::process::Command::new("scutil")
        .args(["--get", "LocalHostName"])
        .output()
        .ok()
        .filter(|out| out.status.success())?;

    String::from_utf8(out.stdout).ok()
}

#[cfg(not(target_os = "macos"))]
const fn macos_hostname() -> Option<String> {
    None
}

#[cfg(test)]
mod tests;
