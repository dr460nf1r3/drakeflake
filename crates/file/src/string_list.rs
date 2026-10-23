use drakeflake_plan::{InputMode, ManagedState};
use drakeflake_render::NixValue;
use drakeflake_validate::{parse_input, parse_type};

pub const LIST_TYPE: &str = "list of string";

#[must_use = "only reads the state"]
pub fn listed(state: &ManagedState, option: &str) -> Option<Vec<String>> {
    let Some(managed) = state.options.get(option) else {
        return Some(Vec::new());
    };

    if managed.host.is_some() || managed.mode != InputMode::Typed {
        return None;
    }

    let NixValue::List(items) = parse_input(&parse_type(LIST_TYPE), &managed.input).ok()? else {
        return None;
    };

    items
        .into_iter()
        .map(|item| {
            if let NixValue::Str(text) = item {
                Some(text)
            } else {
                None
            }
        })
        .collect()
}

#[must_use = "only builds the text"]
pub fn list_input(entries: &[String]) -> String {
    entries.join("\n")
}

#[cfg(test)]
mod tests;
