mod guide;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use qtbridge::qobject;
use serde_json::{Value, json};

// The path stays in the log, so the page only shows these fixed texts.
const NOT_A_PAGE: &str = "it is not a page of the guide";
const UNREADABLE: &str = "the file is missing or unreadable";

/// Read from disk per call so an edit to `docs/src` shows without a rebuild.
#[derive(Default)]
pub struct DocsModel {
    dir: PathBuf,
}

impl DocsModel {
    pub fn new() -> Self {
        Self { dir: guide::dir() }
    }

    #[cfg(test)]
    pub const fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn read(&self, file: &str) -> Result<String, &'static str> {
        let path = self.dir.join(file);
        std::fs::read_to_string(&path).map_err(|err| {
            log::warn!("cannot read the guide page {}: {err}", path.display());
            UNREADABLE
        })
    }
}

#[qobject]
impl DocsModel {
    #[qslot]
    pub fn contents(&self) -> Value {
        match self.read("SUMMARY.md") {
            Ok(summary) => json!({ "entries": guide::contents(&summary), "error": "" }),
            Err(error) => json!({ "entries": [], "error": error }),
        }
    }

    #[qslot]
    pub fn page(&self, file: String) -> Value {
        if !guide::is_page_name(&file) {
            log::warn!("not a guide page: {file:?}");
            return json!({ "file": file, "title": "", "blocks": [], "error": NOT_A_PAGE });
        }

        match self.read(&file) {
            Ok(markdown) => {
                let page = guide::page(&file, &markdown, &self.dir);
                json!({ "file": page.file, "title": page.title, "blocks": page.blocks, "error": "" })
            }
            Err(error) => json!({ "file": file, "title": "", "blocks": [], "error": error }),
        }
    }
}
