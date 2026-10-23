mod check;
mod json;
mod persist;
mod preview;
mod rows;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;

use check::CheckState;
use drakeflake_file::StagedChanges;
use drakeflake_plan::{ManagedOption, ManagedState};
use drakeflake_render::OptionName;
use drakeflake_validate::{parse_input, parse_type};
use qtbridge::qobject;
use rows::ChangeRow;
use serde_json::{Value, json};

use crate::hub::Hub;
use crate::models::count_i32;
use crate::models::vec_model::vec_list_model;

#[derive(Default)]
pub struct ChangeModel {
    hub: Arc<Hub>,
    managed: ManagedState,
    managed_error: String,
    loaded: bool,
    changes: StagedChanges,
    saved: StagedChanges,
    checks: HashMap<OptionName, (String, CheckState)>,
    rows: Vec<ChangeRow>,
    /// Saving before the saved edits are restored would overwrite them.
    restored: bool,
    revision: i32,
}

vec_list_model!(ChangeModel, ChangeRow, rows, notify = changed);

impl ChangeModel {
    pub fn new(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            ..Self::default()
        }
    }

    pub fn plan(&self) -> ManagedState {
        self.changes.plan(&self.managed)
    }

    fn normalize(&mut self) {
        self.changes.normalize(&self.managed);

        let changes = &self.changes;
        self.checks.retain(|name, (input, _)| {
            changes
                .staged_option(name)
                .is_some_and(|option| drakeflake_file::check_key(option) == *input)
        });
    }
}

#[qobject(Base = QListModel)]
impl ChangeModel {
    qproperty!("count", Read = row_count, Notify = changed);
    qproperty!("managed_count", Read = managed_count, Notify = changed);
    qproperty!("error", Read = error, Notify = changed);
    qproperty!("ready", Read = ready, Notify = changed);
    qproperty!("revision", Read = revision, Notify = changed);
    qproperty!("checks_pending", Read = checks_pending, Notify = changed);
    qproperty!("checks_failed", Read = checks_failed, Notify = changed);

    pub fn checks_pending(&self) -> i32 {
        count_i32(self.count_pending())
    }

    pub fn checks_failed(&self) -> i32 {
        count_i32(self.count_failed())
    }

    pub fn row_count(&self) -> i32 {
        count_i32(self.rows.len())
    }

    pub fn managed_count(&self) -> i32 {
        count_i32(self.managed.options.len() + self.managed.packages.len())
    }

    pub fn error(&self) -> String {
        self.managed_error.clone()
    }

    pub const fn ready(&self) -> bool {
        self.loaded && self.managed_error.is_empty()
    }

    pub const fn revision(&self) -> i32 {
        self.revision
    }

    #[qsignal]
    fn changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let messages = self.hub.mail().changes.take();

        if messages.is_empty() {
            return;
        }

        log::debug!("changes mailbox: {} messages", messages.len());

        for message in messages {
            self.handle(message);
        }

        self.normalize();
        self.rebuild_rows();
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn validate(&self, type_name: String, input: String) -> String {
        parse_input(&parse_type(&type_name), &input)
            .err()
            .map_or_else(String::new, |error| error.to_string())
    }

    #[qslot]
    pub fn stage_option(&mut self, name: String, type_name: String, input: String) -> String {
        self.stage_setting(name, type_name, input, Value::Null)
    }

    #[qslot]
    pub fn stage_setting(
        &mut self,
        name: String,
        type_name: String,
        input: String,
        settings: Value,
    ) -> String {
        let name = OptionName::from(name);
        let staged = self.changes.stage_option(
            &name,
            ManagedOption::typed(&type_name, &input),
            &settings,
            &self.managed,
        );

        match staged {
            Ok(needed) => {
                self.normalize();

                if let Some(needed) = needed
                    && self
                        .checks
                        .get(&name)
                        .is_none_or(|(checked, _)| *checked != needed.key)
                {
                    self.request_check(&name, needed.key, needed.nix_value);
                }

                self.rebuild_rows();
                String::new()
            }
            Err(error) => {
                log::debug!("not staging {name}: {error}");
                error.to_string()
            }
        }
    }

    #[qslot]
    pub fn stage_reset(&mut self, name: String) {
        log::debug!("staging reset of {name}");

        self.changes.stage_reset(&name.into(), &self.managed);
        self.normalize();
        self.rebuild_rows();
    }

    #[qslot]
    pub fn stage_package(&mut self, attr: String, add: bool) -> String {
        log::debug!(
            "staging {} of package {attr}",
            if add { "add" } else { "remove" }
        );

        match self.changes.stage_package(&attr, add, &self.managed) {
            Ok(()) => {
                self.rebuild_rows();
                String::new()
            }
            Err(error) => {
                log::warn!("not staging package {attr:?}: {error}");
                error.to_string()
            }
        }
    }

    #[qslot]
    pub fn unstage(&mut self, key: String) {
        if self.changes.unstage(&key) {
            log::debug!("unstaged {key}");
        } else {
            log::warn!("ignoring unknown change key {key:?}");
        }

        self.normalize();
        self.rebuild_rows();
    }

    #[qslot]
    pub fn clear(&mut self) {
        log::info!(
            "unstaged all: {} options, {} packages",
            self.changes.options().len(),
            self.changes.packages().len()
        );
        self.changes.clear();
        self.normalize();
        self.rebuild_rows();
    }

    #[qslot]
    pub fn check_status(&self, name: String) -> String {
        self.check_of(&name.into())
            .map(ToString::to_string)
            .unwrap_or_default()
    }

    #[qslot]
    pub fn check_message(&self, name: String) -> String {
        self.check_of(&name.into())
            .map(CheckState::message)
            .unwrap_or_default()
    }

    #[qslot]
    pub fn option_state(&self, name: String) -> String {
        self.changes
            .option_state(&name.into(), &self.managed)
            .to_owned()
    }

    #[qslot]
    pub fn staged_input(&self, name: String) -> String {
        self.changes
            .current_option(&name.into(), &self.managed)
            .map(|option| option.input.clone())
            .unwrap_or_default()
    }

    #[qslot]
    pub fn staged_settings(&self, name: String) -> Value {
        self.changes
            .current_option(&name.into(), &self.managed)
            .map_or_else(|| json!({}), json::settings_value)
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn priority_choices(&self, type_name: String, input: String, mode: String) -> Value {
        json::priority_choices_value(&type_name, &input, &mode)
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn host_scope_allowed(&self, name: String) -> bool {
        drakeflake_file::host_scope_allowed(&name)
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn validate_expression(&self, input: String) -> String {
        drakeflake_validate::parse_expression(&input)
            .err()
            .map_or_else(String::new, |error| error.to_string())
    }

    #[qslot]
    pub fn package_state(&self, attr: String) -> String {
        self.changes
            .package_state(&attr.into(), &self.managed)
            .to_owned()
    }

    #[qslot]
    pub fn plan_json(&self) -> Value {
        json!(self.plan())
    }

    #[qslot]
    pub fn saved_json(&self) -> Value {
        json!(self.managed)
    }

    #[qslot]
    pub fn managed_list(&self) -> Value {
        self.managed_rows()
    }

    #[qslot]
    pub fn preview(&self, plan: Value) -> String {
        preview::render(&self.hub, &plan)
    }
}
