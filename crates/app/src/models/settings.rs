#[cfg(test)]
mod tests;
pub mod tray;

use std::rc::Rc;
use std::sync::Arc;

use qtbridge::qobject;

use crate::hub::Hub;
use crate::models::config::SavedConfig;
use crate::models::open_folder;
use crate::models::relay::{Relay, SystemNote};
use crate::window_state::{Geometry, WindowState, geometry_valid};

#[derive(Default)]
pub struct SettingsModel {
    hub: Arc<Hub>,
    relay: Arc<Relay>,
    config: Rc<SavedConfig>,
    window: WindowState,
    desktop_theme: String,
    global_menu: bool,
    tray_autostart: bool,
    version: String,
}

impl SettingsModel {
    pub fn new(hub: Arc<Hub>, relay: Arc<Relay>, config: Rc<SavedConfig>) -> Self {
        hub.set_layout(config.read(|config| config.layout.clone()));
        hub.set_auto_commit(config.read(|config| config.general.auto_commit));

        Self {
            hub,
            relay,
            config,
            window: WindowState::load(),
            desktop_theme: drakeflake_config::desktop_theme_name().unwrap_or_default(),
            global_menu: crate::global_menu::available(),
            tray_autostart: tray::autostart_file().is_some_and(|file| file.is_file()),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

#[qobject]
impl SettingsModel {
    qproperty!("layout_mode", Read = layout_mode, Notify = state_changed);
    qproperty!("layout_rules", Read = layout_rules, Notify = state_changed);
    qproperty!(
        "highlight_theme",
        Read = highlight_theme,
        Notify = state_changed
    );
    qproperty!("blur", Read = blur, Notify = state_changed);
    qproperty!(
        "blur_effective",
        Read = blur_effective,
        Notify = state_changed
    );
    qproperty!("log_level", Read = log_level, Notify = state_changed);
    qproperty!("auto_commit", Read = auto_commit, Notify = state_changed);
    qproperty!(
        "tray_autostart",
        Read = tray_autostart,
        Notify = state_changed
    );
    qproperty!(
        "desktop_theme",
        Read = desktop_theme,
        Notify = state_changed
    );
    qproperty!("global_menu", Read = global_menu, Notify = state_changed);
    qproperty!("version", Read = version, Notify = state_changed);
    qproperty!(
        "tray_settings",
        Read = tray_settings,
        Notify = state_changed
    );
    qproperty!(
        "window_geometry",
        Read = window_geometry,
        Notify = state_changed
    );

    pub fn layout_mode(&self) -> String {
        self.config.read(|config| config.layout.mode.to_string())
    }

    pub fn layout_rules(&self) -> String {
        self.config.read(|config| config.layout.rules_text())
    }

    pub fn highlight_theme(&self) -> String {
        self.config
            .read(|config| config.general.highlight_theme.clone())
    }

    pub fn blur(&self) -> String {
        self.config
            .read(|config| config.general.blur.as_str().to_owned())
    }

    pub fn blur_effective(&self) -> bool {
        self.config
            .read(|config| drakeflake_config::effective_blur(config.general.blur))
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn blur_env(&self) -> String {
        drakeflake_config::blur_env_override()
            .map_or_else(String::new, |mode| mode.as_str().to_owned())
    }

    pub fn log_level(&self) -> String {
        self.config.read(|config| config.general.log_level.clone())
    }

    pub fn auto_commit(&self) -> bool {
        self.config.read(|config| config.general.auto_commit)
    }

    pub const fn tray_autostart(&self) -> bool {
        self.tray_autostart
    }

    pub fn desktop_theme(&self) -> String {
        self.desktop_theme.clone()
    }

    pub const fn global_menu(&self) -> bool {
        self.global_menu
    }

    pub fn tray_settings(&self) -> serde_json::Value {
        self.config.read(|config| serde_json::json!(config.tray))
    }

    pub fn version(&self) -> String {
        self.version.clone()
    }

    pub fn window_geometry(&self) -> serde_json::Value {
        let geometry = self.window.geometry();

        serde_json::json!({
            "x": geometry.x,
            "y": geometry.y,
            "width": geometry.width,
            "height": geometry.height,
        })
    }

    #[qslot]
    pub fn save_window_geometry(&mut self, x: i32, y: i32, width: i32, height: i32) {
        if !geometry_valid(x, y, width, height) {
            log::warn!("ignoring window geometry {width}x{height} at {x},{y}");
            return;
        }

        log::debug!("window geometry {width}x{height} at {x},{y}");
        if let Err(err) = self.window.save(Geometry {
            x: Some(x),
            y: Some(y),
            width: Some(width),
            height: Some(height),
        }) {
            self.hub
                .log_error(format!("Cannot save the window geometry: {err}"));
        }

        self.state_changed();
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn set_layout(&mut self, mode: String, rules: String) -> String {
        let Ok(parsed) = mode.parse::<drakeflake_config::LayoutMode>() else {
            log::warn!("ignoring unknown layout {mode:?}");
            return format!("Unknown layout \"{mode}\"");
        };

        let rules = match drakeflake_config::LayoutConfig::parse_rules(&rules) {
            Ok(rules) => rules,
            Err(error) => {
                log::warn!("ignoring invalid layout rules: {error}");
                return error;
            }
        };

        log::info!("layout set to {mode} with {} rules", rules.len());
        let layout = drakeflake_config::LayoutConfig {
            mode: parsed,
            rules,
        };

        self.hub.set_layout(layout.clone());
        self.relay.system.post(SystemNote::LayoutChanged);
        self.config.update(|config| config.layout = layout);
        self.state_changed();
        String::new()
    }

    #[qslot]
    pub fn set_highlight_theme(&mut self, name: String) {
        if name != self.highlight_theme() {
            log::info!("highlight theme set to {name:?}");
            self.config
                .update(|config| config.general.highlight_theme = name);
            self.state_changed();
        }
    }

    #[qslot]
    pub fn set_blur(&mut self, mode: String) -> String {
        let Ok(parsed) = mode.parse::<drakeflake_config::BlurMode>() else {
            log::warn!("ignoring unknown blur mode {mode:?}");
            return format!("Unknown blur mode \"{mode}\"");
        };

        if parsed == self.config.read(|config| config.general.blur) {
            return String::new();
        }

        log::info!("blur set to {}", parsed.as_str());
        self.config.update(|config| config.general.blur = parsed);
        let resolved = drakeflake_config::blur_env_override().unwrap_or(parsed);
        crate::blur::refresh(resolved.into());
        self.state_changed();
        String::new()
    }

    #[qslot]
    pub fn set_tray_report(&mut self, name: String, enabled: bool) {
        if self.edit_tray_settings(|tray| tray.set(&name, enabled)) {
            log::info!("tray report {name} {}", if enabled { "on" } else { "off" });
        } else {
            log::warn!("ignoring unknown tray report {name:?}");
        }

        self.state_changed();
    }

    #[qslot]
    pub fn set_tray_interval(&mut self, name: String, value: i32) -> String {
        let accepted = u32::try_from(value)
            .is_ok_and(|value| self.edit_tray_settings(|tray| tray.set_interval(&name, value)));

        self.state_changed();
        if accepted {
            log::info!("tray interval {name} = {value}");
            String::new()
        } else {
            log::warn!("ignoring tray interval {name} = {value}");
            format!("{value} is not allowed for {name}")
        }
    }

    #[qslot]
    pub fn set_auto_commit(&mut self, enabled: bool) {
        if enabled == self.auto_commit() {
            return;
        }

        log::info!("automatic commit {}", if enabled { "on" } else { "off" });
        self.hub.set_auto_commit(enabled);
        self.config
            .update(|config| config.general.auto_commit = enabled);
        self.state_changed();
    }

    #[qslot]
    pub fn set_log_level(&mut self, level: String) {
        if level == self.log_level() {
            return;
        }

        if drakeflake_log::parse_level(&level).is_none() {
            log::warn!("ignoring unknown log level {level:?}");
            return;
        }

        drakeflake_log::set_level(&level);
        log::info!("log level set to {level}");
        self.config
            .update(|config| config.general.log_level = level);
        self.state_changed();
    }

    #[qslot]
    #[expect(clippy::unused_self, reason = "qtbridge slots are methods")]
    pub fn log_file(&self) -> String {
        drakeflake_log::log_file(crate::PROGRAM)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    #[qslot]
    pub fn open_log(&self) {
        log::logger().flush();

        let opened = drakeflake_log::log_file(crate::PROGRAM)
            .map_err(|error| error.to_string())
            .and_then(|path| crate::hub::editor::open_own_file(self.hub.nix().context(), &path));

        match opened {
            Ok(done) => self.hub.log(done),
            Err(error) => self.hub.log_error(error),
        }
    }

    #[qslot]
    pub fn show_log_folder(&self) {
        let opened = drakeflake_log::log_dir()
            .map_err(|error| error.to_string())
            .and_then(|folder| open_folder(folder.to_string_lossy().into_owned()));

        if let Err(error) = opened {
            self.hub
                .log_error(format!("Could not open the log folder: {error}"));
        }
    }

    #[qslot]
    pub fn start_tray(&self) {
        if drakeflake_config::tray_running() {
            self.hub.log("The tray icon is already running.");
            return;
        }

        if let Err(error) = tray::start() {
            self.hub.log_error(error);
        }
    }

    #[qslot]
    pub fn set_tray_autostart(&mut self, enabled: bool) {
        let Some(file) = tray::autostart_file() else {
            log::warn!("no home or config directory; tray autostart unchanged");
            return;
        };

        if let Err(error) = tray::set_autostart(&file, enabled) {
            self.hub.log_error(error);
        }

        self.tray_autostart = file.is_file();
        self.state_changed();
    }
}
