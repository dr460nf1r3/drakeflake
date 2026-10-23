use std::collections::BTreeMap;

use drakeflake_render::{HostName, render_attr_name, render_string};

use crate::option::RenderedValue;

const INDENT: &str = "  ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting {
    pub path: Vec<String>,
    pub value: RenderedValue,
}

#[derive(Debug, Default)]
pub struct Body {
    pub imports: Vec<String>,
    pub preamble: Vec<String>,
    pub shared: Vec<Setting>,
    pub hosts: BTreeMap<HostName, Vec<Setting>>,
}

fn attrpath(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| render_attr_name(part))
        .collect::<Vec<_>>()
        .join(".")
}

/// Continuation lines are indented like the first, unless the value is verbatim user text, where indentation could change strings.
fn assignment(indent: &str, path: &[String], value: &RenderedValue) -> String {
    let text = if value.verbatim {
        value.text.clone()
    } else {
        value.text.replace('\n', &format!("\n{indent}"))
    };

    format!("{indent}{} = {text};", attrpath(path))
}

fn common_prefix(group: &[(&[String], &RenderedValue)]) -> usize {
    let Some(((first, _), rest)) = group.split_first() else {
        return 0;
    };

    if rest.is_empty() {
        return 0;
    }

    let mut len = 0;

    while let Some(component) = first.get(len)
        && group
            .iter()
            .all(|(path, _)| path.len() > len + 1 && path.get(len) == Some(component))
    {
        len += 1;
    }

    len
}

fn tree(items: &[(&[String], &RenderedValue)], depth: usize, out: &mut Vec<String>) {
    let indent = INDENT.repeat(depth);
    let mut rest = items;

    while let Some(((first, _), _)) = rest.split_first() {
        let head = first.first();
        let count = rest
            .iter()
            .take_while(|(path, _)| path.first() == head)
            .count();

        let (group, tail) = rest.split_at(count);
        rest = tail;

        let prefix = common_prefix(group);

        if prefix == 0 {
            out.extend(
                group
                    .iter()
                    .map(|(path, value)| assignment(&indent, path, value)),
            );
            continue;
        }

        let inner: Vec<(&[String], &RenderedValue)> = group
            .iter()
            .map(|(path, value)| (path.get(prefix..).unwrap_or_default(), *value))
            .collect();

        out.push(format!(
            "{indent}{} = {{",
            attrpath(first.get(..prefix).unwrap_or_default())
        ));
        tree(&inner, depth + 1, out);
        out.push(format!("{indent}}};"));
    }
}

fn settings_lines(settings: &[Setting], depth: usize) -> Vec<String> {
    let mut items: Vec<(&[String], &RenderedValue)> = settings
        .iter()
        .map(|setting| (setting.path.as_slice(), &setting.value))
        .collect();

    items.sort_by(|left, right| left.0.cmp(right.0));

    let mut out = Vec::new();
    tree(&items, depth, &mut out);
    out
}

#[must_use = "the condition is only built as text"]
pub fn host_condition(host: &str) -> String {
    format!("config.networking.hostName == {}", render_string(host))
}

fn conditional_lines(body: &Body) -> Vec<String> {
    let block = INDENT.repeat(2);
    let inner = INDENT.repeat(3);
    let mut lines = vec![format!("{INDENT}config = lib.mkMerge [")];

    if !body.preamble.is_empty() || !body.shared.is_empty() {
        lines.push(format!("{block}{{"));
        lines.extend(body.preamble.iter().map(|line| format!("{inner}{line}")));
        lines.extend(settings_lines(&body.shared, 3));
        lines.push(format!("{block}}}"));
    }

    for (host, settings) in &body.hosts {
        lines.push(format!("{block}(lib.mkIf ({}) {{", host_condition(host)));
        lines.extend(settings_lines(settings, 3));
        lines.push(format!("{block}}})"));
    }

    lines.push(format!("{INDENT}];"));
    lines
}

#[must_use = "the module is only rendered, nothing is written"]
pub fn module_text(header: &str, body: &Body) -> String {
    let mut lines = Vec::new();

    if !body.imports.is_empty() {
        lines.push(format!("{INDENT}imports = ["));
        lines.extend(
            body.imports
                .iter()
                .map(|path| format!("{INDENT}{INDENT}./{path}")),
        );
        lines.push(format!("{INDENT}];"));
    }

    let mut definitions = if body.hosts.is_empty() {
        let mut plain: Vec<String> = body
            .preamble
            .iter()
            .map(|line| format!("{INDENT}{line}"))
            .collect();

        plain.extend(settings_lines(&body.shared, 1));
        plain
    } else {
        conditional_lines(body)
    };

    if !lines.is_empty() && !definitions.is_empty() {
        lines.push(String::new());
    }

    lines.append(&mut definitions);

    format!(
        "{header}{{ config, lib, pkgs, ... }}:\n{{\n{}\n}}\n",
        lines.join("\n")
    )
}

#[cfg(test)]
mod tests;
