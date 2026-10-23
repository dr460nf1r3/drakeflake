use std::sync::Arc;

use drakeflake_plan::ManagedState;

use super::operations::Reporting;
use super::{Hub, HubError, Mode};

pub fn start(hub: &Arc<Hub>, plan: ManagedState, mode: Mode) -> Result<(), HubError> {
    if !matches!(mode, Mode::Switch | Mode::Boot | Mode::Test) {
        return Err(HubError::invalid(
            "Only switch, boot and test can be reviewed",
        ));
    }

    log::info!("review and apply started ({})", mode.verb());
    hub.spawn_exclusive("review-apply", Reporting::Shown, move |run| {
        run.save_and_rebuild(&plan, mode, |run, target| {
            run.rebuild_with_review(target, mode)
        })
    })
}

#[cfg(test)]
mod tests;
