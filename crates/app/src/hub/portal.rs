//! Qt Quick's dialogs use the platform dialog only in a `QApplication`.
//! This app runs a `QGuiApplication`.

use std::collections::HashMap;
use std::os::unix::ffi::OsStrExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use super::{Hub, SetupMsg};

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const CHOOSER: &str = "org.freedesktop.portal.FileChooser";
const REQUEST: &str = "org.freedesktop.portal.Request";
const CANCELLED: u32 = 1;

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    Folder,
    SaveFile(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Chosen(PathBuf),
    Cancelled,
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picked {
    pub purpose: String,
    pub outcome: Outcome,
}

pub fn pick(owner: &Arc<Hub>, purpose: String, title: String, pick: Pick, start: PathBuf) {
    owner.spawn_background("portal", move |hub, _| {
        let outcome = if cfg!(target_os = "macos") {
            log::debug!("chooser {purpose}: using the native macOS dialog");
            Outcome::Unavailable("no portal on macOS".to_owned())
        } else {
            match choose(&title, &pick, &start) {
                Ok(Some(path)) => Outcome::Chosen(path),
                Ok(None) => Outcome::Cancelled,
                Err(error) => {
                    log::warn!("file chooser portal unavailable: {error}");
                    Outcome::Unavailable(error)
                }
            }
        };

        log::debug!("chooser {purpose}: {outcome:?}");
        hub.mail
            .setup
            .post(SetupMsg::Picked(Picked { purpose, outcome }));
    });
}

fn choose(title: &str, pick: &Pick, start: &Path) -> Result<Option<PathBuf>, String> {
    let connection = Connection::session().map_err(|err| format!("no session bus: {err}"))?;
    let token = format!(
        "drakeflake{}_{}",
        std::process::id(),
        NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
    );

    let sender = connection
        .unique_name()
        .map(|name| name.trim_start_matches(':').replace('.', "_"))
        .ok_or("no unique bus name")?;

    let expected = format!("{PORTAL_PATH}/request/{sender}/{token}");

    let mut responses = request_responses(&connection, &expected)?;

    let mut options: HashMap<&str, Value<'_>> = HashMap::new();
    options.insert("handle_token", Value::from(token.as_str()));
    options.insert("modal", Value::from(true));
    options.insert("current_folder", Value::from(nul_terminated(start)));

    let method = match pick {
        Pick::Folder => {
            options.insert("directory", Value::from(true));
            "OpenFile"
        }
        Pick::SaveFile(name) => {
            options.insert("current_name", Value::from(name.as_str()));
            "SaveFile"
        }
    };

    let chooser = Proxy::new(&connection, PORTAL, PORTAL_PATH, CHOOSER)
        .map_err(|err| format!("no file chooser portal: {err}"))?;

    let handle: OwnedObjectPath = chooser
        .call(method, &("", title, options))
        .map_err(|err| format!("{method} failed: {err}"))?;

    if handle.as_str() != expected {
        responses = request_responses(&connection, handle.as_str())?;
    }

    let message = responses.next().ok_or("the portal closed the request")?;
    let (code, results): (u32, HashMap<String, OwnedValue>) = message
        .body()
        .deserialize()
        .map_err(|err| format!("unexpected portal answer: {err}"))?;

    if code == CANCELLED {
        return Ok(None);
    }

    if code != 0 {
        return Err(format!("the chooser failed (code {code})"));
    }

    let uris: Vec<String> = results
        .get("uris")
        .and_then(|value| Vec::<String>::try_from(value.clone()).ok())
        .unwrap_or_default();

    Ok(uris.first().and_then(|uri| file_uri_path(uri)))
}

fn request_responses(
    connection: &Connection,
    path: &str,
) -> Result<zbus::blocking::proxy::SignalIterator<'static>, String> {
    let request = Proxy::new(connection, PORTAL, path.to_owned(), REQUEST)
        .map_err(|err| format!("cannot watch {path}: {err}"))?;

    request
        .receive_signal("Response")
        .map_err(|err| format!("cannot watch {path}: {err}"))
}

fn nul_terminated(path: &Path) -> Vec<u8> {
    let mut bytes = path.as_os_str().as_bytes().to_vec();
    bytes.push(0);
    bytes
}

fn file_uri_path(uri: &str) -> Option<PathBuf> {
    let encoded = uri.strip_prefix("file://")?;
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut rest = encoded.as_bytes();

    while let Some((&first, tail)) = rest.split_first() {
        let decoded = match (first, tail) {
            (b'%', [high, low, ..]) => hex_value(*high)
                .zip(hex_value(*low))
                .map(|(high, low)| ((high << 4_u32) | low, 2_usize)),
            _ => None,
        };

        if let Some((byte, used)) = decoded {
            bytes.push(byte);
            rest = tail.get(used..).unwrap_or_default();
        } else {
            bytes.push(first);
            rest = tail;
        }
    }

    Some(PathBuf::from(std::ffi::OsStr::from_bytes(&bytes)))
}

const fn hex_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uris_are_decoded() {
        assert_eq!(
            file_uri_path("file:///home/me/My%20Flake").as_deref(),
            Some(Path::new("/home/me/My Flake"))
        );
        assert_eq!(
            file_uri_path("file:///a%zzb").as_deref(),
            Some(Path::new("/a%zzb"))
        );
        assert_eq!(file_uri_path("https://example.org"), None);
    }

    #[test]
    fn escapes_are_decoded_bytewise_and_broken_ones_kept() {
        for (uri, path) in [
            ("file:///caf%C3%A9", "/café"),
            ("file:///a%2Fb", "/a/b"),
            ("file:///A%2a%2A", "/A**"),
            ("file:///50%", "/50%"),
            ("file:///x%4", "/x%4"),
            ("file:///%%41", "/%A"),
            ("file://", ""),
        ] {
            assert_eq!(
                file_uri_path(uri).as_deref(),
                Some(Path::new(path)),
                "{uri}"
            );
        }

        assert_eq!(file_uri_path("FILE:///a"), None);
    }

    #[test]
    fn paths_are_nul_terminated() {
        assert_eq!(nul_terminated(Path::new("/etc")), b"/etc\0".to_vec());
    }
}
