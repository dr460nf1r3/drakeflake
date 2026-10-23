mod messages;
mod runs;
mod target;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use qtbridge::qobject;
use runs::RunState;
use serde_json::{Value, json};
use target::TargetState;

use crate::hub::{Hub, Warning};
use crate::models::config::SavedConfig;
use crate::models::relay::Relay;

/// Bumped whenever QML starts using new slots or properties of the models.
const API_VERSION: i32 = 32;

#[derive(Default)]
pub struct SystemModel {
    hub: Arc<Hub>,
    relay: Arc<Relay>,
    config: Rc<SavedConfig>,
    target: TargetState,
    run: RunState,
    activities: Vec<String>,
    warnings: Vec<Warning>,
    home_users: Vec<String>,
    home_user: String,
    home_check: Option<String>,
    ui_ready: bool,
}

impl SystemModel {
    pub fn new(hub: Arc<Hub>, relay: Arc<Relay>, config: Rc<SavedConfig>) -> Self {
        let target = config.read(|config| TargetState::from_config(&config.nix));

        Self {
            hub,
            relay,
            config,
            target,
            ..Self::default()
        }
    }
}

#[qobject]
impl SystemModel {
    qproperty!("flake", Read = flake, Notify = state_changed);
    qproperty!("host", Read = host, Notify = state_changed);
    qproperty!("kind", Read = kind, Notify = state_changed);
    qproperty!("hosts", Read = hosts, Notify = state_changed);
    qproperty!("capabilities", Read = capabilities, Notify = state_changed);
    qproperty!("managed_path", Read = managed_path, Notify = state_changed);
    qproperty!(
        "managed_setting",
        Read = managed_setting,
        Notify = state_changed
    );
    qproperty!("target_error", Read = target_error, Notify = state_changed);
    qproperty!("target_ready", Read = target_ready, Notify = state_changed);
    qproperty!("running", Read = running, Notify = state_changed);
    qproperty!("last_result", Read = last_result, Notify = state_changed);
    qproperty!("can_restore", Read = can_restore, Notify = state_changed);
    qproperty!("activity", Read = activity, Notify = state_changed);
    qproperty!("busy", Read = busy, Notify = state_changed);
    qproperty!("warnings", Read = warnings, Notify = state_changed);
    qproperty!("home_users", Read = home_users, Notify = state_changed);
    qproperty!("home_user", Read = home_user, Notify = state_changed);
    qproperty!("home_state", Read = home_state, Notify = state_changed);

    pub fn flake(&self) -> String {
        self.target.flake.clone()
    }

    pub fn host(&self) -> String {
        self.target.host.clone()
    }

    pub fn kind(&self) -> String {
        self.target
            .kind
            .map_or("", drakeflake_flake::SystemKind::as_str)
            .to_owned()
    }

    pub fn hosts(&self) -> Value {
        self.target.hosts_json()
    }

    pub fn capabilities(&self) -> Value {
        self.target.capabilities().to_json()
    }

    pub fn managed_path(&self) -> String {
        self.target.managed_path.clone()
    }

    pub fn managed_setting(&self) -> String {
        self.target.managed_setting.clone()
    }

    pub fn target_error(&self) -> String {
        self.target.error.clone()
    }

    pub const fn target_ready(&self) -> bool {
        self.target.ready
    }

    pub const fn running(&self) -> bool {
        self.run.running
    }

    pub fn last_result(&self) -> String {
        self.run.last_result.to_string()
    }

    pub fn can_restore(&self) -> bool {
        self.run.can_restore()
    }

    pub fn activity(&self) -> String {
        self.activities.join("\n")
    }

    pub const fn busy(&self) -> bool {
        !self.activities.is_empty()
    }

    pub fn warnings(&self) -> Value {
        json!(self.warnings)
    }

    pub fn home_users(&self) -> Value {
        json!(self.home_users)
    }

    pub fn home_user(&self) -> String {
        self.home_user.clone()
    }

    pub fn home_state(&self) -> String {
        match &self.home_check {
            None => String::new(),
            Some(error) if !error.is_empty() => format!("error: {error}"),
            Some(_) if self.home_users.is_empty() => "none".to_owned(),
            Some(_) => "available".to_owned(),
        }
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let notes = self.relay.system.take();
        let messages = self.hub.mail().system.take();

        if notes.is_empty() && messages.is_empty() {
            return;
        }

        log::trace!(
            "system mailbox: {} messages, {} notes",
            messages.len(),
            notes.len()
        );

        // Notes first: they were posted by slots that ran before the hub's answers.
        for note in notes {
            self.handle_note(note);
        }

        for message in messages {
            self.handle(message);
        }

        self.state_changed();
    }

    #[qslot]
    pub const fn ui_ready(&mut self) {
        self.ui_ready = true;
    }

    #[qslot]
    pub fn start(&self) {
        self.load_configured();
    }

    #[qslot]
    pub fn retarget(&mut self, flake: String, host: String, managed: String) {
        self.switch_target(flake.trim(), host.trim(), managed.trim());
        self.state_changed();
    }

    #[qslot]
    pub fn reload(&mut self) {
        self.reload_target(false);
        self.state_changed();
    }

    #[qslot]
    pub fn rebuild_caches(&mut self) {
        self.reload_target(true);
        self.state_changed();
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub const fn api_version(&self) -> i32 {
        API_VERSION
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn selftest(&self, name: String) -> String {
        std::env::var(format!("DRAKEFLAKE_SELFTEST_{}", name.to_uppercase())).unwrap_or_default()
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn set_window_dirty(&self, dirty: bool) {
        crate::macos::window::set_window_dirty(dirty);
    }

    #[qslot]
    pub fn cancel(&self) {
        log::info!("cancel requested");
        self.hub.cancel_apply();
    }

    #[qslot]
    pub fn select_home_user(&mut self, user: String) {
        if !self.home_users.contains(&user) {
            log::warn!("ignoring unknown home manager user {user:?}");
            return;
        }

        if user != self.home_user {
            log::info!("home manager user selected: {user}");
            self.home_user.clone_from(&user);
            self.hub.select_home_user(user);
            self.state_changed();
        }
    }

    #[qslot]
    pub fn track_in_git(&self) {
        log::info!("adding the generated files to git");
        self.hub.track_in_git();
    }

    #[qslot]
    pub fn apply(&mut self, plan: Value, mode: String) {
        if let Some((plan, mode)) = self.parse_apply(&plan, &mode, |can| can.apply) {
            let result = self.hub.apply(plan, mode);
            self.run_started(result);
        }

        self.state_changed();
    }

    #[qslot]
    pub fn review_apply(&mut self, plan: Value, mode: String) {
        if let Some((plan, mode)) = self.parse_apply(&plan, &mode, |can| can.review) {
            let result = self.hub.review_apply(plan, mode);
            self.run_started(result);
            self.state_changed();
        }
    }

    #[qslot]
    pub fn rebuild(&mut self, mode: String) -> String {
        self.start_rebuild(&mode)
    }

    #[qslot]
    pub fn update_lock(&mut self, names: Value) -> String {
        self.start_lock_update(&names)
    }

    #[qslot]
    pub fn restore_backup(&mut self) {
        self.start_restore_backup();
    }

    #[qslot]
    pub fn restore_history(&mut self, file: String) {
        log::info!("restoring {file}");
        let result = self.hub.restore(PathBuf::from(file));
        self.run_started(result);
        self.state_changed();
    }

    #[qslot]
    pub fn generation_action(&mut self, id: i32, action: String) {
        self.start_generation_action(id, &action);
    }

    #[qslot]
    pub fn clean_up(&mut self, keep: i32, days: i32, optimise: bool) {
        let keep = u32::try_from(keep).unwrap_or(1);
        let days = u32::try_from(days).unwrap_or(0);
        let result = self
            .hub
            .clean_up(keep, days, optimise.into())
            .inspect(|()| {
                log::info!("clean-up started: keep {keep}, {days} days, optimise {optimise}");
            });

        self.run_started(result);
        self.state_changed();
    }

    #[qslot]
    pub fn build_image(&mut self, host: String, format: String) {
        let result = self
            .hub
            .build_image(&host, format)
            .inspect(|()| log::info!("image build of {host:?} started"));

        self.run_started(result);
        self.state_changed();
    }
}
