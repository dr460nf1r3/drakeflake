use crate::error::ExecError;
use crate::process::{Captured, Invocation, run_logged};

pub trait Runner: Send + Sync {
    fn run(&self, invocation: Invocation<'_>) -> Result<Captured, ExecError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, invocation: Invocation<'_>) -> Result<Captured, ExecError> {
        run_logged(invocation)
    }
}
