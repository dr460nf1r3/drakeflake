//! One lock so a reader never sees a new target next to the old fingerprint.

use std::path::PathBuf;
use std::sync::Mutex;

use drakeflake_config::{LayoutConfig, LayoutMode};
use drakeflake_flake::Target;
use drakeflake_render::UserName;

use super::error::HubError;
use super::ids::{Generation, HomeEpoch};
use super::lock;

#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub target: Option<Target>,
    pub home_user: Option<UserName>,
    pub managed_setting: Option<String>,
    pub layout: LayoutConfig,
    pub written_layout: Option<LayoutConfig>,
    pub fresh: Option<String>,
    pub index_key: Option<String>,
    pub package_version: Option<String>,
    pub home_epoch: HomeEpoch,
    pub generation: Generation,
    pub auto_commit: bool,
}

pub struct LoadStart {
    pub generation: Generation,
    pub same_system: bool,
}

#[derive(Debug, Default)]
pub struct Session {
    state: Mutex<SessionState>,
}

impl Session {
    #[cfg(test)]
    pub fn update(&self, change: impl FnOnce(&mut SessionState)) {
        change(&mut lock(&self.state));
    }

    #[must_use = "the snapshot is only a copy"]
    pub fn snapshot(&self) -> SessionState {
        lock(&self.state).clone()
    }

    pub fn target(&self) -> Option<Target> {
        lock(&self.state).target.clone()
    }

    pub fn generation(&self) -> Generation {
        lock(&self.state).generation
    }

    pub fn is_current(&self, generation: Generation) -> bool {
        lock(&self.state).generation == generation
    }

    pub fn auto_commit(&self) -> bool {
        lock(&self.state).auto_commit
    }

    pub fn set_auto_commit(&self, enabled: bool) {
        lock(&self.state).auto_commit = enabled;
    }

    pub fn set_layout(&self, layout: LayoutConfig) {
        lock(&self.state).layout = layout;
    }

    pub fn layout(&self) -> LayoutConfig {
        lock(&self.state).layout.clone()
    }

    pub fn home_user(&self) -> Option<UserName> {
        lock(&self.state).home_user.clone()
    }

    pub fn select_home_user(&self, user: Option<UserName>) -> Option<UserName> {
        std::mem::replace(&mut lock(&self.state).home_user, user)
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "the selection stays locked while posting"
    )]
    pub fn while_home_user_is(&self, user: &str, post: impl FnOnce()) -> bool {
        let state = lock(&self.state);

        if state.home_user.as_deref() != Some(user) {
            return false;
        }

        post();
        true
    }

    pub fn fresh(&self) -> Option<String> {
        lock(&self.state).fresh.clone()
    }

    pub fn mark_fresh(&self, fingerprint: Option<&str>) {
        lock(&self.state).fresh = fingerprint.map(str::to_owned);
    }

    pub fn index_key(&self) -> Option<String> {
        lock(&self.state).index_key.clone()
    }

    pub fn set_index_key(&self, key: Option<&str>) {
        lock(&self.state).index_key = key.map(str::to_owned);
    }

    pub fn package_version(&self) -> Option<String> {
        lock(&self.state).package_version.clone()
    }

    pub fn next_home_epoch(&self) -> HomeEpoch {
        let mut state = lock(&self.state);
        state.home_epoch = state.home_epoch.next();
        state.home_epoch
    }

    pub fn home_epoch(&self) -> HomeEpoch {
        lock(&self.state).home_epoch
    }

    pub fn set_package_version(&self, version: Option<&str>) {
        lock(&self.state).package_version = version.map(str::to_owned);
    }

    pub fn set_managed_setting(&self, setting: Option<String>) {
        lock(&self.state).managed_setting = setting.filter(|text| !text.trim().is_empty());
    }

    pub fn begin_load(
        &self,
        managed_setting: Option<String>,
        is_same_system: impl FnOnce(Option<&Target>) -> bool,
    ) -> LoadStart {
        let mut state = lock(&self.state);

        state.generation = state.generation.next();
        let same_system = is_same_system(state.target.as_ref());
        state.target = None;
        state.fresh = None;
        state.index_key = None;
        state.managed_setting = managed_setting.filter(|text| !text.trim().is_empty());

        LoadStart {
            generation: state.generation,
            same_system,
        }
    }

    pub fn store_target_if_current(&self, generation: Generation, target: &Target) -> bool {
        let mut state = lock(&self.state);

        if state.generation != generation {
            return false;
        }

        state.target = Some(target.clone());
        true
    }

    pub fn note_written_layout(&self) -> bool {
        let mut state = lock(&self.state);

        if state.written_layout.as_ref() == Some(&state.layout) {
            return false;
        }

        state.written_layout = Some(state.layout.clone());
        true
    }

    pub fn managed_file(&self) -> Result<PathBuf, HubError> {
        let state = lock(&self.state);
        let target = state.target.as_ref().ok_or(HubError::NoTarget)?;
        let flake = target.flake.clone();
        let managed = state.managed_setting.clone();
        let mode = state.layout.mode;

        drop(state);
        resolve_managed(&flake, managed.as_deref(), mode)
    }

    pub fn generated_paths_of(&self, flake: &str) -> Vec<PathBuf> {
        let state = lock(&self.state);
        let managed = state.managed_setting.clone();
        let mode = state.layout.mode;

        drop(state);
        drakeflake_evaluate::generated_paths(flake, managed.as_deref(), mode)
    }
}

fn resolve_managed(
    flake: &str,
    managed: Option<&str>,
    mode: LayoutMode,
) -> Result<PathBuf, HubError> {
    drakeflake_file::resolve_managed_path(
        drakeflake_flake::local_flake_dir(flake).as_deref(),
        managed,
        mode,
    )
    .map_err(HubError::from)
}

#[cfg(test)]
mod tests;
