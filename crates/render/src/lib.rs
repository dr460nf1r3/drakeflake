mod ids;

pub use ids::{HostName, OptionName, PackageAttr, UserName};

#[derive(Debug, Clone, PartialEq)]
pub enum NixValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Self>),
    Attrs(Vec<(String, Self)>),
    Null,
    Raw(String),
}

#[must_use = "the rendered Nix text is the only result"]
pub fn render_value(value: &NixValue) -> String {
    match value {
        NixValue::Bool(true) => "true".to_owned(),
        NixValue::Bool(false) => "false".to_owned(),
        NixValue::Null => "null".to_owned(),
        NixValue::Int(number) => int_repr(*number),
        NixValue::Float(number) => float_repr(*number),
        NixValue::Str(text) => render_string(text),
        NixValue::List(items) => render_list(items),
        NixValue::Attrs(pairs) => render_attrs(pairs),
        NixValue::Raw(expr) => render_raw(expr),
    }
}

fn render_raw(expr: &str) -> String {
    let trimmed = expr.trim();

    if is_plain_attrpath(trimmed) {
        trimmed.to_owned()
    } else {
        wrap_expression(trimmed)
    }
}

/// The closing parenthesis goes on its own line when a `#` comment could swallow it.
#[must_use = "the wrapped expression is the only result"]
pub fn wrap_expression(expression: &str) -> String {
    let (open, close) = delimiters(expression);
    format!("{open}{expression}{close}")
}

#[must_use = "the delimiters are only chosen"]
pub fn delimiters(expression: &str) -> (&'static str, &'static str) {
    if expression.contains('#') || expression.contains('\n') {
        ("(\n", "\n)")
    } else {
        ("(", ")")
    }
}

fn int_repr(value: i64) -> String {
    if value == i64::MIN {
        // Nix reads the literal as a negation of 2^63, which overflows.
        "(-9223372036854775807 - 1)".to_owned()
    } else {
        value.to_string()
    }
}

/// A bare negative number is not a valid list item.
fn render_list_item(item: &NixValue) -> String {
    let text = render_value(item);

    if matches!(item, NixValue::Int(_) | NixValue::Float(_)) && text.starts_with('-') {
        format!("({text})")
    } else {
        text
    }
}

fn is_plain_attrpath(text: &str) -> bool {
    !text.is_empty() && text.split('.').all(is_identifier)
}

fn float_repr(value: f64) -> String {
    if !value.is_finite() {
        // Nix has no literal for NaN/inf. Never emit something unparsable.
        return "0.0".to_owned();
    }

    let text = value.to_string();

    if text.contains('.') || text.contains('e') || text.contains('E') {
        text
    } else {
        format!("{text}.0")
    }
}

#[must_use = "the rendered Nix text is the only result"]
pub fn render_string(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + 2);
    escaped.push('"');

    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '$' => escaped.push_str("\\$"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }

    escaped.push('"');
    escaped
}

pub fn render_list(items: &[NixValue]) -> String {
    if items.is_empty() {
        return "[ ]".to_owned();
    }

    let rendered: Vec<String> = items.iter().map(render_list_item).collect();
    format!("[ {} ]", rendered.join(" "))
}

#[must_use = "the rendered Nix text is the only result"]
pub fn render_attrs(pairs: &[(String, NixValue)]) -> String {
    let mut sorted: Vec<&(String, NixValue)> = pairs.iter().collect();
    sorted.sort_by(|left, right| left.0.cmp(&right.0));

    let mut lines = vec!["{".to_owned()];
    lines.extend(
        sorted
            .into_iter()
            .map(|(key, value)| format!("  {} = {};", render_attr_name(key), render_value(value))),
    );
    lines.push("}".to_owned());
    lines.join("\n")
}

#[must_use = "the check has no other effect"]
pub fn is_identifier(part: &str) -> bool {
    let mut chars = part.chars();

    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '\''))
        && !matches!(
            part,
            "if" | "then" | "else" | "let" | "in" | "with" | "rec" | "inherit" | "assert" | "or"
        )
}

#[must_use = "the rendered Nix text is the only result"]
pub fn render_attr_name(part: &str) -> String {
    if is_identifier(part) {
        part.to_owned()
    } else {
        render_string(part)
    }
}

#[must_use = "the name is not changed in place"]
pub fn split_option_name(name: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = name.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => quoted = !quoted,
            '\\' if quoted => match chars.next() {
                Some('n') => current.push('\n'),
                Some('r') => current.push('\r'),
                Some('t') => current.push('\t'),
                other => current.extend(other),
            },
            '.' if !quoted => parts.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }

    parts.push(current);
    parts
}

#[cfg(test)]
mod tests;
