//! Runs every minute, so each check stays cheap: file reads, one `statvfs` or one `systemctl` call.

use std::path::Path;

const GIB: u64 = 1024 * 1024 * 1024;
const LOW_SPACE: u64 = 5 * GIB;
const BOOT_PARTS: [&str; 3] = ["kernel", "initrd", "kernel-modules"];

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Health {
    pub restart_needed: bool,
    pub staged: usize,
    pub failed_units: Vec<String>,
    pub low_space: Option<u64>,
}

impl Health {
    #[must_use = "this only reads the findings, the tray shows them"]
    pub fn read(reports: drakeflake_config::Tray) -> Self {
        let space = rustix::fs::statvfs("/nix/store")
            .ok()
            .filter(|_| reports.low_space)
            .map(|stat| {
                (
                    stat.f_bavail.saturating_mul(stat.f_frsize),
                    stat.f_blocks.saturating_mul(stat.f_frsize),
                )
            });

        Self {
            restart_needed: reports.restart
                && restart_needed(
                    Path::new("/run/booted-system"),
                    Path::new("/run/current-system"),
                ),
            staged: if reports.staged {
                drakeflake_file::staged_dir()
                    .inspect_err(|err| drakeflake_log::warn!("staged edits not counted: {err}"))
                    .map_or(0, |dir| staged_edits(&dir))
            } else {
                0
            },
            failed_units: if reports.failed_services {
                failed_units()
            } else {
                Vec::new()
            },
            low_space: space.and_then(|(free, total)| is_low(free, total).then_some(free)),
        }
    }

    #[must_use = "this only builds the notes"]
    pub fn notes(&self) -> Vec<String> {
        let mut notes = Vec::new();
        if self.restart_needed {
            notes.push("Restart to finish the update (new kernel)".to_owned());
        }

        if !self.failed_units.is_empty() {
            notes.push(format!("Failed services: {}", self.failed_units.join(", ")));
        }

        if let Some(free) = self.low_space {
            notes.push(format!("Low disk space: {} free for Nix", gib(free)));
        }

        match self.staged {
            0 => {}
            1 => notes.push("1 change staged in DrakeFlake, not applied".to_owned()),
            count => notes.push(format!("{count} changes staged in DrakeFlake, not applied")),
        }

        notes
    }

    #[must_use = "this only tells whether a finding wants attention"]
    pub const fn needs_attention(&self) -> bool {
        self.restart_needed || !self.failed_units.is_empty() || self.low_space.is_some()
    }
}

pub fn report_health(before: &Health, now: &Health, notify: impl Fn(&str, &str)) {
    if now.restart_needed && !before.restart_needed {
        notify(
            "Restart needed",
            "The system was switched to a new kernel. Restart to finish the update.",
        );
    }

    let new_failures: Vec<&str> = now
        .failed_units
        .iter()
        .filter(|unit| !before.failed_units.contains(unit))
        .map(String::as_str)
        .collect();

    if !new_failures.is_empty() {
        notify(
            "Services failed",
            &format!(
                "{} failed. Check `systemctl status` for the reason.",
                new_failures.join(", ")
            ),
        );
    }

    if let (Some(free), None) = (now.low_space, before.low_space) {
        notify(
            "Low disk space",
            &format!(
                "Only {} free for the Nix store. Open DrakeFlake → Cleanup to free space.",
                gib(free)
            ),
        );
    }
}

#[must_use = "this only compares the two systems"]
pub fn restart_needed(booted: &Path, current: &Path) -> bool {
    BOOT_PARTS.iter().any(|part| {
        match (
            booted.join(part).canonicalize(),
            current.join(part).canonicalize(),
        ) {
            (Ok(booted), Ok(current)) => booted != current,
            _ => false,
        }
    })
}

#[must_use = "this only counts the files"]
pub fn staged_edits(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };

    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .map(|saved| {
            ["options", "packages"]
                .iter()
                .filter_map(|field| saved.get(field)?.as_object().map(serde_json::Map::len))
                .sum::<usize>()
        })
        .sum()
}

fn failed_units() -> Vec<String> {
    if !cfg!(target_os = "linux") {
        return Vec::new();
    }

    std::process::Command::new("systemctl")
        .args(["--failed", "--no-legend", "--plain", "--type=service"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| failed_units_in(&String::from_utf8_lossy(&out.stdout)))
        .unwrap_or_default()
}

#[must_use = "this only parses the output"]
pub fn failed_units_in(output: &str) -> Vec<String> {
    // Some systemctl versions mark failed units with a leading `●`.
    output
        .lines()
        .filter_map(|line| {
            line.trim_start_matches(['●', ' '])
                .split_whitespace()
                .next()
        })
        .map(str::to_owned)
        .collect()
}

#[must_use = "this only compares the sizes"]
pub const fn is_low(free: u64, total: u64) -> bool {
    free < LOW_SPACE || free < total / 10
}

fn gib(bytes: u64) -> String {
    let tenths = bytes.saturating_mul(10) / GIB;
    format!("{}.{} GiB", tenths / 10, tenths % 10)
}

#[cfg(test)]
mod tests;
