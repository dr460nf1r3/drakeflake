use std::path::Path;

use drakeflake_exec::Context;
use drakeflake_plan::WritePlan;
use log::{debug, error, info};

use crate::direct::apply_direct;
use crate::elevated::write_elevated;
use crate::error::FileError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteMethod {
    Direct,
    Elevated,
}

pub fn write_files(
    exec: &Context,
    plan: &WritePlan,
    helper: Option<&Path>,
) -> Result<WriteMethod, FileError> {
    let direct = plan
        .clone()
        .validate_local()
        .map_err(FileError::Refused)
        .and_then(|validated| apply_direct(&validated).map_err(FileError::Write));

    let result = match direct {
        Ok(()) => Ok(WriteMethod::Direct),
        // Also a target that validation could not read: root may.
        Err(err) if err.is_permission_denied() => {
            debug!("direct write denied ({err}), elevating through the helper");
            write_elevated(exec, plan, helper)
        }
        Err(err) => Err(err),
    };

    match &result {
        Ok(method) => info!(
            "write plan applied ({method:?}): {} files written, {} removed",
            plan.write.len(),
            plan.remove.len()
        ),
        Err(err) => error!("cannot apply write plan: {err}"),
    }

    result
}
