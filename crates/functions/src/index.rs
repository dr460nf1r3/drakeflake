use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context as _, Result, bail};
use drakeflake_store::{Haystacks, rank};
use serde::{Deserialize, Serialize};

use crate::signature::{Shape, TypeKind};

/// Bumped whenever the file's meaning changes, so an app never shows half of
/// an index it does not understand.
pub const FORMAT_VERSION: u32 = 1;

const PATH_VARIABLE: &str = "DRAKEFLAKE_FUNCTION_INDEX";
const RELATIVE_PATH: &str = "../share/drakeflake/functions.json";
const CARD_MINIMUM: usize = 4;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexFile {
    pub version: u32,
    pub source: Source,
    pub functions: Vec<Function>,
}

#[derive(Deserialize)]
struct Header {
    version: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Source {
    pub rev: String,
    pub date: i64,
    pub nix: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Function {
    pub path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc_from: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub signature: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub builtin: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub experimental: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub impure: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attr: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lambda: Option<Position>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub code: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    pub namespace: String,
    pub takes: Option<TypeKind>,
    pub returns: Option<TypeKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Namespace {
    pub name: String,
    pub count: usize,
    pub direct: usize,
    pub card: bool,
}

#[derive(Debug, Default)]
pub struct FunctionIndex {
    source: Source,
    functions: Vec<Function>,
    summaries: Vec<String>,
    shapes: Vec<Shape>,
    haystacks: Haystacks,
}

impl Function {
    #[must_use = "the namespace is only computed"]
    pub fn namespace(&self) -> &str {
        self.path
            .rsplit_once('.')
            .map_or("", |(namespace, _)| namespace)
    }
}

impl Position {
    #[must_use = "the URL is only built, nothing is opened"]
    pub fn url(&self, rev: &str) -> Option<String> {
        (!rev.is_empty() && !self.file.is_empty() && !self.file.starts_with('/')).then(|| {
            format!(
                "https://github.com/NixOS/nixpkgs/blob/{rev}/{}#L{}",
                self.file, self.line
            )
        })
    }
}

impl Filter {
    #[must_use = "checking the filter has no other effect"]
    pub const fn is_empty(&self) -> bool {
        self.namespace.is_empty() && self.takes.is_none() && self.returns.is_none()
    }

    fn admits(&self, function: &Function, shape: Shape) -> bool {
        let in_namespace = self.namespace.is_empty()
            || function
                .path
                .strip_prefix(self.namespace.as_str())
                .is_some_and(|rest| rest.starts_with('.'));

        in_namespace
            && self.takes.is_none_or(|kind| shape.takes == Some(kind))
            && self.returns.is_none_or(|kind| shape.returns == Some(kind))
    }
}

impl FunctionIndex {
    pub fn from_json_str(text: &str) -> Result<Self> {
        let started = Instant::now();
        let header: Header =
            serde_json::from_str(text).context("the function index is not valid JSON")?;

        if header.version != FORMAT_VERSION {
            bail!(
                "the function index has format {}, this program reads format {FORMAT_VERSION}",
                header.version
            );
        }

        let file: IndexFile =
            serde_json::from_str(text).context("the function index is not valid JSON")?;

        let index = Self::from_file(file);
        log::debug!(
            "function index: {} functions from {} bytes in {:?}",
            index.len(),
            text.len(),
            started.elapsed()
        );
        Ok(index)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;

        Self::from_json_str(&text)
    }

    #[must_use = "building the index is costly and has no other effect"]
    pub fn from_file(file: IndexFile) -> Self {
        let mut functions = file.functions;
        functions.sort_by(|left, right| left.path.cmp(&right.path));
        functions.dedup_by(|later, kept| later.path == kept.path);

        let summaries: Vec<String> = functions
            .iter()
            .map(|function| summary(&function.doc))
            .collect();

        let shapes = functions
            .iter()
            .map(|function| Shape::of(&function.signature))
            .collect();

        let descriptions: Vec<String> = functions
            .iter()
            .zip(&summaries)
            .map(|(function, summary)| format!("{summary} {}", function.signature))
            .collect();

        let haystacks = Haystacks::new(
            functions
                .iter()
                .zip(&descriptions)
                .map(|(function, description)| (function.path.as_str(), description.as_str())),
        );

        Self {
            source: file.source,
            functions,
            summaries,
            shapes,
            haystacks,
        }
    }

    #[must_use = "counting has no other effect"]
    pub const fn len(&self) -> usize {
        self.functions.len()
    }

    #[must_use = "checking emptiness has no other effect"]
    pub const fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    #[must_use = "the source is only returned"]
    pub const fn source(&self) -> &Source {
        &self.source
    }

    #[must_use = "the lookup has no other effect"]
    pub fn get(&self, index: usize) -> Option<&Function> {
        self.functions.get(index)
    }

    #[must_use = "the lookup has no other effect"]
    pub fn summary(&self, index: usize) -> &str {
        self.summaries.get(index).map_or("", String::as_str)
    }

    #[must_use = "the lookup has no other effect"]
    pub fn shape(&self, index: usize) -> Shape {
        self.shapes.get(index).copied().unwrap_or_default()
    }

    #[must_use = "the lookup has no other effect"]
    pub fn position(&self, path: &str) -> Option<usize> {
        self.functions
            .binary_search_by(|function| function.path.as_str().cmp(path))
            .ok()
    }

    #[must_use = "the lookup has no other effect"]
    pub fn by_path(&self, path: &str) -> Option<&Function> {
        let index = self.position(path)?;
        self.functions.get(index)
    }

    #[must_use = "the ranked matches are the only result of the search"]
    pub fn search(&self, query: &str, filter: &Filter) -> Vec<usize> {
        rank(query, self.functions.len(), |index| {
            let function = self.functions.get(index)?;

            if filter.admits(function, self.shape(index)) {
                self.haystacks.get(index)
            } else {
                None
            }
        })
    }

    #[must_use = "counting the namespaces has no other effect"]
    pub fn namespaces(&self) -> Vec<Namespace> {
        let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();

        for function in &self.functions {
            let namespace = function.namespace();
            if namespace.is_empty() {
                continue;
            }

            counts.entry(namespace).or_default().1 += 1;

            for (end, _) in namespace.match_indices('.').chain([(namespace.len(), "")]) {
                if let Some(prefix) = namespace.get(..end) {
                    counts.entry(prefix).or_default().0 += 1;
                }
            }
        }

        counts
            .into_iter()
            .map(|(name, (count, direct))| Namespace {
                name: name.to_owned(),
                count,
                direct,
                card: direct >= CARD_MINIMUM,
            })
            .collect()
    }
}

#[must_use = "the path is only computed, nothing is read"]
pub fn index_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(PATH_VARIABLE).filter(|path| !path.is_empty()) {
        return Some(PathBuf::from(path));
    }

    let binary = std::env::current_exe().ok()?;
    Some(binary.parent()?.join(RELATIVE_PATH))
}

fn summary(doc: &str) -> String {
    doc.lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| {
            !line.is_empty()
                && !line.starts_with('#')
                && !line.starts_with("```")
                && !line.starts_with(":::")
        })
        .collect::<Vec<_>>()
        .join(" ")
}
