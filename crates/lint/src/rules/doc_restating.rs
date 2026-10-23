use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::finding::Finding;
use crate::pattern::re;
use crate::source::{Kind, Source, Unit};
use crate::words::{WHY_WORDS, identifier_words, prose_words};

const RULE: &str = "no-restating-doc";

const GENERIC: &[&str] = &[
    "method",
    "function",
    "class",
    "service",
    "component",
    "helper",
    "value",
    "instance",
    "object",
    "property",
    "field",
    "variable",
    "constant",
    "handler",
    "data",
    "info",
    "information",
    "list",
    "parameter",
    "argument",
    "struct",
    "enum",
    "variant",
    "type",
    "output",
    "path",
    "file",
    "directory",
    "folder",
    "absolute",
    "relative",
    "option",
    "setting",
    "priority",
    "command",
    "package",
    "input",
    "revision",
    "name",
    "condition",
    "host",
    "entry",
    "user",
    "text",
    "number",
];
const GENERIC_RETURNS: &[&str] = &[
    "result", "value", "returned", "data", "response", "output", "promise", "object", "nothing",
    "void", "true", "false", "boolean", "string", "number", "array", "list", "get",
];

static PARAM_TAG: LazyLock<Regex> =
    LazyLock::new(|| re(r"^@param\s+(?:\{[^}]*\}\s+)?\[?([\w$.]+)\S*\s*(?:-\s*)?(.*)$"));
static RETURNS_TAG: LazyLock<Regex> = LazyLock::new(|| re(r"^@returns?\s+(?:\{[^}]*\}\s+)?(.*)$"));

pub fn check(source: &Source<'_>, findings: &mut Vec<Finding>) {
    for unit in source
        .units()
        .iter()
        .filter(|unit| unit.kind() == Kind::Doc)
    {
        let Some(name) = source
            .documented_item(unit)
            .and_then(|item| item.child_by_field_name("name"))
            .map(|name| source.node_text(name))
        else {
            continue;
        };

        let name_words: HashSet<String> = identifier_words(name).into_iter().collect();

        if only_known_words(&description(unit), &name_words, GENERIC) {
            findings.push(
                Finding::new(
                    unit.start(),
                    RULE,
                    format!("The doc description of `{name}` only repeats its name."),
                    "A doc comment that restates the signature costs reading time and adds nothing the editor does not already show.",
                )
                .fix("Delete it, or say what the name cannot: preconditions, side effects, units, the rule behind it."),
            );
        }

        check_tags(unit, &name_words, findings);
    }
}

fn check_tags(unit: &Unit<'_, '_>, name_words: &HashSet<String>, findings: &mut Vec<Finding>) {
    for line in unit.lines() {
        let text = line.text.trim();

        if let Some(param) = PARAM_TAG.captures(text) {
            let name = param.get(1).map_or("", |group| group.as_str());
            let name = name.rsplit('.').next().unwrap_or(name);
            let words: HashSet<String> = identifier_words(name).into_iter().collect();
            if only_known_words(
                param.get(2).map_or("", |group| group.as_str()),
                &words,
                GENERIC,
            ) {
                findings.push(
                    Finding::new(
                        line.at,
                        RULE,
                        format!("`@param {name}` only repeats the parameter name."),
                        "The name and type already say this.",
                    )
                    .fix("Delete the tag, or say what is not obvious: allowed values, units, where it comes from, what happens when it is empty."),
                );
            }

            continue;
        }

        if let Some(returns) = RETURNS_TAG.captures(text) {
            let what = returns.get(1).map_or("", |group| group.as_str());
            if only_known_words(what, name_words, GENERIC_RETURNS) {
                findings.push(
                    Finding::new(
                        line.at,
                        RULE,
                        format!("`@returns {what}` says nothing about the result."),
                        "The return type already says this.",
                    )
                    .fix("Delete the tag, or describe the result: its shape, ordering, units, or when it is empty."),
                );
            }
        }
    }
}

fn description(unit: &Unit<'_, '_>) -> String {
    unit.lines()
        .map(|line| line.text.trim())
        .take_while(|text| {
            !(text.starts_with("# ") || text.starts_with('@') || text.starts_with("```"))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn only_known_words(text: &str, known: &HashSet<String>, generic: &[&str]) -> bool {
    if text.trim().is_empty() || WHY_WORDS.is_match(text) {
        return false;
    }

    let words = prose_words(text);
    !words.is_empty()
        && words
            .iter()
            .all(|word| known.contains(word) || generic.contains(&word.as_str()))
}
