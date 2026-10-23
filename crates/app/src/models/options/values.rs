use std::sync::Arc;

use drakeflake_render::OptionName;
use drakeflake_store::ValueSnapshot;
use drakeflake_validate::{display_value, parse_type, value_to_input};
use serde_json::{Value, json};

use super::rows::is_error;
use super::tree_job::BuiltTree;
use super::{OptionModel, View};
use crate::hub::{Generation, HomeEpoch, OptionsMsg};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Refresh {
    Nothing,
    Values,
    Search,
}

impl OptionModel {
    fn reevaluate_missing(&mut self, snapshot: &ValueSnapshot) {
        let missing: Vec<OptionName> = self
            .evaluated
            .keys()
            .filter(|name| snapshot.value(name).is_none())
            .cloned()
            .collect();

        self.evaluated.clear();

        if self.view == View::Configured {
            return;
        }

        for name in missing {
            log::debug!("{:?} options: evaluating {name} again", self.view);
            self.pending.borrow_mut().insert(name.clone());
            self.hub.request_value(name);
        }
    }

    pub(super) fn start_evaluation(&self, name: &OptionName) -> bool {
        if self.pending.borrow().contains(name) {
            return true;
        }

        if self.hub.target().is_none() {
            return false;
        }

        log::debug!("{:?} options: evaluating {name} on demand", self.view);
        self.pending.borrow_mut().insert(name.to_owned());
        self.hub.request_value(name.to_owned());
        true
    }

    fn on_value(
        &mut self,
        name: OptionName,
        result: Result<Value, String>,
        generation: Generation,
    ) -> Refresh {
        if !self.hub.is_current(generation) {
            log::debug!("{:?} options: stale value of {name} dropped", self.view);
            return Refresh::Nothing;
        }

        if let Err(error) = &result {
            log::debug!("value of {name} failed: {error}");
        }

        self.pending.borrow_mut().remove(&name);
        if self.view.is_tree() {
            self.changed_values.push(name.clone());
        }

        self.evaluated.insert(name, result);
        Refresh::Values
    }

    fn on_home_value(
        &mut self,
        name: OptionName,
        result: Result<Value, String>,
        generation: Generation,
        epoch: HomeEpoch,
    ) -> Refresh {
        if epoch != self.home_epoch {
            log::debug!(
                "{:?} options: value of {name} for another user dropped",
                self.view
            );
            return Refresh::Nothing;
        }

        self.on_value(name, result, generation)
    }

    fn reset(&mut self) -> Refresh {
        self.index = Arc::default();
        self.snapshot = Arc::default();
        self.has_snapshot = false;
        self.evaluated.clear();
        self.pending.borrow_mut().clear();
        self.index_error.clear();
        self.values_error.clear();
        Refresh::Search
    }

    pub(super) fn handle(&mut self, message: OptionsMsg) -> Refresh {
        match message {
            OptionsMsg::Reset => self.reset(),
            OptionsMsg::HomeReset { epoch } => {
                self.home_epoch = epoch;
                self.reset()
            }
            OptionsMsg::HomeValue {
                name,
                result,
                generation,
                epoch,
            } => self.on_home_value(name, result, generation, epoch),
            OptionsMsg::Index(index) => {
                log::debug!("{:?} options: index of {} options", self.view, index.len());
                self.index = index;
                self.index_error.clear();
                Refresh::Search
            }
            OptionsMsg::IndexError(error) => {
                log::warn!("{:?} options: no index: {error}", self.view);
                self.index_error = error;
                Refresh::Nothing
            }
            OptionsMsg::Snapshot(snapshot) => {
                log::debug!("{:?} options: values loaded", self.view);
                self.reevaluate_missing(&snapshot);
                self.snapshot = snapshot;
                self.has_snapshot = true;
                self.values_error.clear();

                if self.only_user || self.view.is_tree() {
                    Refresh::Search
                } else {
                    Refresh::Values
                }
            }
            OptionsMsg::SnapshotError(error) => {
                log::warn!("{:?} options: no values: {error}", self.view);
                self.values_error = error;
                Refresh::Nothing
            }
            OptionsMsg::Value(name, result, generation) => self.on_value(name, result, generation),
            OptionsMsg::TreeBuilt { number, tree } => {
                match tree.downcast::<BuiltTree>() {
                    Ok(built) => self.apply_built_tree(number, built),
                    Err(_) => log::error!("{:?}: tree build {number} has another type", self.view),
                }

                Refresh::Nothing
            }
        }
    }

    pub(super) fn details_value(&self, name: &OptionName) -> Value {
        let Some(info) = self.index.get(name) else {
            return json!({});
        };

        let ty = parse_type(&info.type_name);
        let (min, max) = ty.bounds();

        let (value_state, value_display, value_input, value_error) =
            match (self.evaluated.get(name), self.snapshot.value(name)) {
                (Some(Err(error)), _) => ("error", String::new(), None, error.clone()),
                (Some(Ok(value)), _) | (None, Some(value)) if is_error(value) => (
                    "error",
                    String::new(),
                    None,
                    "This option fails to evaluate in the current configuration".to_owned(),
                ),
                (Some(Ok(value)), _) | (None, Some(value)) => (
                    "ready",
                    display_value(value),
                    value_to_input(&ty, value),
                    String::new(),
                ),
                (None, None) if self.start_evaluation(name) => {
                    ("loading", String::new(), None, String::new())
                }
                (None, None) => ("unknown", String::new(), None, String::new()),
            };

        json!({
            "name": info.name,
            "description": info.description,
            "type": info.type_name,
            "kind": ty.kind(),
            "nullable": ty.nullable,
            "merges": ty.merges(),
            "choices": ty.choices(),
            "min": min,
            "max": max,
            "default": info.default_text,
            "example": info.example_text,
            "declarations": info.declarations,
            "readOnly": info.read_only,
            "templated": info.templated,
            "userFiles": self.snapshot.user_files(name),
            "locations": crate::hub::editor::locate(
                name,
                self.snapshot.user_files(name),
                self.hub.target().and_then(|target| target.local_dir()).as_deref(),
            ),
            "valueState": value_state,
            "valueDisplay": value_display,
            "valueInput": value_input,
            "valueError": value_error,
        })
    }
}
