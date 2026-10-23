use drakeflake_file::{ManagedOptionExt as _, parse_mode, priority_choices};
use drakeflake_plan::{InputMode, ManagedOption};
use serde_json::{Value, json};

pub fn settings_value(option: &ManagedOption) -> Value {
    json!({
        "mode": option.mode.to_string(),
        "priority": option.priority.map(|priority| priority.to_string()).unwrap_or_default(),
        "effectivePriority": option.effective_priority().to_string(),
        "host": option.host.clone().unwrap_or_default(),
    })
}

pub fn priority_choices_value(type_name: &str, input: &str, mode: &str) -> Value {
    let mode = parse_mode(mode).unwrap_or_else(|error| {
        log::warn!("{error}: offering the priorities of a typed value");
        InputMode::Typed
    });

    let offered = priority_choices(type_name, input, mode);
    let choices: Vec<String> = offered
        .choices
        .into_iter()
        .map(|priority| priority.to_string())
        .collect();

    json!({ "choices": choices, "implicit": offered.implicit.to_string() })
}
