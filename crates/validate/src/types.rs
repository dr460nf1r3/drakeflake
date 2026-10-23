use std::iter::Peekable;
use std::str::Chars;

use drakeflake_render::{NixValue, render_value};

pub const KIND_BOOL: &str = "bool";
pub const KIND_INT: &str = "int";
pub const KIND_FLOAT: &str = "float";
pub const KIND_STRING: &str = "string";
pub const KIND_TEXT: &str = "text";
pub const KIND_PATH: &str = "path";
pub const KIND_ENUM: &str = "enum";
pub const KIND_LIST: &str = "list";
pub const KIND_PACKAGE: &str = "package";
pub const KIND_RAW: &str = "raw";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueShape {
    Scalar,
    List,
    Attrs,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Editor {
    Bool,
    Int { min: Option<i64>, max: Option<i64> },
    Float,
    Str { multiline: bool },
    Path,
    Enum(Vec<NixValue>),
    List(Box<Self>),
    Package,
    Raw,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionType {
    pub nullable: bool,
    pub editor: Editor,
}

impl OptionType {
    #[must_use = "the name is only returned"]
    pub const fn kind(&self) -> &'static str {
        match self.editor {
            Editor::Bool => KIND_BOOL,
            Editor::Int { min: _, max: _ } => KIND_INT,
            Editor::Float => KIND_FLOAT,
            Editor::Str { multiline: false } => KIND_STRING,
            Editor::Str { multiline: true } => KIND_TEXT,
            Editor::Path => KIND_PATH,
            Editor::Enum(_) => KIND_ENUM,
            Editor::List(_) => KIND_LIST,
            Editor::Package => KIND_PACKAGE,
            Editor::Raw => KIND_RAW,
        }
    }

    #[must_use = "the choices are only computed"]
    pub fn choices(&self) -> Vec<String> {
        match &self.editor {
            Editor::Enum(values) => values.iter().map(choice_text).collect(),
            Editor::Bool
            | Editor::Int { min: _, max: _ }
            | Editor::Float
            | Editor::Str { multiline: _ }
            | Editor::Path
            | Editor::List(_)
            | Editor::Package
            | Editor::Raw => Vec::new(),
        }
    }

    #[must_use = "the bounds are only returned"]
    pub const fn bounds(&self) -> (Option<i64>, Option<i64>) {
        match self.editor {
            Editor::Int { min, max } => (min, max),
            Editor::Bool
            | Editor::Float
            | Editor::Str { multiline: _ }
            | Editor::Path
            | Editor::Enum(_)
            | Editor::List(_)
            | Editor::Package
            | Editor::Raw => (None, None),
        }
    }

    #[must_use = "the check has no other effect"]
    pub const fn merges(&self) -> bool {
        matches!(
            self.editor,
            Editor::List(_) | Editor::Str { multiline: true }
        )
    }
}

#[must_use = "classifying the type has no other effect"]
pub fn value_shape(type_name: &str) -> ValueShape {
    let (_, rest) = strip_nullable(type_name);
    let lowered = strip_parens(rest).to_lowercase();
    let mut lower = lowered.as_str();

    while let Some(inner) = lower.strip_prefix("non-empty (") {
        lower = inner.strip_suffix(')').unwrap_or(inner);
    }

    if lower.starts_with("list of")
        || lower.starts_with("non-empty list of")
        || lower == "lines"
        || lower.starts_with("strings concatenated with")
    {
        ValueShape::List
    } else if lower.starts_with("attribute set")
        || lower.starts_with("lazy attribute set")
        || lower.starts_with("submodule")
        || lower.starts_with("open submodule")
    {
        ValueShape::Attrs
    } else {
        ValueShape::Scalar
    }
}

pub fn choice_text(value: &NixValue) -> String {
    match value {
        NixValue::Str(text) => text.clone(),
        other @ (NixValue::Bool(_)
        | NixValue::Int(_)
        | NixValue::Float(_)
        | NixValue::List(_)
        | NixValue::Attrs(_)
        | NixValue::Null
        | NixValue::Raw(_)) => render_value(other),
    }
}

#[must_use = "parsing the type has no other effect"]
pub fn parse_type(type_name: &str) -> OptionType {
    let (nullable, rest) = strip_nullable(type_name);

    OptionType {
        nullable,
        editor: parse_editor(rest, 0),
    }
}

fn strip_nullable(type_name: &str) -> (bool, &str) {
    let mut rest = type_name.trim();
    let mut nullable = false;

    while let Some(stripped) = rest.strip_prefix("null or ") {
        nullable = true;
        rest = strip_parens(stripped.trim());
    }

    (nullable, rest)
}

fn strip_parens(text: &str) -> &str {
    let trimmed = text.trim();

    trimmed
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .filter(|_| balanced_outer(trimmed))
        .unwrap_or(trimmed)
}

fn balanced_outer(text: &str) -> bool {
    let mut depth = 0_usize;

    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);

                if depth == 0 && index + 1 != text.len() {
                    return false;
                }
            }
            _ => {}
        }
    }

    depth == 0
}

fn parse_editor(text: &str, depth: usize) -> Editor {
    // Nesting this deep is never editable. The cap also bounds recursion.
    if depth > 4 {
        return Editor::Raw;
    }

    let text = strip_parens(text);
    let lower = text.to_lowercase();

    if let Some(choices) = text.strip_prefix("one of ") {
        return parse_choices(choices).map_or(Editor::Raw, Editor::Enum);
    }

    if let Some(inner) = lower.strip_prefix("non-empty (") {
        let inner = inner.strip_suffix(')').unwrap_or(inner);
        return parse_editor(inner, depth + 1);
    }

    if let Some(inner) = text.strip_prefix("list of ") {
        return match parse_editor(inner, depth + 1) {
            Editor::List(_) | Editor::Raw => Editor::Raw,
            element @ (Editor::Bool
            | Editor::Int { min: _, max: _ }
            | Editor::Float
            | Editor::Str { multiline: _ }
            | Editor::Path
            | Editor::Enum(_)
            | Editor::Package) => Editor::List(Box::new(element)),
        };
    }

    if lower.contains(" or ") {
        return Editor::Raw;
    }

    if lower == "boolean" {
        return Editor::Bool;
    }

    if lower.starts_with("strings concatenated with") || lower == "lines" {
        return Editor::Str { multiline: true };
    }

    if lower.contains("integer") {
        return parse_int(&lower);
    }

    if lower.contains("floating point") || lower == "float" {
        return Editor::Float;
    }

    if lower == "package" {
        return Editor::Package;
    }

    if lower == "absolute path" || lower == "path" {
        return Editor::Path;
    }

    if lower.starts_with("string")
        || lower.starts_with("non-empty string")
        || lower.starts_with("single-line string")
        || lower == "str"
    {
        return Editor::Str { multiline: false };
    }

    Editor::Raw
}

fn parse_int(lower: &str) -> Editor {
    let (mut min, mut max) = (None, None);

    if let Some(range) = lower.split("between ").nth(1) {
        let numbers: Vec<i64> = range
            .split(|ch: char| !(ch.is_ascii_digit() || ch == '-'))
            .filter_map(|token| token.parse().ok())
            .take(2)
            .collect();

        if let [low, high] = numbers[..] {
            min = Some(low);
            max = Some(high);
        }
    }

    if min.is_none() {
        min = if lower.contains(">=0") || lower.starts_with("unsigned") {
            Some(0)
        } else if lower.contains(">0") || lower.starts_with("positive") {
            Some(1)
        } else {
            None
        };
    }

    Editor::Int { min, max }
}

fn parse_choices(text: &str) -> Option<Vec<NixValue>> {
    let mut chars = text.chars().peekable();
    let mut choices = Vec::new();

    loop {
        while chars
            .next_if(|ch| *ch == ',' || ch.is_whitespace())
            .is_some()
        {}

        let choice = match chars.peek() {
            None => break,
            Some('"') => {
                chars.next();
                NixValue::Str(quoted_choice(&mut chars)?)
            }
            Some(_) => {
                let token: String = std::iter::from_fn(|| chars.next_if(|ch| *ch != ',')).collect();
                bare_choice(token.trim())?
            }
        };

        choices.push(choice);
    }

    (!choices.is_empty()).then_some(choices)
}

fn quoted_choice(chars: &mut Peekable<Chars<'_>>) -> Option<String> {
    let mut value = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => value.extend(chars.next()),
            '"' => return Some(value),
            _ => value.push(ch),
        }
    }

    None
}

fn bare_choice(token: &str) -> Option<NixValue> {
    match token {
        "null" => Some(NixValue::Null),
        "true" => Some(NixValue::Bool(true)),
        "false" => Some(NixValue::Bool(false)),
        _ => token.parse().ok().map(NixValue::Int),
    }
}
