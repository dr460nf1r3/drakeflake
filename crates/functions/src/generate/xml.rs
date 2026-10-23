//! Nix writes every value as an element with attributes and no text, so the reader only handles tags, quoted attributes and entities.

use std::collections::HashMap;

use anyhow::{Context as _, Result, bail};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RawPos {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collected {
    pub path: String,
    pub attr: Option<RawPos>,
    pub lambda: Option<RawPos>,
    pub args: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<Self>,
}

impl Element {
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(String::as_str)
    }

    fn number(&self, name: &str) -> Option<u32> {
        self.attribute(name)?.parse().ok()
    }

    fn field(&self, name: &str) -> Option<&Self> {
        self.children
            .iter()
            .find(|child| child.name == "attr" && child.attribute("name") == Some(name))?
            .children
            .first()
    }
}

pub fn collected(text: &str) -> Result<Vec<Collected>> {
    let document = parse(text)?;
    let Some(list) = document.children.first().filter(|list| list.name == "list") else {
        bail!("the positions are not a list");
    };

    list.children
        .iter()
        .map(|item| {
            let path = item
                .field("path")
                .and_then(|value| value.attribute("value"))
                .context("an entry has no path")?;

            let lambda = item
                .field("lambda")
                .filter(|value| value.name == "function");

            Ok(Collected {
                path: path.to_owned(),
                attr: item.field("attr").and_then(attr_position),
                lambda: lambda.and_then(|function| {
                    Some(RawPos {
                        file: function.attribute("path")?.to_owned(),
                        line: function.number("line")?,
                        column: function.number("column")?,
                    })
                }),
                args: lambda.map(arguments).unwrap_or_default(),
            })
        })
        .collect()
}

fn attr_position(value: &Element) -> Option<RawPos> {
    (value.name == "attrs").then_some(())?;

    Some(RawPos {
        file: value.field("file")?.attribute("value")?.to_owned(),
        line: value.field("line")?.number("value")?,
        column: value.field("column")?.number("value")?,
    })
}

fn arguments(function: &Element) -> Vec<String> {
    function
        .children
        .iter()
        .flat_map(|pattern| match pattern.name.as_str() {
            "attrspat" => pattern
                .children
                .iter()
                .filter_map(|attr| attr.attribute("name"))
                .map(str::to_owned)
                .collect(),
            "varpat" => pattern
                .attribute("name")
                .map(str::to_owned)
                .into_iter()
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

pub fn parse(text: &str) -> Result<Element> {
    let mut stack: Vec<Element> = vec![Element::default()];
    let mut rest = text;

    while let Some(open) = rest.find('<') {
        rest = rest.get(open + 1..).unwrap_or("");

        if let Some(after) = rest.strip_prefix('?') {
            rest = after.split_once("?>").map_or("", |(_, after)| after);
        } else if let Some(after) = rest.strip_prefix("!--") {
            rest = after.split_once("-->").map_or("", |(_, after)| after);
        } else if let Some(after) = rest.strip_prefix('/') {
            rest = after.split_once('>').map_or("", |(_, after)| after);
            let element = stack
                .pop()
                .context("a closing tag without an opening one")?;

            stack
                .last_mut()
                .context("a closing tag without an opening one")?
                .children
                .push(element);
        } else {
            let (element, closed, after) = tag(rest)?;
            rest = after;
            if closed {
                stack
                    .last_mut()
                    .context("an element outside the document")?
                    .children
                    .push(element);
            } else {
                stack.push(element);
            }
        }
    }

    match stack.pop() {
        Some(document) if stack.is_empty() => document
            .children
            .into_iter()
            .next()
            .context("the XML has no document element"),
        _ => bail!("the XML ends inside an element"),
    }
}

fn tag(text: &str) -> Result<(Element, bool, &str)> {
    let name_end = text
        .find(|ch: char| ch.is_whitespace() || ch == '>' || ch == '/')
        .context("a tag does not end")?;

    let mut element = Element {
        name: text.get(..name_end).unwrap_or("").to_owned(),
        ..Element::default()
    };

    let mut rest = text.get(name_end..).unwrap_or("");

    loop {
        rest = rest.trim_start();

        if let Some(after) = rest.strip_prefix("/>") {
            return Ok((element, true, after));
        }

        if let Some(after) = rest.strip_prefix('>') {
            return Ok((element, false, after));
        }

        let (key, after) = rest.split_once('=').context("an attribute has no value")?;
        let after = after.trim_start();
        let quote = after.chars().next().context("an attribute has no value")?;
        let value = after.get(1..).unwrap_or("");
        let (value, remainder) = value
            .split_once(quote)
            .context("an attribute value does not end")?;

        element
            .attributes
            .insert(key.trim().to_owned(), unescape(value));
        rest = remainder;
    }
}

fn unescape(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find('&') {
        let (before, entity) = rest.split_at(start);
        result.push_str(before);

        let Some(end) = entity.find(';') else {
            result.push_str(entity);
            return result;
        };

        let name = entity.get(1..end).unwrap_or("");
        let decoded = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => name
                .strip_prefix("#x")
                .map_or_else(
                    || {
                        let digits = name.strip_prefix('#')?;
                        digits.parse().ok()
                    },
                    |hex| u32::from_str_radix(hex, 16).ok(),
                )
                .and_then(char::from_u32),
        };

        match decoded {
            Some(ch) => result.push(ch),
            None => result.push_str(entity.get(..=end).unwrap_or("")),
        }

        rest = entity.get(end + 1..).unwrap_or("");
    }

    result.push_str(rest);
    result
}
