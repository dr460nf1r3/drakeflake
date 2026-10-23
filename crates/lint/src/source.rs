mod code;
mod comment;
#[cfg(test)]
mod tests;

use std::path::Path;

pub use code::looks_like_code;
pub use comment::{Comment, Kind, Unit};
use tree_sitter::{Language, Node, Parser, Point, Tree};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Qml,
    Js,
}

#[derive(Clone, Copy, Debug)]
pub struct Leaf<'t> {
    pub at: Point,
    pub text: &'t str,
    pub kind: &'t str,
    pub named: bool,
}

pub struct Source<'t> {
    pub lang: Lang,
    pub text: &'t str,
    pub comments: Vec<Comment<'t>>,
    pub leaves: Vec<Leaf<'t>>,
    pub root: Node<'t>,
    pub lines: Vec<&'t str>,
}

impl Lang {
    pub fn of(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Self::Rust),
            "qml" => Some(Self::Qml),
            "js" => Some(Self::Js),
            _ => None,
        }
    }

    /// QML JavaScript starts with `.pragma` and `.import` lines that plain
    /// JavaScript lacks. Blanking them keeps every offset in place.
    pub fn prepare(self, text: &str) -> String {
        if self != Self::Js {
            return text.to_owned();
        }

        text.split('\n')
            .map(|line| {
                if line.starts_with('.') {
                    " ".repeat(line.len())
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn grammar(self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Qml => tree_sitter_qmljs::LANGUAGE.into(),
            Self::Js => tree_sitter_javascript::LANGUAGE.into(),
        }
    }

    pub fn is_comment(self, kind: &str) -> bool {
        match self {
            Self::Rust => kind == "line_comment" || kind == "block_comment",
            Self::Qml | Self::Js => kind == "comment",
        }
    }
}

impl<'t> Source<'t> {
    pub fn new(lang: Lang, text: &'t str, tree: &'t Tree) -> Self {
        let mut source = Self {
            lang,
            text,
            comments: Vec::new(),
            leaves: Vec::new(),
            root: tree.root_node(),
            lines: text.split('\n').collect(),
        };

        source.collect(tree.root_node());
        source
    }

    pub fn node_text(&self, node: Node<'_>) -> &'t str {
        node.utf8_text(self.text.as_bytes()).unwrap_or_default()
    }

    pub fn units(&self) -> Vec<Unit<'_, 't>> {
        let mut units: Vec<Unit<'_, 't>> = Vec::new();

        for comment in self.comments.iter().filter(|comment| !comment.directive) {
            match units.last_mut() {
                Some(unit) if unit.last().continues_with(comment) => unit.comments.push(comment),
                _ => units.push(Unit {
                    comments: vec![comment],
                }),
            }
        }

        units
    }

    pub fn adjacent_code(&self, unit: &Unit<'_, 't>) -> Vec<Leaf<'t>> {
        let last = unit.last();

        if last.alone {
            let next_row = self
                .leaves
                .iter()
                .find(|leaf| leaf.at.row > last.end.row)
                .map(|leaf| leaf.at.row);

            return next_row.map_or_else(Vec::new, |row| self.leaves_on_row(row, usize::MAX));
        }

        let start = last.node.start_position();
        if unit.comments.len() == 1 && start.row == last.end.row {
            return self.leaves_on_row(start.row, start.column);
        }

        Vec::new()
    }

    pub fn in_function_body(&self, node: Node<'_>) -> bool {
        let body = match self.lang {
            Lang::Rust => "block",
            Lang::Qml | Lang::Js => "statement_block",
        };

        ancestors(node).any(|ancestor| ancestor.kind() == body)
    }

    pub fn documented_item(&self, unit: &Unit<'_, 't>) -> Option<Node<'t>> {
        let mut next = unit.last().node.next_named_sibling();

        while let Some(node) = next {
            if !self.lang.is_comment(node.kind()) && node.kind() != "attribute_item" {
                return Some(node);
            }

            next = node.next_named_sibling();
        }

        None
    }

    pub fn nodes_of_kind(&self, kind: &str) -> Vec<Node<'t>> {
        let mut found = Vec::new();
        let mut stack = vec![self.root];

        while let Some(node) = stack.pop() {
            if node.kind() == kind {
                found.push(node);
            }

            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
        }

        found.sort_by_key(Node::start_byte);
        found
    }

    fn leaves_on_row(&self, row: usize, before_column: usize) -> Vec<Leaf<'t>> {
        self.leaves
            .iter()
            .filter(|leaf| leaf.at.row == row && leaf.at.column < before_column)
            .copied()
            .collect()
    }

    fn line(&self, row: usize) -> &'t str {
        self.lines.get(row).copied().unwrap_or_default()
    }

    fn collect(&mut self, root: Node<'t>) {
        let mut stack = vec![root];

        while let Some(node) = stack.pop() {
            if self.lang.is_comment(node.kind()) {
                let comment = Comment::new(self, node);
                self.comments.push(comment);
            } else if node.child_count() == 0 {
                self.leaves.push(Leaf {
                    at: node.start_position(),
                    text: self.node_text(node),
                    kind: node.kind(),
                    named: node.is_named(),
                });
            } else {
                let mut cursor = node.walk();
                stack.extend(node.children(&mut cursor));
            }
        }

        self.comments
            .sort_by_key(|comment| comment.node.start_byte());
        self.leaves
            .sort_by_key(|leaf| (leaf.at.row, leaf.at.column));
    }
}

pub fn parse(lang: Lang, text: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(&lang.grammar()).ok()?;
    parser.parse(text, None)
}

fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    std::iter::successors(node.parent(), Node::parent)
}
