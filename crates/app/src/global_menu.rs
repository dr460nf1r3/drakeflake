use zbus::blocking::Connection;
use zbus::blocking::fdo::DBusProxy;
use zbus::names::BusName;

const REGISTRAR: &str = "com.canonical.AppMenu.Registrar";

pub fn available() -> bool {
    if cfg!(target_os = "macos") {
        return false;
    }

    // The labs.platform bar crashes without a display and the self-test inherits the session bus.
    if is_headless_qpa(&std::env::var("QT_QPA_PLATFORM").unwrap_or_default()) {
        log::debug!("no global menu without a display");
        return false;
    }

    match registrar_present() {
        Ok(present) => {
            log::debug!("global menu registrar present: {present}");
            present
        }
        Err(err) => {
            log::debug!("no global menu: {err}");
            false
        }
    }
}

fn is_headless_qpa(platform: &str) -> bool {
    matches!(platform, "offscreen" | "minimal")
}

fn registrar_present() -> zbus::Result<bool> {
    let connection = Connection::session()?;
    let name = BusName::try_from(REGISTRAR)?;

    Ok(DBusProxy::new(&connection)?.name_has_owner(name)?)
}

#[cfg(test)]
mod tests;
