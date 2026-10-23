mod validated;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use log::{debug, warn};
use serde::{Deserialize, Serialize};

pub use self::validated::ValidatedPlan;
use crate::header::{generated_files, has_generated_header};
use crate::refusal::Refusal;
use crate::target::{
    check_generated_content, check_local_remove_target, check_local_write_target,
    check_remove_target, check_write_content, check_write_target,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFile {
    pub path: PathBuf,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub entry: String,
    pub parts: Vec<RenderedFile>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WritePlan {
    pub write: BTreeMap<PathBuf, String>,
    pub remove: Vec<PathBuf>,
}

impl WritePlan {
    #[must_use = "the plan is only built, nothing is written"]
    pub fn for_rendered(
        entry: &Path,
        rendered: &Rendered,
        previous_entry: Option<&str>,
        parts: &Path,
    ) -> Self {
        let dir = entry.parent().map_or_else(PathBuf::new, Path::to_path_buf);
        let mut write: BTreeMap<PathBuf, String> = rendered
            .parts
            .iter()
            .map(|part| (dir.join(&part.path), part.text.clone()))
            .collect();

        write.insert(entry.to_path_buf(), rendered.entry.clone());

        // A file list in a hand-written file is not ours to act on.
        let removed = match previous_entry
            .filter(|text| has_generated_header(text))
            .map(generated_files)
        {
            Some(list) if !list.is_empty() => list
                .into_iter()
                .map(|path| dir.join(path))
                .filter(|path| !write.contains_key(path))
                .collect(),
            // No usable file list: a hand-edited or older entry. Remove the
            // whole parts folder so its stale modules do not stay imported.
            _ => fs::read_dir(parts)
                .map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .map(|found| found.path())
                        .filter(|path| !write.contains_key(path))
                        .collect()
                })
                .unwrap_or_default(),
        };

        let plan = Self {
            write,
            remove: removed,
        };

        debug!("planned {}", plan.describe());
        plan
    }

    pub fn validate(self) -> Result<ValidatedPlan, Refusal> {
        self.check_each(check_write_target, check_write_content, check_remove_target)?;

        Ok(ValidatedPlan::new(self))
    }

    /// Without the flake rule, since the output file may be configured elsewhere, and without the parse check, which the app already ran.
    pub fn validate_local(self) -> Result<ValidatedPlan, Refusal> {
        self.check_each(
            check_local_write_target,
            check_generated_content,
            check_local_remove_target,
        )?;

        Ok(ValidatedPlan::new(self))
    }

    fn check_each(
        &self,
        target: impl Fn(&Path) -> Result<(), Refusal>,
        content: impl Fn(&str) -> Result<(), Refusal>,
        removal: impl Fn(&Path) -> Result<(), Refusal>,
    ) -> Result<(), Refusal> {
        const MAX_PLAN_FILES: usize = 1024;

        let refused = |path: &Path, cause: Refusal| {
            warn!("refused {}: {cause}", path.display());
            cause.in_file(path)
        };

        if self.write.len() > MAX_PLAN_FILES || self.remove.len() > MAX_PLAN_FILES {
            let count = self.write.len().max(self.remove.len());
            return Err(Refusal::TooManyFiles(count));
        }

        if let Some(path) = self
            .remove
            .iter()
            .find(|path| self.write.contains_key(*path))
        {
            return Err(refused(path, Refusal::BothWrittenAndRemoved));
        }

        // Two spellings of one file: on a case-insensitive file system they
        // are the same file, so only the last text would survive.
        if let Some(path) = duplicate_target(self.write.keys()) {
            return Err(refused(path, Refusal::DuplicateTarget));
        }

        for (path, text) in &self.write {
            target(path).map_err(|err| refused(path, err))?;
            content(text).map_err(|err| refused(path, err))?;
        }

        for path in &self.remove {
            removal(path).map_err(|err| refused(path, err))?;
        }

        Ok(())
    }

    fn describe(&self) -> String {
        let list = |paths: &mut dyn Iterator<Item = &PathBuf>| {
            paths
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };

        format!(
            "write [{}], remove [{}]",
            list(&mut self.write.keys()),
            list(&mut self.remove.iter())
        )
    }
}

/// Compared lower cased, as HFS+ and APFS compare names.
fn duplicate_target<'a>(paths: impl Iterator<Item = &'a PathBuf>) -> Option<&'a PathBuf> {
    let mut seen: Vec<String> = Vec::new();

    for path in paths {
        let key = path.to_string_lossy().to_lowercase();

        if seen.contains(&key) {
            return Some(path);
        }

        seen.push(key);
    }

    None
}
