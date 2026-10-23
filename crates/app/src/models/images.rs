use std::path::{Path, PathBuf};
use std::sync::Arc;

use qtbridge::qobject;
use serde_json::{Value, json};

use crate::hub::{Hub, ImagesMsg};
use crate::models::open_folder;

#[derive(Default)]
pub struct ImagesModel {
    hub: Arc<Hub>,
    formats: Vec<String>,
    error: String,
    loading: bool,
    saving: bool,
    file: String,
    format: String,
}

impl ImagesModel {
    pub fn new(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            ..Self::default()
        }
    }

    fn handle(&mut self, message: ImagesMsg) {
        match message {
            ImagesMsg::Formats(result) => {
                self.loading = false;
                match result {
                    Ok(formats) => {
                        self.formats = formats;
                        self.error.clear();
                    }
                    Err(error) => self.error = error,
                }
            }
            ImagesMsg::Saved => self.saving = false,
            ImagesMsg::Built { format, file } => {
                self.format = format;
                self.file = file;
            }
        }
    }
}

#[qobject]
impl ImagesModel {
    qproperty!("formats", Read = formats, Notify = state_changed);
    qproperty!("error", Read = error, Notify = state_changed);
    qproperty!("loading", Read = loading, Notify = state_changed);
    qproperty!("saving", Read = saving, Notify = state_changed);
    qproperty!("file", Read = file, Notify = state_changed);
    qproperty!("format", Read = format, Notify = state_changed);

    pub fn formats(&self) -> Value {
        json!(self.formats)
    }

    pub fn error(&self) -> String {
        self.error.clone()
    }

    pub const fn loading(&self) -> bool {
        self.loading
    }

    pub const fn saving(&self) -> bool {
        self.saving
    }

    pub fn file(&self) -> String {
        self.file.clone()
    }

    pub fn format(&self) -> String {
        self.format.clone()
    }

    #[qsignal]
    fn state_changed(&mut self);

    #[qslot]
    pub fn drain(&mut self) {
        let messages = self.hub.mail().images.take();

        if messages.is_empty() {
            return;
        }

        for message in messages {
            self.handle(message);
        }

        self.state_changed();
    }

    #[qslot]
    pub fn load_formats(&mut self, host: String) {
        log::debug!("listing image formats of {host:?}");
        self.loading = true;
        self.error.clear();
        self.formats.clear();
        self.hub.load_image_formats(host);
        self.state_changed();
    }

    #[qslot]
    pub fn save(&mut self, destination: String) {
        let destination = destination.strip_prefix("file://").unwrap_or(&destination);
        self.saving = true;
        self.hub
            .save_image(self.file.clone(), PathBuf::from(destination));
        self.state_changed();
    }

    #[qslot]
    pub fn show(&self) {
        let folder = Path::new(&self.file)
            .parent()
            .map(|dir| dir.to_string_lossy().into_owned())
            .unwrap_or_default();

        if let Err(error) = open_folder(folder) {
            self.hub.log_error(error);
        }
    }

    #[qslot]
    pub fn start(&self) {
        self.hub.start_image(self.file.clone(), self.format.clone());
    }
}
