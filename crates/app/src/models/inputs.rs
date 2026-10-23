use std::collections::HashMap;
use std::sync::Arc;

use drakeflake_flake::lock::{LockedInput, UpdateCheck};
use qtbridge::qobject;
use serde::Serialize;
use serde_json::{Value, json};

use crate::hub::{Hub, InputsMsg};
use crate::models::relay::{Relay, SystemNote};

#[derive(Serialize)]
struct InputRow<'a> {
    #[serde(flatten)]
    input: &'a LockedInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    check: Option<&'a UpdateCheck>,
}

#[derive(Default)]
pub struct InputsModel {
    hub: Arc<Hub>,
    relay: Arc<Relay>,
    flake: String,
    inputs: Vec<LockedInput>,
    checks: HashMap<String, UpdateCheck>,
    error: String,
    checking: bool,
}

impl InputsModel {
    pub fn new(hub: Arc<Hub>, relay: Arc<Relay>, flake: String) -> Self {
        Self {
            hub,
            relay,
            flake,
            ..Self::default()
        }
    }

    fn rows(&self) -> Vec<InputRow<'_>> {
        self.inputs
            .iter()
            .map(|input| InputRow {
                input,
                check: self.checks.get(&input.name),
            })
            .collect()
    }

    fn handle(&mut self, message: InputsMsg) {
        match message {
            InputsMsg::Flake(flake) => {
                self.checking = false;
                self.flake = flake;
            }
            InputsMsg::Lock(result) => self.on_lock(result),
            InputsMsg::Checks { flake, checks } if flake == self.flake => {
                self.checks = checks.into_iter().collect();
                self.checking = false;
                log::debug!("upstream checks of {} inputs done", self.checks.len());
            }
            InputsMsg::Checks { flake, checks: _ } => {
                log::debug!("dropping the upstream checks of {flake}: another flake is shown");
            }
            InputsMsg::Updated(ok) => self.on_updated(ok),
        }
    }

    fn on_lock(&mut self, result: Result<Vec<LockedInput>, String>) {
        match result {
            Ok(inputs) => {
                log::debug!("flake.lock read: {} inputs", inputs.len());
                self.inputs = inputs;
                self.error.clear();
            }
            Err(error) => {
                log::debug!("flake.lock error shown: {error}");
                self.inputs.clear();
                self.error = error;
                self.checking = false;
            }
        }

        self.checks.clear();
    }

    fn on_updated(&mut self, ok: bool) {
        if !ok {
            log::debug!("flake.lock update failed; keeping the old view");
            return;
        }

        self.hub
            .log("flake.lock updated. Rebuild to start using the new versions.");
        self.checking = true;
        self.hub.load_lock(self.flake.clone(), true);
        self.relay.system.post(SystemNote::Reload);
    }
}

#[qobject]
impl InputsModel {
    qproperty!("inputs", Read = inputs, Notify = state_changed);
    qproperty!("error", Read = error, Notify = state_changed);
    qproperty!("checking", Read = checking, Notify = state_changed);

    pub fn inputs(&self) -> Value {
        json!(self.rows())
    }

    pub fn error(&self) -> String {
        self.error.clone()
    }

    pub const fn checking(&self) -> bool {
        self.checking
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let messages = self.hub.mail().inputs.take();

        if messages.is_empty() {
            return;
        }

        for message in messages {
            self.handle(message);
        }

        self.state_changed();
    }

    #[qslot]
    pub fn load(&mut self, check: bool) {
        log::debug!("reading flake.lock (check upstream: {check})");
        self.checking = check;
        self.hub.load_lock(self.flake.clone(), check);
        self.state_changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_hand(model: &InputsModel) -> Value {
        Value::Array(
            model
                .inputs
                .iter()
                .map(|input| {
                    let mut row = json!(input);
                    if let (Some(object), Some(check)) =
                        (row.as_object_mut(), model.checks.get(&input.name))
                    {
                        object.insert("check".to_owned(), json!(check));
                    }

                    row
                })
                .collect(),
        )
    }

    #[test]
    fn rows_keep_their_json_shape() {
        let mut model = InputsModel {
            inputs: vec![
                LockedInput {
                    name: "nixpkgs".to_owned(),
                    ..LockedInput::default()
                },
                LockedInput {
                    name: "home-manager".to_owned(),
                    ..LockedInput::default()
                },
            ],
            ..InputsModel::default()
        };

        model.checks.insert(
            "nixpkgs".to_owned(),
            UpdateCheck::Failed {
                error: "offline".to_owned(),
            },
        );

        let rows = model.inputs();
        assert_eq!(rows, by_hand(&model));
        assert!(rows[0].get("check").is_some() && rows[1].get("check").is_none());
    }

    #[test]
    fn a_new_lock_drops_the_old_checks() {
        let mut model = InputsModel::default();
        let input = LockedInput {
            name: "nixpkgs".to_owned(),
            ..LockedInput::default()
        };

        let checked = || checked_for("");

        model.checking = true;
        model.handle(InputsMsg::Lock(Ok(vec![input.clone()])));
        model.handle(checked());
        assert!(!model.checking);
        assert!(model.inputs()[0].get("check").is_some());

        model.handle(InputsMsg::Updated(false));
        assert!(model.inputs()[0].get("check").is_some());

        model.handle(InputsMsg::Lock(Ok(vec![input])));
        assert!(model.inputs()[0].get("check").is_none());

        model.handle(checked());
        model.checking = true;
        model.handle(InputsMsg::Lock(Err("no flake.lock".to_owned())));
        assert_eq!(
            (model.inputs(), model.error(), model.checking()),
            (json!([]), "no flake.lock".to_owned(), false)
        );
        assert_eq!(model.checks, HashMap::new());
    }

    #[test]
    fn checks_of_another_flake_are_dropped() {
        let mut model = InputsModel::default();

        model.handle(InputsMsg::Flake("/new".to_owned()));
        model.checking = true;
        model.handle(checked_for("/old"));
        assert_eq!((model.checks.len(), model.checking), (0, true));

        model.handle(checked_for("/new"));
        assert_eq!((model.checks.len(), model.checking), (1, false));

        model.checking = true;
        model.handle(InputsMsg::Flake("/other".to_owned()));
        assert!(!model.checking);
    }

    fn checked_for(flake: &str) -> InputsMsg {
        InputsMsg::Checks {
            flake: flake.to_owned(),
            checks: vec![("nixpkgs".to_owned(), UpdateCheck::UpToDate)],
        }
    }
}
