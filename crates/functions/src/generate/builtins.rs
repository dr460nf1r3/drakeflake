use anyhow::{Context as _, Result};
use drakeflake_functions::Function;
use serde_json::{Map, Value};

const MANUAL: &str = "https://nix.dev/manual/nix/latest";

pub fn parse(text: &str) -> Result<Vec<Function>> {
    let dump: Value = serde_json::from_str(text).context("the builtins dump is not JSON")?;
    let top = dump
        .as_object()
        .context("the builtins dump is not an object")?;

    // Older Nix versions nest them as `{ "builtins": { … }, "constants": … }`.
    let entries: &Map<String, Value> = match top.get("builtins").and_then(Value::as_object) {
        Some(nested) if !nested.contains_key("doc") => nested,
        _ => top,
    };

    Ok(entries
        .iter()
        .filter_map(|(name, entry)| {
            let args: Vec<String> = entry
                .get("args")?
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();

            Some(Function {
                path: format!("builtins.{name}"),
                doc: manual_links(entry.get("doc").and_then(Value::as_str).unwrap_or("")),
                args,
                builtin: true,
                experimental: entry
                    .get("experimental-feature")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                impure: entry
                    .get("impure-only")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                ..Function::default()
            })
        })
        .collect())
}

pub fn manual_links(doc: &str) -> String {
    let mut result = String::with_capacity(doc.len());
    let mut rest = doc;

    while let Some(start) = rest.find("@docroot@") {
        let (before, link) = rest.split_at(start);
        let target = link.get("@docroot@".len()..).unwrap_or("");
        let end = target
            .find(|ch: char| ch == ')' || ch == '#' || ch.is_whitespace())
            .unwrap_or(target.len());

        let page = target.get(..end).unwrap_or("");

        result.push_str(before);
        result.push_str(MANUAL);

        match page.strip_suffix(".md") {
            Some(stem) => {
                result.push_str(stem);
                result.push_str(".html");
            }
            None => result.push_str(page),
        }

        rest = target.get(end..).unwrap_or("");
    }

    result.push_str(rest);
    result
}
