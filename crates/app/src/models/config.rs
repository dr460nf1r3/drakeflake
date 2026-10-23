use std::cell::RefCell;
use std::sync::Arc;

use drakeflake_config::Config;

use crate::hub::Hub;

#[derive(Default)]
pub struct SavedConfig {
    config: RefCell<Config>,
    hub: Arc<Hub>,
}

impl SavedConfig {
    pub const fn new(hub: Arc<Hub>, config: Config) -> Self {
        Self {
            config: RefCell::new(config),
            hub,
        }
    }

    pub fn read<T>(&self, part: impl FnOnce(&Config) -> T) -> T {
        part(&self.config.borrow())
    }

    pub fn update(&self, edit: impl FnOnce(&mut Config)) {
        let config = {
            let mut config = self.config.borrow_mut();
            edit(&mut config);
            config.clone()
        };

        match config.save() {
            Ok(()) => log::debug!("settings saved"),
            Err(err) => self.hub.log_error(format!("Cannot save settings: {err:#}")),
        }

        if std::env::var_os("DRAKEFLAKE_FLAKE").is_some() {
            self.hub.log(
                "Note: DRAKEFLAKE_FLAKE is set and overrides the saved flake on the next start.",
            );
        }

        if std::env::var_os("DRAKEFLAKE_MANAGED_FILE").is_some() {
            self.hub.log("Note: DRAKEFLAKE_MANAGED_FILE is set and overrides the saved output path on the next start.");
        }
    }
}
