//! `AppKit` objects live on the main thread only, so state from the check thread arrives as blocks on the main queue.

use std::cell::RefCell;
use std::sync::mpsc;

use dispatch2::DispatchQueue;
use drakeflake_tray_core::{Request, State, menu_lines, open_link, open_manager};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

use crate::icons::Icons;

const OPEN: &str = "open";
const CHECK: &str = "check";
const QUIT: &str = "quit";
const INPUT: &str = "input-";

thread_local! {
    static MENU_BAR: RefCell<Option<MenuBar>> = const { RefCell::new(None) };
}

struct MenuBar {
    item: TrayIcon,
    links: Vec<Option<String>>,
    icons: Icons,
}

impl MenuBar {
    fn new() -> Result<Self, String> {
        let icons = Icons::load();
        let state = State::default();
        let (menu, links) = menu(&state)?;

        let builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("DrakeFlake");

        // Without its icon the item shows its name, so it can still be found.
        let builder = match icons.normal.clone() {
            Some(icon) => builder.with_icon_templated(icon),
            None => builder.with_title("DrakeFlake"),
        };

        let item = builder.build().map_err(|error| error.to_string())?;

        Ok(Self { item, links, icons })
    }

    fn update(&mut self, state: &State) {
        match menu(state) {
            Ok((menu, links)) => {
                self.item.set_menu(Some(Box::new(menu)));
                self.links = links;
            }
            Err(error) => drakeflake_log::warn!("cannot rebuild the menu: {error}"),
        }

        if let Err(error) = self
            .item
            .set_tooltip(Some(format!("DrakeFlake: {}", state.summary())))
        {
            drakeflake_log::warn!("cannot set the tooltip: {error}");
        }

        let icon = if state.needs_attention() {
            self.icons.update.clone()
        } else {
            self.icons.normal.clone()
        };

        if icon.is_some()
            && let Err(error) = self.item.set_icon_templated(icon)
        {
            drakeflake_log::warn!("cannot change the menu bar icon: {error}");
        }
    }
}

fn menu(state: &State) -> Result<(Menu, Vec<Option<String>>), String> {
    let menu = Menu::new();
    let (notes, inputs) = menu_lines(state);
    let mut links = Vec::new();

    let append = |item: &dyn tray_icon::menu::IsMenuItem| {
        menu.append(item).map_err(|error| error.to_string())
    };

    append(&MenuItem::new(state.summary(), false, None))?;
    for note in notes {
        append(&MenuItem::new(note, false, None))?;
    }

    for (label, link) in inputs {
        append(&MenuItem::with_id(
            format!("{INPUT}{}", links.len()),
            label,
            link.is_some(),
            None,
        ))?;
        links.push(link);
    }

    append(&PredefinedMenuItem::separator())?;
    append(&MenuItem::with_id(OPEN, "Open DrakeFlake", true, None))?;
    append(&MenuItem::with_id(
        CHECK,
        "Check for Updates Now",
        !state.checking,
        None,
    ))?;
    append(&PredefinedMenuItem::separator())?;
    append(&MenuItem::with_id(QUIT, "Quit", true, None))?;

    Ok((menu, links))
}

/// Installs the menu bar on the main thread, whose run loop `tray-icon` needs.
pub fn install(requests: mpsc::Sender<Request>) {
    // Menu actions arrive on the main thread, where the item lives.
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let id = event.id().as_ref();
        let request = match id {
            OPEN => {
                open_manager();
                return;
            }
            CHECK => Request::Check,
            QUIT => Request::Quit,
            _ => {
                if let Some(index) = id.strip_prefix(INPUT).and_then(|n| n.parse::<usize>().ok()) {
                    open_input(index);
                }

                return;
            }
        };

        if requests.send(request).is_err() {
            drakeflake_log::warn!("the update checks have stopped");
        }
    }));

    DispatchQueue::main().exec_async(|| match MenuBar::new() {
        Ok(menu_bar) => MENU_BAR.with(|slot| *slot.borrow_mut() = Some(menu_bar)),
        Err(error) => {
            drakeflake_log::error!("no menu bar item: {error}");
            #[expect(
                clippy::exit,
                reason = "a tray without its menu bar item has nothing to show, and AppKit's run loop never returns by itself"
            )]
            std::process::exit(1);
        }
    });
}

fn open_input(index: usize) {
    let link = MENU_BAR.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|menu_bar| menu_bar.links.get(index).cloned().flatten())
    });

    if let Some(link) = link {
        open_link(&link);
    }
}

pub fn show(state: &State) {
    let state = state.clone();
    DispatchQueue::main().exec_async(move || {
        MENU_BAR.with(|slot| {
            if let Some(menu_bar) = slot.borrow_mut().as_mut() {
                menu_bar.update(&state);
            }
        });
    });
}
