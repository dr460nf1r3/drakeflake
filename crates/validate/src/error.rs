use std::fmt;

use crate::source::Site;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    NotBool(String),
    NotWholeNumber(String),
    BelowMinimum { value: i64, minimum: i64 },
    AboveMaximum { value: i64, maximum: i64 },
    NotNumber(String),
    RelativePath,
    NotOneOf(Vec<String>),
    NotPackage(String),
    Item { number: usize, cause: Box<Self> },
    EmptyExpression,
    Unbalanced(char),
    UnterminatedString,
    Unclosed(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionError {
    Empty,
    Syntax {
        message: String,
        position: Option<Position>,
    },
    NotOneExpression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    At(Site),
    End { line: usize },
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotBool(text) => write!(f, "Expected true or false, got \"{text}\""),
            Self::NotWholeNumber(text) => write!(f, "Expected a whole number, got \"{text}\""),
            Self::BelowMinimum { value, minimum } => {
                write!(f, "{value} is below the minimum of {minimum}")
            }
            Self::AboveMaximum { value, maximum } => {
                write!(f, "{value} is above the maximum of {maximum}")
            }
            Self::NotNumber(text) => write!(f, "Expected a number, got \"{text}\""),
            Self::RelativePath => f.write_str("Paths must be absolute (start with /)"),
            Self::NotOneOf(allowed) => write!(f, "Must be one of: {}", allowed.join(", ")),
            Self::NotPackage(text) => {
                write!(f, "\"{text}\" is not a package attribute name")
            }
            Self::Item { number, cause } => write!(f, "Item {number}: {cause}"),
            Self::EmptyExpression => f.write_str("Enter a Nix expression"),
            Self::Unbalanced(ch) => write!(f, "Unbalanced \"{ch}\""),
            Self::UnterminatedString => f.write_str("Unterminated string"),
            Self::Unclosed(ch) => write!(f, "Unclosed \"{ch}\""),
        }
    }
}

impl fmt::Display for ExpressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("Enter a Nix expression"),
            Self::Syntax {
                message,
                position: _,
            } => f.write_str(message),
            Self::NotOneExpression => {
                f.write_str("Enter one expression: it closes a parenthesis it did not open")
            }
        }
    }
}

impl std::error::Error for InputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Item { number: _, cause } => Some(cause.as_ref()),
            Self::NotBool(_)
            | Self::NotWholeNumber(_)
            | Self::BelowMinimum {
                value: _,
                minimum: _,
            }
            | Self::AboveMaximum {
                value: _,
                maximum: _,
            }
            | Self::NotNumber(_)
            | Self::RelativePath
            | Self::NotOneOf(_)
            | Self::NotPackage(_)
            | Self::EmptyExpression
            | Self::Unbalanced(_)
            | Self::UnterminatedString
            | Self::Unclosed(_) => None,
        }
    }
}

impl std::error::Error for ExpressionError {}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::At(Site { line, column }) => write!(f, "at line {line}, column {column}"),
            Self::End { line } => write!(f, "at the end (line {line})"),
        }
    }
}
