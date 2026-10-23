use drakeflake_exec::Context;
use drakeflake_plan::{RunError, SYNTAX_CHECK_TIMEOUT, SyntaxError, SyntaxRun, check_syntax_with};
use drakeflake_validate::{parse_expression, wrap_expression};

use crate::error::FileError;

pub fn check_expression(exec: &Context, text: &str) -> Result<(), FileError> {
    let expression = parse_expression(text)?;

    check_syntax(
        exec,
        &format!(
            "{{ config, lib, pkgs, ... }}:\n{}\n",
            wrap_expression(&expression)
        ),
    )?;

    Ok(())
}

pub fn check_syntax(exec: &Context, text: &str) -> Result<(), SyntaxError> {
    check_syntax_with(text, |program, args, input| {
        exec.command_with_input(program, args, input, SYNTAX_CHECK_TIMEOUT)
            .map(|captured| SyntaxRun {
                success: captured.success,
                stderr: captured.stderr,
            })
            .map_err(RunError::from)
    })
}
