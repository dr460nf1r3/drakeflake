use drakeflake_render::{NixValue, is_identifier};

use crate::error::InputError;
use crate::types::{Editor, OptionType, choice_text};

#[must_use = "the parsed value is the only result"]
pub fn parse_input(ty: &OptionType, text: &str) -> Result<NixValue, InputError> {
    if ty.nullable && text.trim() == "null" {
        return Ok(NixValue::Null);
    }

    parse_with(&ty.editor, text)
}

fn parse_with(editor: &Editor, text: &str) -> Result<NixValue, InputError> {
    match editor {
        Editor::Bool => parse_bool(text).map(NixValue::Bool),
        Editor::Int { min, max } => parse_int_value(text, *min, *max).map(NixValue::Int),
        Editor::Float => parse_float(text).map(NixValue::Float),
        Editor::Str { multiline: _ } => Ok(NixValue::Str(text.to_owned())),
        Editor::Path => parse_path(text).map(NixValue::Str),
        Editor::Enum(choices) => parse_enum(choices, text),
        Editor::Package => parse_package(text).map(NixValue::Raw),
        Editor::List(element) => parse_list(element, text),
        Editor::Raw => parse_raw(text).map(NixValue::Raw),
    }
}

fn parse_bool(text: &str) -> Result<bool, InputError> {
    match text.trim().to_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        _ => Err(InputError::NotBool(text.trim().to_owned())),
    }
}

fn parse_int_value(text: &str, min: Option<i64>, max: Option<i64>) -> Result<i64, InputError> {
    let trimmed = text.trim();
    let Ok(number) = trimmed.parse::<i64>() else {
        return Err(InputError::NotWholeNumber(trimmed.to_owned()));
    };

    if let Some(low) = min.filter(|low| number < *low) {
        return Err(InputError::BelowMinimum {
            value: number,
            minimum: low,
        });
    }

    if let Some(high) = max.filter(|high| number > *high) {
        return Err(InputError::AboveMaximum {
            value: number,
            maximum: high,
        });
    }

    Ok(number)
}

fn parse_float(text: &str) -> Result<f64, InputError> {
    let trimmed = text.trim();

    trimmed
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| InputError::NotNumber(trimmed.to_owned()))
}

fn parse_path(text: &str) -> Result<String, InputError> {
    let trimmed = text.trim();

    if trimmed.starts_with('/') {
        Ok(trimmed.to_owned())
    } else {
        Err(InputError::RelativePath)
    }
}

fn parse_enum(choices: &[NixValue], text: &str) -> Result<NixValue, InputError> {
    choices
        .iter()
        .find(|choice| choice_text(choice) == text)
        .cloned()
        .ok_or_else(|| {
            let allowed: Vec<String> = choices.iter().map(choice_text).collect();
            InputError::NotOneOf(allowed)
        })
}

#[must_use = "the parsed path is the only result"]
pub fn parse_package(text: &str) -> Result<String, InputError> {
    let trimmed = text.trim();
    let attr = trimmed.strip_prefix("pkgs.").unwrap_or(trimmed);

    if !attr.is_empty() && attr.split('.').all(is_identifier) {
        Ok(format!("pkgs.{attr}"))
    } else {
        Err(InputError::NotPackage(trimmed.to_owned()))
    }
}

fn parse_list(element: &Editor, text: &str) -> Result<NixValue, InputError> {
    let splits_on_commas = matches!(
        element,
        Editor::Int { min: _, max: _ } | Editor::Float | Editor::Bool | Editor::Package
    );

    let items: Vec<&str> = if splits_on_commas {
        text.split(|ch: char| ch == ',' || ch.is_whitespace())
            .filter(|item| !item.is_empty())
            .collect()
    } else {
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .collect()
    };

    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            parse_with(element, item).map_err(|cause| InputError::Item {
                number: index + 1,
                cause: Box::new(cause),
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(NixValue::List)
}

fn parse_raw(text: &str) -> Result<String, InputError> {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return Err(InputError::EmptyExpression);
    }

    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in trimmed.chars() {
        if in_string {
            match (escaped, ch) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }

            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' | '[' | '{' => stack.push(ch),
            ')' | ']' | '}' => {
                let expected = match ch {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };

                if stack.pop() != Some(expected) {
                    return Err(InputError::Unbalanced(ch));
                }
            }
            _ => {}
        }
    }

    if in_string {
        return Err(InputError::UnterminatedString);
    }

    if let Some(open) = stack.pop() {
        return Err(InputError::Unclosed(open));
    }

    Ok(trimmed.to_owned())
}
