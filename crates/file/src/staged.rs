mod persist;
mod settings;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use drakeflake_plan::{ManagedOption, ManagedState};
use drakeflake_render::{OptionName, PackageAttr};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use self::settings::{
    PriorityChoices, SettingsInput, StageError, parse_mode, priority_choices,
};
use crate::ManagedOptionExt as _;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Staged {
    Set(ManagedOption),
    Reset,
    Add,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckNeeded {
    pub key: String,
    pub nix_value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StagedChanges {
    options: BTreeMap<OptionName, Staged>,
    packages: BTreeMap<PackageAttr, Staged>,
}

impl StagedChanges {
    #[must_use = "only reads the state"]
    pub const fn options(&self) -> &BTreeMap<OptionName, Staged> {
        &self.options
    }

    #[must_use = "only reads the state"]
    pub const fn packages(&self) -> &BTreeMap<PackageAttr, Staged> {
        &self.packages
    }

    #[must_use = "only reads the state"]
    pub fn is_empty(&self) -> bool {
        self.options.is_empty() && self.packages.is_empty()
    }

    #[must_use = "only reads the state"]
    pub fn staged_option(&self, name: &OptionName) -> Option<&ManagedOption> {
        match self.options.get(name) {
            Some(Staged::Set(option)) => Some(option),
            _ => None,
        }
    }

    #[must_use = "only reads the state"]
    pub fn current_option<'a>(
        &'a self,
        name: &OptionName,
        managed: &'a ManagedState,
    ) -> Option<&'a ManagedOption> {
        self.staged_option(name)
            .or_else(|| managed.options.get(name))
    }

    pub fn stage_option(
        &mut self,
        name: &OptionName,
        option: ManagedOption,
        settings: &Value,
        managed: &ManagedState,
    ) -> Result<Option<CheckNeeded>, StageError> {
        if name.is_empty() {
            return Err(StageError::NoOption);
        }

        let option = SettingsInput::from_value(settings)?.apply(name, option)?;
        let nix_value = option.check_source().map_err(StageError::Option)?;
        let key = check_key(&option);

        log::debug!(
            "staging option {name} ({}, priority {}, host {:?})",
            option.mode,
            option.effective_priority(),
            option.host
        );
        self.options.insert(name.clone(), Staged::Set(option));
        self.normalize(managed);

        Ok(self
            .staged_option(name)
            .map(|_| CheckNeeded { key, nix_value }))
    }

    pub fn stage_reset(&mut self, name: &OptionName, managed: &ManagedState) {
        if managed.options.contains_key(name) {
            self.options.insert(name.clone(), Staged::Reset);
        } else {
            self.options.remove(name);
        }
    }

    pub fn stage_package(
        &mut self,
        attr: &str,
        add: bool,
        managed: &ManagedState,
    ) -> Result<(), StageError> {
        drakeflake_validate::parse_package(attr).map_err(StageError::Package)?;

        let attr = PackageAttr::from(attr.strip_prefix("pkgs.").unwrap_or(attr));
        self.packages
            .insert(attr, if add { Staged::Add } else { Staged::Remove });
        self.normalize(managed);
        Ok(())
    }

    pub fn unstage(&mut self, key: &str) -> bool {
        if let Some(name) = key.strip_prefix("option:") {
            self.options.remove(&OptionName::from(name));
        } else if let Some(attr) = key.strip_prefix("package:") {
            self.packages.remove(&PackageAttr::from(attr));
        } else {
            return false;
        }

        true
    }

    pub fn clear(&mut self) {
        self.options.clear();
        self.packages.clear();
    }

    #[must_use = "only builds the planned state"]
    pub fn plan(&self, managed: &ManagedState) -> ManagedState {
        let mut plan = managed.clone();

        for (name, staged) in &self.options {
            match staged {
                Staged::Set(option) => {
                    plan.options.insert(name.clone(), option.clone());
                }
                Staged::Reset => {
                    plan.options.remove(name);
                }
                Staged::Add | Staged::Remove => {}
            }
        }

        for (attr, staged) in &self.packages {
            match staged {
                Staged::Add => {
                    plan.packages.insert(attr.clone());
                }
                Staged::Remove => {
                    plan.packages.remove(attr);
                }
                Staged::Set(_) | Staged::Reset => {}
            }
        }

        plan
    }

    pub fn normalize(&mut self, managed: &ManagedState) {
        self.options.retain(|name, staged| match staged {
            Staged::Set(option) => managed.options.get(name) != Some(option),
            Staged::Reset => managed.options.contains_key(name),
            Staged::Add | Staged::Remove => false,
        });

        self.packages.retain(|attr, staged| match staged {
            Staged::Add => !managed.packages.contains(attr),
            Staged::Remove => managed.packages.contains(attr),
            Staged::Set(_) | Staged::Reset => false,
        });
    }

    pub fn merge(
        &mut self,
        restored: Self,
        managed: &ManagedState,
    ) -> Vec<(OptionName, CheckNeeded)> {
        let mut checks = Vec::new();

        for (name, staged) in restored.options {
            let Entry::Vacant(slot) = self.options.entry(name.clone()) else {
                continue;
            };

            if let Staged::Set(option) = &staged {
                match option.check_source() {
                    Ok(nix_value) => {
                        checks.push((
                            name,
                            CheckNeeded {
                                key: check_key(option),
                                nix_value,
                            },
                        ));
                    }
                    Err(error) => {
                        log::warn!("restored value of {name} needs a re-check: {error}");
                        checks.push((
                            name,
                            CheckNeeded {
                                key: check_key(option),
                                nix_value: String::new(),
                            },
                        ));
                    }
                }
            }

            slot.insert(staged);
        }

        for (attr, staged) in restored.packages {
            self.packages.entry(attr).or_insert(staged);
        }

        // Restored values may equal what the file holds now (an external edit,
        // or a reload while staged): drop the phantom entries.
        self.normalize(managed);

        checks
    }

    #[must_use = "only reads the state"]
    pub fn option_state(&self, name: &OptionName, managed: &ManagedState) -> &'static str {
        match self.options.get(name) {
            Some(Staged::Set(_)) => "set",
            Some(Staged::Reset) => "reset",
            _ if managed.options.contains_key(name) => "managed",
            _ => "",
        }
    }

    #[must_use = "only reads the state"]
    pub fn package_state(&self, attr: &PackageAttr, managed: &ManagedState) -> &'static str {
        match self.packages.get(attr) {
            Some(Staged::Add) => "add",
            Some(Staged::Remove) => "remove",
            _ if managed.packages.contains(attr) => "managed",
            _ => "",
        }
    }
}

/// Priority and host scope do not change the value, so they stay out of the key.
#[must_use = "only builds the key"]
pub fn check_key(option: &ManagedOption) -> String {
    format!("{}:{}", option.mode, option.input)
}
