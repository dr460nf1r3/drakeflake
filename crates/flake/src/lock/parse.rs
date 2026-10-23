use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use super::kind::InputKind;
use super::urls::Urls;
use super::{LockedInput, NestedInput};
use crate::error::LockError;

const LOCK_VERSION: u32 = 7;

#[derive(Deserialize)]
struct LockFile {
    version: Option<u32>,
    nodes: Option<BTreeMap<String, Node>>,
    root: Option<String>,
}

#[derive(Default, Deserialize)]
struct Node {
    #[serde(default)]
    inputs: BTreeMap<String, InputRef>,
    locked: Option<Attrs>,
    original: Option<Attrs>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum InputRef {
    Node(String),
    Follows(Vec<String>),
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct Attrs {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub host: Option<String>,
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    pub rev: Option<String>,
    pub url: Option<String>,
    pub path: Option<String>,
    pub id: Option<String>,
    pub last_modified: Option<i64>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

pub fn read_lock(flake_dir: &Path) -> Result<Vec<LockedInput>, LockError> {
    let path = flake_dir.join("flake.lock");
    let text = std::fs::read_to_string(&path).map_err(|source| LockError::Read {
        path: path.clone(),
        source,
    })?;

    let inputs = parse_lock(&text).inspect_err(|err| log::warn!("{}: {err}", path.display()))?;

    log::debug!("{}: {} direct inputs", path.display(), inputs.len());
    Ok(inputs)
}

pub fn parse_lock(text: &str) -> Result<Vec<LockedInput>, LockError> {
    let lock: LockFile = serde_json::from_str(text).map_err(|error| {
        if error.is_data() {
            LockError::Malformed(error)
        } else {
            LockError::NotJson(error)
        }
    })?;

    let nodes = lock.nodes.ok_or(LockError::NoNodes)?;

    if lock.version.is_none_or(|version| version < LOCK_VERSION) {
        return Err(LockError::Version(lock.version));
    }

    let root = nodes
        .get(lock.root.as_deref().unwrap_or("root"))
        .ok_or(LockError::NoRootNode)?;

    let mut described = Vec::with_capacity(root.inputs.len());

    for (name, target) in &root.inputs {
        described.push(match target {
            InputRef::Follows(path) => LockedInput {
                name: name.clone(),
                kind: InputKind::Follows,
                follows: Some(path.join("/")),
                ..LockedInput::default()
            },
            InputRef::Node(key) => describe(name, node_of(&nodes, name, key)?),
        });
    }

    // Separate pass: nested inputs are compared with every direct input.
    let nested = root
        .inputs
        .values()
        .map(|target| match target {
            InputRef::Node(key) => nodes.get(key).map_or_else(
                || Ok(Vec::new()),
                |node| nested_inputs(node, &nodes, &root.inputs, &described),
            ),
            InputRef::Follows(_) => Ok(Vec::new()),
        })
        .collect::<Result<Vec<_>, LockError>>()?;

    for (input, children) in described.iter_mut().zip(nested) {
        input.nested = children;
    }

    Ok(described)
}

fn node_of<'a>(
    nodes: &'a BTreeMap<String, Node>,
    name: &str,
    key: &str,
) -> Result<&'a Node, LockError> {
    nodes.get(key).ok_or_else(|| LockError::MissingNode {
        input: name.to_owned(),
        node: key.to_owned(),
    })
}

fn nested_inputs(
    node: &Node,
    nodes: &BTreeMap<String, Node>,
    roots: &BTreeMap<String, InputRef>,
    described: &[LockedInput],
) -> Result<Vec<NestedInput>, LockError> {
    node.inputs
        .iter()
        .map(|(name, target)| match target {
            InputRef::Follows(path) => Ok(NestedInput {
                name: name.clone(),
                follows: Some(path.join("/")),
                follows_root: path.first().is_some_and(|first| roots.contains_key(first)),
                ..NestedInput::default()
            }),
            InputRef::Node(key) => {
                let child = describe(name, node_of(nodes, name, key)?);
                let same_as = roots
                    .iter()
                    .find(|(_, root)| matches!(root, InputRef::Node(root_key) if root_key == key))
                    .map(|(root, _)| root.clone());

                let could_follow = if same_as.is_some() {
                    None
                } else {
                    described
                        .iter()
                        .find(|root| root.follows.is_none() && same_repository(root, &child))
                        .map(|root| root.name.clone())
                };

                Ok(NestedInput {
                    name: name.clone(),
                    source: Some(child.source),
                    rev: child.rev,
                    last_modified: child.last_modified,
                    same_as,
                    could_follow,
                    ..NestedInput::default()
                })
            }
        })
        .collect()
}

fn same_repository(left: &LockedInput, right: &LockedInput) -> bool {
    let identity = |input: &LockedInput| {
        input
            .web_url
            .as_deref()
            .unwrap_or(&input.source)
            .to_ascii_lowercase()
    };

    !left.source.is_empty() && identity(left) == identity(right)
}

fn describe(name: &str, node: &Node) -> LockedInput {
    let empty = Attrs::default();
    let locked = node.locked.as_ref().unwrap_or(&empty);
    let original = node.original.as_ref().unwrap_or(&empty);
    let kind = locked
        .kind
        .as_deref()
        .map(InputKind::from)
        .unwrap_or_default();

    let urls = Urls::for_node(&kind, locked, original);

    LockedInput {
        name: name.to_owned(),
        source: source_text(&kind, original),
        reference: original.reference.clone(),
        last_modified: locked.last_modified,
        web_url: urls.web,
        commit_url: urls.commit,
        remote: urls.remote,
        locked_url: locked.url.clone(),
        pin_base: pin_base(&kind, node.original.as_ref()),
        pinned: original.rev.is_some(),
        nested: Vec::new(),
        rev: locked.rev.clone(),
        kind,
        follows: None,
    }
}

fn pin_base(kind: &InputKind, original: Option<&Attrs>) -> Option<String> {
    let original = original?;
    let only_repo = original.other.is_empty()
        && [
            &original.host,
            &original.rev,
            &original.url,
            &original.path,
            &original.id,
        ]
        .iter()
        .all(|attribute| attribute.is_none());

    (kind.is_forge() && only_repo).then(|| format!("{kind}:{}", forge_path(original)))
}

fn forge_path(original: &Attrs) -> String {
    format!(
        "{}/{}",
        flake_ref_owner(original),
        original.repo.as_deref().unwrap_or_default()
    )
}

fn source_text(kind: &InputKind, original: &Attrs) -> String {
    let reference = original.reference.as_deref();

    match kind {
        InputKind::Github | InputKind::Gitlab | InputKind::Sourcehut => {
            let base = format!("{kind}:{}", forge_path(original));

            match reference {
                Some(reference) => format!("{base}/{reference}"),
                None => base,
            }
        }
        InputKind::Path => original.path.clone().unwrap_or_default(),
        InputKind::Indirect => format!("flake:{}", original.id.as_deref().unwrap_or_default()),
        InputKind::Git
        | InputKind::Tarball
        | InputKind::File
        | InputKind::Follows
        | InputKind::Other(_) => match (original.url.as_deref(), reference) {
            (Some(url), Some(reference)) => format!("{url}?ref={reference}"),
            (Some(url), None) => url.to_owned(),
            (None, _) => kind.to_string(),
        },
    }
}

/// The owner as written in a flake reference. Nix decodes percent escapes in
/// it once, so a subgroup's `%2F` from the lock file becomes `%252F` again.
fn flake_ref_owner(original: &Attrs) -> String {
    original
        .owner
        .as_deref()
        .unwrap_or_default()
        .replace('%', "%25")
}

#[cfg(test)]
mod tests;
