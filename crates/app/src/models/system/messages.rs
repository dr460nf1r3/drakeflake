use super::SystemModel;
use crate::hub::SystemMsg;
use crate::models::relay::SystemNote;

impl SystemModel {
    pub(super) fn handle(&mut self, message: SystemMsg) {
        match message {
            SystemMsg::Target(info) => self.on_target(info),
            SystemMsg::TargetError {
                flake,
                hosts,
                error,
            } => self.on_target_error(flake, hosts, error),
            SystemMsg::Running(running) => self.run.running = running,
            SystemMsg::Finished { ok, backup, commit } => self.run.finished(ok, backup, commit),
            SystemMsg::Activities(list) => self.activities = list,
            SystemMsg::Warnings(list) => {
                if log::log_enabled!(log::Level::Debug) {
                    let ids: Vec<&str> = list.iter().map(|warning| warning.id.as_str()).collect();
                    log::debug!("warnings: {ids:?}");
                }

                self.warnings = list;
            }
            SystemMsg::HomeUsers {
                users,
                selected,
                error,
            } => {
                log::debug!("home manager users {users:?}, selected {selected:?}");
                self.home_users = users.into_iter().map(String::from).collect();
                self.home_user = String::from(selected);
                self.home_check = Some(error.unwrap_or_default());
            }
            SystemMsg::CheckUi => self.check_ui(),
        }
    }

    pub(super) fn handle_note(&mut self, note: SystemNote) {
        match note {
            SystemNote::Reload => self.reload_target(false),
            SystemNote::LayoutChanged => {
                if self.target.ready {
                    self.hub
                        .set_managed(self.target.setting(), self.target.hosts.clone());
                }
            }
        }
    }

    fn check_ui(&self) {
        if self.ui_ready {
            return;
        }

        // Source-tree runs outside the dev shell have no QML modules.
        let hint = if std::env::var_os("DRAKEFLAKE_QML_DIR").is_none()
            && std::env::var_os("QML_IMPORT_PATH").is_none()
            && std::env::var_os("QML2_IMPORT_PATH").is_none()
        {
            " (QML errors above; run inside nix develop (start-dev) or set QML_IMPORT_PATH)"
        } else {
            " (QML errors above; set QT_FORCE_STDERR_LOGGING=1 to see them)"
        };

        crate::fail_startup("the user interface failed to load", hint, 2);
    }
}
