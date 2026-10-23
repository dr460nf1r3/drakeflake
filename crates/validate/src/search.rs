use rnix::SyntaxNode;
use rnix::ast::{self, HasEntry as _};
use rowan::ast::AstNode as _;

use crate::source::{attr_name, site_at};

const SNIPPET_LENGTH: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitKind {
    Attribute,
    Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceHit {
    pub kind: HitKind,
    pub path: String,
    pub snippet: String,
    pub line: usize,
}

/// Below a matching path, children hit only with their own names, or every setting of a matching set would be a hit.
#[derive(Clone, Default)]
struct Scope {
    path: Vec<String>,
    hit_line: Option<usize>,
    path_matched: bool,
}

struct Search<'a> {
    text: &'a str,
    needle: &'a str,
    hits: Vec<SourceHit>,
}

impl Search<'_> {
    fn node(&mut self, node: &SyntaxNode, scope: &Scope) {
        if let Some(binding) = ast::AttrpathValue::cast(node.clone()) {
            self.binding(&binding, scope);
        } else if let Some(inherit) = ast::Inherit::cast(node.clone()) {
            self.inherit(&inherit, scope);
        } else if let Some(lambda) = ast::Lambda::cast(node.clone()) {
            if let Some(body) = lambda.body() {
                self.node(body.syntax(), scope);
            }
        } else if let Some(let_in) = ast::LetIn::cast(node.clone()) {
            let local = Scope {
                path: Vec::new(),
                ..scope.clone()
            };

            for entry in let_in.entries() {
                self.node(entry.syntax(), &local);
            }

            if let Some(body) = let_in.body() {
                self.node(body.syntax(), scope);
            }
        } else if is_leaf(node) {
            self.leaf(node, scope);
        } else {
            for child in node.children() {
                self.node(&child, scope);
            }
        }
    }

    fn binding(&mut self, binding: &ast::AttrpathValue, scope: &Scope) {
        let own: Vec<String> = binding
            .attrpath()
            .map(|attrpath| {
                attrpath
                    .attrs()
                    .map(|attr| {
                        attr_name(&attr).unwrap_or_else(|| attr.syntax().text().to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut path = scope.path.clone();
        path.extend(own.iter().cloned());
        let joined = path.join(".");
        let matched = if scope.path_matched {
            self.matches(&own.join("."))
        } else {
            self.matches(&joined)
        };

        let value = binding.value();
        let mut inner = Scope {
            path,
            hit_line: scope.hit_line,
            path_matched: scope.path_matched || matched,
        };

        if matched {
            let line = self.line_of(binding.syntax());
            self.hits.push(SourceHit {
                kind: HitKind::Attribute,
                path: joined,
                snippet: value
                    .as_ref()
                    .map(|value| snippet(&value.syntax().text().to_string()))
                    .unwrap_or_default(),
                line,
            });
            inner.hit_line = Some(line);
        }

        if let Some(value) = value {
            self.node(value.syntax(), &inner);
        }
    }

    fn inherit(&mut self, inherit: &ast::Inherit, scope: &Scope) {
        for attr in inherit.attrs() {
            let Some(name) = attr_name(&attr) else {
                continue;
            };

            let mut path = scope.path.clone();
            path.push(name.clone());
            let joined = path.join(".");
            let matched = if scope.path_matched {
                self.matches(&name)
            } else {
                self.matches(&joined)
            };

            if matched {
                self.hits.push(SourceHit {
                    kind: HitKind::Attribute,
                    path: joined,
                    snippet: snippet(&inherit.syntax().text().to_string()),
                    line: self.line_of(attr.syntax()),
                });
            }
        }
    }

    fn leaf(&mut self, node: &SyntaxNode, scope: &Scope) {
        let text = node.text().to_string();

        if !self.matches(&text) {
            return;
        }

        let line = self.line_of(node);

        if scope.hit_line == Some(line) {
            return;
        }

        self.hits.push(SourceHit {
            kind: HitKind::Value,
            path: scope.path.join("."),
            snippet: snippet(&text),
            line,
        });
    }

    fn matches(&self, text: &str) -> bool {
        text.to_lowercase().contains(self.needle)
    }

    fn line_of(&self, node: &SyntaxNode) -> usize {
        site_at(self.text, usize::from(node.text_range().start())).line
    }
}

#[must_use = "parsing the file has no other effect"]
pub fn search_source(text: &str, query: &str) -> Vec<SourceHit> {
    let needle = query.trim().to_lowercase();

    if needle.is_empty() {
        return Vec::new();
    }

    let root = rnix::Root::parse(text).tree();
    let mut search = Search {
        text,
        needle: &needle,
        hits: Vec::new(),
    };

    search.node(root.syntax(), &Scope::default());
    search.hits.sort_by_key(|hit| hit.line);
    search.hits
}

fn is_leaf(node: &SyntaxNode) -> bool {
    matches!(
        ast::Expr::cast(node.clone()),
        Some(
            ast::Expr::Str(_)
                | ast::Expr::PathAbs(_)
                | ast::Expr::PathRel(_)
                | ast::Expr::PathHome(_)
                | ast::Expr::PathSearch(_)
                | ast::Expr::Literal(_)
                | ast::Expr::Ident(_)
                | ast::Expr::Select(_)
        )
    )
}

fn snippet(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if flat.chars().count() <= SNIPPET_LENGTH {
        flat
    } else {
        let mut cut: String = flat.chars().take(SNIPPET_LENGTH - 1).collect();
        cut.push('…');
        cut
    }
}

#[cfg(test)]
mod tests;
