use drakeflake_render::{NixValue, render_attr_name, render_value};
use serde_json::Value;

use crate::types::{Editor, OptionType};

#[must_use = "the converted text is the only result"]
pub fn value_to_input(ty: &OptionType, value: &Value) -> Option<String> {
    if value.is_null() {
        return ty.nullable.then(|| "null".to_owned());
    }

    match &ty.editor {
        Editor::Bool => value.as_bool().map(|flag| flag.to_string()),
        Editor::Int { min: _, max: _ } => value.as_i64().map(|number| number.to_string()),
        Editor::Float => value.as_f64().map(|number| number.to_string()),
        Editor::Str { multiline: _ } | Editor::Path => value.as_str().map(str::to_owned),
        Editor::Enum(_) => match value {
            Value::String(text) => Some(text.clone()),
            other @ (Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::Array(_)
            | Value::Object(_)) => json_to_nix(other).map(|nix| render_value(&nix)),
        },
        Editor::List(element) => {
            let items = value.as_array()?;
            let separator = if matches!(**element, Editor::Str { multiline: _ } | Editor::Path) {
                "\n"
            } else {
                " "
            };

            let element_type = OptionType {
                nullable: false,
                editor: (**element).clone(),
            };

            items
                .iter()
                .map(|item| value_to_input(&element_type, item))
                .collect::<Option<Vec<_>>>()
                .map(|parts| parts.join(separator))
        }
        Editor::Package => None,
        Editor::Raw => json_to_nix(value).map(|nix| render_value(&nix)),
    }
}

#[must_use = "the converted value is the only result"]
pub fn json_to_nix(value: &Value) -> Option<NixValue> {
    Some(match value {
        Value::Null => NixValue::Null,
        Value::Bool(flag) => NixValue::Bool(*flag),
        Value::Number(number) => match number.as_i64() {
            Some(int) => NixValue::Int(int),
            None => NixValue::Float(number.as_f64()?),
        },
        Value::String(text) => NixValue::Str(text.clone()),
        Value::Array(items) => {
            NixValue::List(items.iter().map(json_to_nix).collect::<Option<Vec<_>>>()?)
        }
        Value::Object(map) => {
            if map.keys().any(|key| key.starts_with("_drakeflake")) {
                return None;
            }

            NixValue::Attrs(
                map.iter()
                    .map(|(key, item)| json_to_nix(item).map(|nix| (key.clone(), nix)))
                    .collect::<Option<Vec<_>>>()?,
            )
        }
    })
}

#[must_use = "the rendered text is the only result"]
pub fn display_value(value: &Value) -> String {
    display_inner(value, 0)
}

fn display_inner(value: &Value, indent: usize) -> String {
    let pad = "  ".repeat(indent + 1);
    let close = "  ".repeat(indent);

    match value {
        Value::Array(items) if items.is_empty() => "[ ]".to_owned(),
        Value::Array(items) => {
            let inner: Vec<String> = items
                .iter()
                .map(|item| format!("{pad}{}", display_inner(item, indent + 1)))
                .collect();

            format!("[\n{}\n{close}]", inner.join("\n"))
        }
        Value::Object(map) => {
            if let Some(name) = map.get("_drakeflakeDrv").and_then(Value::as_str) {
                return format!("«derivation {name}»");
            }

            if let Some(kind) = map.get("_drakeflakeOpaque").and_then(Value::as_str) {
                return format!("«{kind}»");
            }

            if map.contains_key("_drakeflakeError") {
                return "«evaluation error»".to_owned();
            }

            if map.is_empty() {
                return "{ }".to_owned();
            }

            let inner: Vec<String> = map
                .iter()
                .map(|(key, item)| {
                    format!(
                        "{pad}{} = {};",
                        render_attr_name(key),
                        display_inner(item, indent + 1)
                    )
                })
                .collect();

            format!("{{\n{}\n{close}}}", inner.join("\n"))
        }
        scalar @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => {
            json_to_nix(scalar)
                .map(|nix| render_value(&nix))
                .unwrap_or_default()
        }
    }
}

#[cfg(test)]
mod tests;
