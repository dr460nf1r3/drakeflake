use drakeflake_flake::{Host, SystemKind};
use drakeflake_render::HostName;
use serde_json::{Value, json};

use super::SystemModel;
use super::runs::Outcome;
use crate::hub::{CacheUse, Capabilities, InputsMsg, SetupMsg, TargetInfo};

#[derive(Default)]
pub struct TargetState {
    pub flake: String,
    pub host: String,
    pub kind: Option<SystemKind>,
    pub hosts: Vec<Host>,
    pub managed_setting: String,
    pub managed_path: String,
    pub error: String,
    pub ready: bool,
}

impl TargetState {
    pub fn from_config(nix: &drakeflake_config::Nix) -> Self {
        Self {
            flake: nix.flake_path.clone(),
            host: nix.host.clone().map(String::from).unwrap_or_default(),
            managed_setting: nix.managed_path.clone().unwrap_or_default(),
            ..Self::default()
        }
    }

    #[must_use = "the setting is only read"]
    pub fn host_setting(&self) -> Option<String> {
        (!self.host.is_empty()).then(|| self.host.clone())
    }

    #[must_use = "the setting is only read"]
    pub fn setting(&self) -> Option<String> {
        (!self.managed_setting.is_empty()).then(|| self.managed_setting.clone())
    }

    #[must_use = "the JSON is only built"]
    pub fn hosts_json(&self) -> Value {
        self.hosts
            .iter()
            .map(|host| json!({ "name": host.name, "kind": host.kind.as_str() }))
            .collect()
    }

    #[must_use = "the capabilities are only read"]
    pub fn capabilities(&self) -> Capabilities {
        Capabilities::here(self.kind.unwrap_or_else(SystemKind::native))
    }
}

impl SystemModel {
    pub(super) fn load_configured(&self) {
        log::info!(
            "loading {}#{} (managed file setting {:?})",
            self.target.flake,
            self.target.host,
            self.target.managed_setting
        );
        self.load(false);
    }

    pub(super) fn load(&self, rebuild_cache: bool) {
        self.hub.start(
            self.target.flake.clone(),
            self.target.host_setting(),
            self.target.setting(),
            CacheUse::reuse_or_rebuild(rebuild_cache),
        );
    }

    pub(super) fn reload_target(&mut self, rebuild_cache: bool) {
        log::info!(
            "reloading {}#{} (rebuild cache: {rebuild_cache})",
            self.target.flake,
            self.target.host
        );
        self.target.ready = false;
        self.home_check = None;
        self.load(rebuild_cache);
    }

    pub(super) fn switch_target(&mut self, flake: &str, host: &str, managed: &str) {
        if flake.is_empty() {
            self.hub
                .log_warn("The flake path is empty; keeping the current target.");
            return;
        }

        if let Err(error) = check_target(host, managed) {
            self.hub.log_warn(error);
            return;
        }

        log::info!("target changed: flake {flake}, host {host:?}, managed file {managed:?}");
        self.hub.mail().setup.post(SetupMsg::TargetChanged);

        if self.target.ready && flake == self.target.flake && host == self.target.host {
            managed.clone_into(&mut self.target.managed_setting);
            self.save_target();
            self.hub
                .set_managed(self.target.setting(), self.target.hosts.clone());
            return;
        }

        self.set_flake(flake.to_owned());
        host.clone_into(&mut self.target.host);
        managed.clone_into(&mut self.target.managed_setting);
        self.target.kind = None;
        self.target.ready = false;
        self.home_check = None;
        self.target.error.clear();
        self.run.last_backup = None;
        self.run.last_result = Outcome::Unknown;
        self.save_target();
        self.load(false);
    }

    pub(super) fn on_target(&mut self, info: TargetInfo) {
        let corrected = info.flake != self.target.flake;
        self.set_flake(info.flake);
        self.target.host = info.host;
        self.target.kind = Some(info.kind);
        self.target.hosts = info.hosts;
        self.target.managed_path = info
            .managed_path
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        self.target.error.clear();
        self.target.ready = true;
        log::info!(
            "system loaded: {}#{} ({} hosts), managed file {:?}",
            self.target.flake,
            self.target.host,
            self.target.hosts.len(),
            self.target.managed_path
        );

        if corrected {
            log::info!("flake path corrected to {}", self.target.flake);
            self.save_target();
        }
    }

    pub(super) fn on_target_error(&mut self, flake: String, hosts: Vec<Host>, error: String) {
        log::debug!("target error shown for {flake} ({} hosts)", hosts.len());
        self.set_flake(flake);
        self.target.hosts = hosts;
        self.target.kind = None;
        self.target.error = error;
        self.target.ready = false;
    }

    fn set_flake(&mut self, flake: String) {
        if flake != self.target.flake {
            self.hub.mail().inputs.post(InputsMsg::Flake(flake.clone()));
        }

        self.target.flake = flake;
    }

    fn save_target(&self) {
        let nix = drakeflake_config::Nix {
            flake_path: self.target.flake.clone(),
            host: self.target.host_setting().map(HostName::from),
            managed_path: self.target.setting(),
        };

        self.config.update(|config| config.nix = nix);
    }
}

fn check_target(host: &str, managed: &str) -> Result<(), String> {
    if !host.is_empty() {
        drakeflake_flake::validate_host(host).map_err(|error| error.to_string())?;
    }

    if !managed.is_empty() {
        drakeflake_file::resolve_managed_path(
            None,
            Some(managed),
            drakeflake_config::LayoutMode::Single,
        )
        .map_err(|error| error.to_string())?;
    }

    Ok(())
}
