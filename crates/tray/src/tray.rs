use std::sync::mpsc;

use drakeflake_tray_core::{Request, State, menu_lines, open_link, open_manager};
use ksni::menu::StandardItem;
use ksni::{MenuItem, Status, ToolTip};

const ICONS: [(&str, &str); 2] = [
    (
        "drakeflake-tray.svg",
        include_str!("../../../data/icons/drakeflake-tray.svg"),
    ),
    (
        "drakeflake-tray-update.svg",
        include_str!("../../../data/icons/drakeflake-tray-update.svg"),
    ),
];

pub struct Tray {
    pub state: State,
    pub requests: mpsc::Sender<Request>,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "drakeflake".to_owned()
    }

    /// The user's theme lacks our icons unless the package is installed system-wide.
    fn icon_theme_path(&self) -> String {
        icon_dir()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        open_manager();
    }

    fn title(&self) -> String {
        "DrakeFlake".to_owned()
    }

    fn status(&self) -> Status {
        if self.state.needs_attention() {
            Status::NeedsAttention
        } else {
            Status::Passive
        }
    }

    fn icon_name(&self) -> String {
        if self.state.needs_attention() {
            "drakeflake-tray-update".to_owned()
        } else {
            "drakeflake-tray".to_owned()
        }
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "DrakeFlake".to_owned(),
            description: std::iter::once(self.state.summary())
                .chain(self.state.health.notes())
                .collect::<Vec<_>>()
                .join("\n"),
            icon_name: self.icon_name(),
            ..ToolTip::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let (notes, inputs) = menu_lines(&self.state);
        let mut items: Vec<MenuItem<Self>> = notes
            .into_iter()
            .map(|note| {
                StandardItem {
                    label: note,
                    enabled: false,
                    ..StandardItem::default()
                }
                .into()
            })
            .collect();

        items.extend(inputs.into_iter().map(|(label, link)| {
            StandardItem {
                label,
                enabled: link.is_some(),
                activate: Box::new(move |_: &mut Self| {
                    if let Some(link) = &link {
                        open_link(link);
                    }
                }),
                ..StandardItem::default()
            }
            .into()
        }));
        if !items.is_empty() {
            items.push(MenuItem::Separator);
        }

        items.extend([
            StandardItem {
                label: "Open DrakeFlake".to_owned(),
                icon_name: "drakeflake".to_owned(),
                activate: Box::new(|_: &mut Self| open_manager()),
                ..StandardItem::default()
            }
            .into(),
            StandardItem {
                label: "Check for updates now".to_owned(),
                icon_name: "view-refresh".to_owned(),
                enabled: !self.state.checking,
                activate: Box::new(|tray: &mut Self| tray.request(Request::Check)),
                ..StandardItem::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".to_owned(),
                icon_name: "application-exit".to_owned(),
                activate: Box::new(|tray: &mut Self| tray.request(Request::Quit)),
                ..StandardItem::default()
            }
            .into(),
        ]);
        items
    }
}

impl Tray {
    fn request(&self, request: Request) {
        if self.requests.send(request).is_err() {
            drakeflake_log::warn!("the update checks have stopped");
        }
    }
}

/// A cargo build has no `share/icons`, so the built-in icons are written to the cache then.
fn icon_dir() -> String {
    drakeflake_tray_core::icon_dir("share/icons")
        .filter(|dir| dir.join("hicolor").is_dir())
        .or_else(cached_icons)
        .map(|dir| dir.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn cached_icons() -> Option<std::path::PathBuf> {
    let base = drakeflake_fs::cache_dir()
        .inspect_err(|error| drakeflake_log::warn!("no place for the tray icons: {error}"))
        .ok()?
        .join("tray-icons");

    let dir = base.join("hicolor/scalable/status");

    let written = std::fs::create_dir_all(&dir).and_then(|()| {
        ICONS
            .iter()
            .try_for_each(|(name, svg)| std::fs::write(dir.join(name), svg))
    });

    match written {
        Ok(()) => {
            drakeflake_log::debug!("tray icons from {}", base.display());
            Some(base)
        }
        Err(error) => {
            drakeflake_log::warn!("cannot write the tray icons to {}: {error}", dir.display());
            None
        }
    }
}
