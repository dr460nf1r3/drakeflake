//! The text is embedded as `(EXPR)`, so rnix checks it cannot leave that slot. `nix-instantiate` has the final say.

use drakeflake_render::delimiters;
use rnix::{ParseError, SyntaxKind, TextRange, ast};

use crate::error::{ExpressionError, Position};
use crate::source::site_at;

const CONTEXT: &str = "{ config, lib, pkgs, ... }:\n";

fn token_text(kind: SyntaxKind) -> String {
    let name = format!("{kind:?}");
    let short = name
        .strip_prefix("TOKEN_")
        .or_else(|| name.strip_prefix("NODE_"))
        .unwrap_or(&name);

    match short {
        "L_PAREN" => "\"(\"".to_owned(),
        "R_PAREN" => "\")\"".to_owned(),
        "L_BRACK" => "\"[\"".to_owned(),
        "R_BRACK" => "\"]\"".to_owned(),
        "L_BRACE" => "\"{\"".to_owned(),
        "R_BRACE" => "\"}\"".to_owned(),
        "SEMICOLON" => "\";\"".to_owned(),
        "ASSIGN" => "\"=\"".to_owned(),
        "COLON" => "\":\"".to_owned(),
        "COMMA" => "\",\"".to_owned(),
        "DOT" => "\".\"".to_owned(),
        other => other.to_lowercase().replace('_', " "),
    }
}

fn wanted_text(kinds: &[SyntaxKind]) -> String {
    let names: Vec<String> = kinds.iter().copied().map(token_text).collect();
    names.join(", ")
}

fn position(expression: &str, start: usize, offset: usize) -> Position {
    let relative = offset.saturating_sub(start).min(expression.len());
    let site = site_at(expression, relative);

    if offset >= start + expression.len() {
        Position::End { line: site.line }
    } else {
        Position::At(site)
    }
}

fn describe(error: &ParseError, expression: &str, start: usize) -> ExpressionError {
    let at = |range: &TextRange| position(expression, start, usize::from(range.start()));
    let placed = |message: String, place: Position| ExpressionError::Syntax {
        message,
        position: Some(place),
    };

    let unplaced = |message: String| ExpressionError::Syntax {
        message,
        position: None,
    };

    match error {
        ParseError::Unexpected(range) => {
            let place = at(range);
            placed(format!("Unexpected input {place}"), place)
        }
        ParseError::UnexpectedExtra(range) => {
            let place = at(range);
            placed(format!("Unexpected text {place}"), place)
        }
        ParseError::UnexpectedWanted(got, range, wanted) => {
            let place = at(range);
            placed(
                format!(
                    "Unexpected {} {place}, expected {}",
                    token_text(*got),
                    wanted_text(wanted)
                ),
                place,
            )
        }
        ParseError::UnexpectedDoubleBind(range) => {
            let place = at(range);
            placed(format!("Pattern bound twice {place}"), place)
        }
        ParseError::UnexpectedEOF => unplaced("The expression ends too early".to_owned()),
        ParseError::UnexpectedEOFWanted(wanted) => unplaced(format!(
            "The expression ends too early, expected {}",
            wanted_text(wanted)
        )),
        ParseError::DuplicatedArgs(range, name) => {
            let place = at(range);
            placed(format!("Argument \"{name}\" is duplicated {place}"), place)
        }
        ParseError::RecursionLimitExceeded => {
            unplaced("The expression is nested too deeply".to_owned())
        }
        other => unplaced(other.to_string()),
    }
}

fn body_is_the_slot(root: &ast::Root, open: usize, end: usize) -> bool {
    let Some(ast::Expr::Lambda(lambda)) = root.expr() else {
        return false;
    };

    let Some(ast::Expr::Paren(paren)) = lambda.body() else {
        return false;
    };

    let (Some(left), Some(right)) = (paren.l_paren_token(), paren.r_paren_token()) else {
        return false;
    };

    usize::from(left.text_range().start()) == open && usize::from(right.text_range().end()) == end
}

pub fn parse_expression(text: &str) -> Result<String, ExpressionError> {
    let expression = text.trim();

    if expression.is_empty() {
        return Err(ExpressionError::Empty);
    }

    let (open, close) = delimiters(expression);
    let source = format!("{CONTEXT}{open}{expression}{close}");
    let start = CONTEXT.len() + open.len();
    let parsed = rnix::Root::parse(&source);

    if let Some(error) = parsed.errors().first() {
        return Err(describe(error, expression, start));
    }

    if !body_is_the_slot(&parsed.tree(), CONTEXT.len(), source.len()) {
        return Err(ExpressionError::NotOneExpression);
    }

    Ok(expression.to_owned())
}

#[cfg(test)]
mod tests;
