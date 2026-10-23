use std::fmt;
use std::str::FromStr;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TypeKind {
    String,
    Number,
    Bool,
    Path,
    List,
    Attrset,
    Function,
    Derivation,
    Null,
    Any,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Shape {
    pub takes: Option<TypeKind>,
    pub returns: Option<TypeKind>,
}

impl TypeKind {
    pub const ALL: [Self; 10] = [
        Self::String,
        Self::Number,
        Self::Bool,
        Self::Path,
        Self::List,
        Self::Attrset,
        Self::Function,
        Self::Derivation,
        Self::Null,
        Self::Any,
    ];

    fn of(text: &str) -> Self {
        let text = text.trim();

        if text.starts_with('[') {
            return Self::List;
        }

        if text.starts_with('{') {
            return Self::Attrset;
        }

        if let Some(inner) = text
            .strip_prefix('(')
            .and_then(|rest| rest.strip_suffix(')'))
            && split_arrows(inner).len() > 1
        {
            return Self::Function;
        }

        let word = text
            .trim_start_matches('(')
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_lowercase();

        match word.as_str() {
            "string" | "str" => Self::String,
            "int" | "integer" | "float" | "number" => Self::Number,
            "bool" | "boolean" => Self::Bool,
            "path" => Self::Path,
            "list" | "listof" => Self::List,
            "attrset" | "attrs" | "attrsof" | "set" | "attributeset" => Self::Attrset,
            "function" | "lambda" => Self::Function,
            "derivation" | "package" | "drv" => Self::Derivation,
            "null" => Self::Null,
            _ => Self::Any,
        }
    }
}

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Bool => "bool",
            Self::Path => "path",
            Self::List => "list",
            Self::Attrset => "attrset",
            Self::Function => "function",
            Self::Derivation => "derivation",
            Self::Null => "null",
            Self::Any => "any",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownTypeKind;

impl FromStr for TypeKind {
    type Err = UnknownTypeKind;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.to_string() == name)
            .ok_or(UnknownTypeKind)
    }
}

impl Shape {
    #[must_use = "parsing the signature has no other effect"]
    pub fn of(signature: &str) -> Self {
        let flat = signature.split_whitespace().collect::<Vec<_>>().join(" ");

        // Only a `::` outside brackets names the function: `{ a :: Int } -> X`
        // has no name.
        let body = top_level_colons(&flat)
            .and_then(|at| flat.get(at + 2..))
            .unwrap_or(flat.as_str());

        let parts = split_arrows(body);

        match (parts.first(), parts.last()) {
            (Some(first), Some(last)) if parts.len() > 1 => Self {
                takes: Some(TypeKind::of(first)),
                returns: Some(TypeKind::of(last)),
            },
            _ => Self::default(),
        }
    }
}

fn top_level_colons(text: &str) -> Option<usize> {
    let mut depth = 0_usize;
    let mut previous = ' ';

    for (offset, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ':' if previous == ':' && depth == 0 => return offset.checked_sub(1),
            _ => {}
        }

        previous = ch;
    }

    None
}

fn split_arrows(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0_usize;
    let mut start = 0;
    let mut previous = ' ';

    for (offset, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            '>' if previous == '-' && depth == 0 => {
                if let Some(part) = text.get(start..offset.saturating_sub(1)) {
                    parts.push(part.trim());
                }

                start = offset + 1;
            }
            _ => {}
        }

        previous = ch;
    }

    if let Some(rest) = text.get(start..) {
        parts.push(rest.trim());
    }

    parts.retain(|part| !part.is_empty());
    parts
}

#[must_use = "extracting the signature has no other effect"]
pub fn signature_from_doc(doc: &str) -> String {
    let mut lines = doc
        .lines()
        .skip_while(|line| !is_type_heading(line))
        .skip(1);

    let mut signature = Vec::new();

    for line in lines.by_ref() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            break;
        }

        if trimmed.starts_with('#') {
            return String::new();
        }
    }

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            break;
        }

        if !trimmed.is_empty() {
            signature.push(trimmed);
        }
    }

    signature.join(" ")
}

fn is_type_heading(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('#') && trimmed.trim_start_matches('#').trim() == "Type"
}
