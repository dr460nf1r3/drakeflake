use std::collections::HashMap;

use drakeflake_functions::{Function, Position, signature_from_doc};

use crate::source::{Comment, Sources};
use crate::xml::{Collected, RawPos};

const WRAPPER_LIMIT: usize = 8;

struct Entry {
    function: Function,
    doc: Option<String>,
    doc_at_lambda: bool,
    plain: Option<String>,
    copies: Option<String>,
    lambda: Option<RawPos>,
}

struct Groups(Vec<usize>);

impl Groups {
    fn root(&mut self, index: usize) -> usize {
        let mut current = index;
        while let Some(&parent) = self.0.get(current) {
            if parent == current {
                break;
            }

            current = parent;
        }

        if let Some(slot) = self.0.get_mut(index) {
            *slot = current;
        }

        current
    }

    fn join(&mut self, first: usize, second: usize) {
        let (first_root, second_root) = (self.root(first), self.root(second));
        if let Some(slot) = self.0.get_mut(first_root.max(second_root)) {
            *slot = first_root.min(second_root);
        }
    }
}

pub fn assemble(collected: Vec<Collected>, builtins: Vec<Function>, root: &str) -> Vec<Function> {
    let mut uses: HashMap<RawPos, usize> = HashMap::new();
    for item in &collected {
        if let Some(lambda) = &item.lambda {
            *uses.entry(lambda.clone()).or_default() += 1;
        }
    }

    let mut sources = Sources::default();
    let mut entries: Vec<Entry> = collected
        .into_iter()
        .map(|mut item| {
            item.lambda = item
                .lambda
                .filter(|position| uses.get(position).copied().unwrap_or(0) <= WRAPPER_LIMIT);
            entry(&mut sources, item, root)
        })
        .collect();

    entries.extend(builtins.into_iter().map(|function| Entry {
        doc: (!function.doc.is_empty()).then(|| function.doc.clone()),
        function,
        doc_at_lambda: false,
        plain: None,
        copies: None,
        lambda: None,
    }));

    let groups = alias_groups(&entries);
    let mut functions: Vec<Function> = (0..entries.len())
        .filter_map(|index| finish(&entries, &groups, index))
        .collect();

    functions.sort_by(|left, right| left.path.cmp(&right.path));
    functions
}

fn entry(sources: &mut Sources, item: Collected, root: &str) -> Entry {
    let Collected {
        path,
        attr,
        lambda,
        args,
    } = item;

    let attr_site = attr.as_ref().and_then(|position| sources.attr(position));
    let lambda_site = lambda
        .as_ref()
        .and_then(|position| sources.lambda(position));

    let attr_comment = attr_site.as_ref().and_then(|site| site.comment.as_ref());
    let lambda_comment = lambda_site.as_ref().and_then(|site| site.comment.as_ref());
    let attr_doc = text_of(attr_comment, true);
    let lambda_doc = text_of(lambda_comment, true);
    let plain = text_of(attr_comment, false).or_else(|| text_of(lambda_comment, false));
    let code = lambda_site
        .as_ref()
        .map(|site| site.code.clone())
        .filter(|code| !code.is_empty())
        .or_else(|| attr_site.as_ref().map(|site| site.code.clone()))
        .unwrap_or_default();

    Entry {
        doc_at_lambda: attr_doc.is_none() && lambda_doc.is_some(),
        doc: attr_doc.or(lambda_doc),
        plain,
        copies: attr_site.and_then(|site| site.copies),
        function: Function {
            path,
            args,
            attr: attr.as_ref().map(|position| relative(position, root)),
            lambda: lambda.as_ref().map(|position| relative(position, root)),
            code,
            ..Function::default()
        },
        lambda,
    }
}

fn text_of(comment: Option<&Comment>, doc: bool) -> Option<String> {
    comment
        .filter(|comment| comment.doc == doc)
        .map(|comment| comment.text.clone())
}

fn alias_groups(entries: &[Entry]) -> Vec<usize> {
    let mut groups = Groups((0..entries.len()).collect());
    let mut by_lambda: HashMap<&RawPos, usize> = HashMap::new();

    for (index, entry) in entries.iter().enumerate() {
        if let Some(lambda) = &entry.lambda {
            match by_lambda.get(lambda) {
                Some(&first) => groups.join(first, index),
                None => {
                    by_lambda.insert(lambda, index);
                }
            }
        }

        if let Some(target) = entry
            .copies
            .as_deref()
            .and_then(|copies| copied(entries, index, copies))
        {
            groups.join(target, index);
        }
    }

    (0..entries.len()).map(|index| groups.root(index)).collect()
}

fn copied(entries: &[Entry], index: usize, copies: &str) -> Option<usize> {
    let segments: Vec<&str> = copies.split('.').collect();

    for skip in 0..segments.len().saturating_sub(1) {
        let ending = segments.get(skip..).unwrap_or_default().join(".");
        let dotted = format!(".{ending}");
        let found: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(other, entry)| {
                *other != index
                    && (entry.function.path == ending || entry.function.path.ends_with(&dotted))
            })
            .map(|(other, _)| other)
            .collect();

        match found.as_slice() {
            [only] => return Some(*only),
            [] => {}
            _ => return None,
        }
    }

    None
}

fn finish(entries: &[Entry], groups: &[usize], index: usize) -> Option<Function> {
    let entry = entries.get(index)?;
    let group = groups.get(index)?;
    let members: Vec<&Entry> = entries
        .iter()
        .zip(groups)
        .filter(|(_, root)| *root == group)
        .map(|(member, _)| member)
        .collect();

    let mut function = entry.function.clone();
    function.aliases = members
        .iter()
        .map(|member| member.function.path.clone())
        .filter(|path| *path != function.path)
        .collect();
    function.aliases.sort();

    let donor = if entry.doc.is_some() {
        Some(entry)
    } else {
        members
            .iter()
            .copied()
            .filter(|member| member.doc.is_some())
            .min_by_key(|member| (!member.doc_at_lambda, member.function.builtin))
    };

    match donor.and_then(|donor| Some((donor.function.path.as_str(), donor.doc.as_ref()?))) {
        Some((from, doc)) => {
            function.doc.clone_from(doc);
            if from != function.path {
                from.clone_into(&mut function.doc_from);
            }
        }
        None => function.doc = entry.plain.clone().unwrap_or_default(),
    }

    function.signature = signature_from_doc(&function.doc);
    Some(function)
}

fn relative(position: &RawPos, root: &str) -> Position {
    let file = position
        .file
        .strip_prefix(root)
        .filter(|_| !root.is_empty())
        .map_or_else(
            || position.file.clone(),
            |rest| rest.trim_start_matches('/').to_owned(),
        );

    Position {
        file,
        line: position.line,
        column: position.column,
    }
}
