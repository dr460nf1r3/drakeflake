use drakeflake_exec::SHORT_TIMEOUT;
use drakeflake_render::split_option_name;
use serde_json::{Map, Value};

use crate::context::Nix;
use crate::error::FlakeError;
use crate::expr::{CHECK, VALUE, VALUES, resolve};
use crate::flake_target::Target;
use crate::nix::{Query, json_arg};
use crate::options::WantedOption;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeCheck {
    Valid,
    Invalid(String),
    Failed(String),
    Unknown,
}

pub fn check_value(
    nix: &Nix,
    target: &Target,
    option_name: &drakeflake_render::OptionName,
    nix_value: &str,
) -> Result<TypeCheck, FlakeError> {
    let loc = split_option_name(option_name);
    let apply = format!(
        "({}) {}",
        CHECK.replace("__VALUE__", nix_value),
        json_arg(&loc)?
    );

    let outcome: Value =
        target.eval_json(nix, &Query::new("type check", SHORT_TIMEOUT).apply(&apply))?;

    let check = type_check_of(&outcome);

    log::debug!("type check of {option_name}: {check:?}");
    Ok(check)
}

fn type_check_of(outcome: &Value) -> TypeCheck {
    let type_name = outcome
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unspecified")
        .to_owned();

    match outcome.get("result").and_then(Value::as_str) {
        Some("ok") => TypeCheck::Valid,
        Some("invalid") => TypeCheck::Invalid(type_name),
        Some("error") => TypeCheck::Failed(format!(
            "evaluating the value failed (expected {type_name})"
        )),
        _ => TypeCheck::Unknown,
    }
}

pub fn eval_value(
    nix: &Nix,
    target: &Target,
    option_name: &drakeflake_render::OptionName,
) -> Result<Value, FlakeError> {
    let loc = split_option_name(option_name);
    let apply = format!("({}) {}", resolve(VALUE), json_arg(&loc)?);

    target.eval_json(nix, &Query::new("value", SHORT_TIMEOUT).apply(&apply))
}

/// An uncatchable failure of one value fails the whole batch.
pub fn eval_values(
    nix: &Nix,
    target: &Target,
    option_names: &[drakeflake_render::OptionName],
) -> Result<Map<String, Value>, FlakeError> {
    let wanted: Vec<WantedOption> = option_names
        .iter()
        .map(|name| WantedOption::new(name, split_option_name(name)))
        .collect();

    let apply = format!("({}) {}", resolve(VALUES), json_arg(&wanted)?);

    target.eval_json(nix, &Query::new("values", SHORT_TIMEOUT).apply(&apply))
}

#[cfg(test)]
mod tests;
