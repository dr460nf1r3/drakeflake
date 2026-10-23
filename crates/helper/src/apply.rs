use std::collections::BTreeMap;
use std::io::Read as _;

use drakeflake_plan::{Refusal, WritePlan};

use crate::error::HelperError;
use crate::{MAX_INPUT_BYTES, secure};

pub fn apply() -> Result<(), HelperError> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(MAX_INPUT_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(HelperError::ReadPlan)?;

    // Checked before decoding: the cut-off may split a character.
    check_size(bytes.len())?;

    let json = String::from_utf8(bytes).map_err(|err| {
        HelperError::ReadPlan(std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    })?;

    apply_json(&json)
}

fn check_size(len: usize) -> Result<(), HelperError> {
    if u64::try_from(len).is_ok_and(|len| len <= MAX_INPUT_BYTES) {
        Ok(())
    } else {
        Err(HelperError::PlanTooLarge)
    }
}

/// One root `sync` per file: bound the amplification of a tiny-entry plan.
const fn check_entries(writes: usize, removes: usize) -> Result<(), HelperError> {
    const MAX_PLAN_FILES: usize = 1024;

    if writes <= MAX_PLAN_FILES && removes <= MAX_PLAN_FILES {
        Ok(())
    } else {
        Err(HelperError::PlanTooLarge)
    }
}

pub fn apply_json(json: &str) -> Result<(), HelperError> {
    check_size(json.len())?;

    let plan: WritePlan = serde_json::from_str(json).map_err(HelperError::InvalidPlan)?;
    check_entries(plan.write.len(), plan.remove.len())?;

    // Resolving again at write time would follow a directory swapped for a symlink in between.
    let mut write = BTreeMap::new();

    for (path, text) in plan.write {
        let resolved = secure::canonical(&path).map_err(HelperError::ApplyPlan)?;

        // Two spellings of one file would silently keep only the last text.
        if write.insert(resolved.clone(), text).is_some() {
            return Err(HelperError::Refused(
                Refusal::DuplicateTarget.in_file(&resolved),
            ));
        }
    }

    let plan = WritePlan {
        write,
        remove: plan
            .remove
            .iter()
            .map(|path| secure::canonical(path))
            .collect::<std::io::Result<_>>()
            .map_err(HelperError::ApplyPlan)?,
    };

    let validated = plan.validate().map_err(HelperError::Refused)?;

    secure::apply_plan(&validated).map_err(HelperError::ApplyPlan)
}
