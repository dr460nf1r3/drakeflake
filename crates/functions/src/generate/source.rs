use std::collections::HashMap;

use rnix::{NodeOrToken, SyntaxKind, SyntaxNode, TextSize};

use crate::xml::RawPos;

const CODE_LINES: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub text: String,
    pub doc: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AttrSite {
    pub comment: Option<Comment>,
    pub copies: Option<String>,
    pub code: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LambdaSite {
    pub comment: Option<Comment>,
    pub code: String,
}

struct Parsed {
    text: String,
    root: SyntaxNode,
    line_starts: Vec<usize>,
}

#[derive(Default)]
pub struct Sources {
    files: HashMap<String, Option<Parsed>>,
}

impl Sources {
    pub fn attr(&mut self, position: &RawPos) -> Option<AttrSite> {
        let parsed = self.parsed(&position.file)?;
        let offset = parsed.offset(position)?;
        let token = parsed.root.token_at_offset(offset).right_biased()?;

        for node in token.parent_ancestors() {
            #[expect(
                clippy::wildcard_enum_match_arm,
                reason = "SyntaxKind is non-exhaustive with dozens of kinds and only these matter here"
            )]
            match node.kind() {
                SyntaxKind::NODE_ATTRPATH_VALUE if node.text_range().start() == offset => {
                    return Some(AttrSite {
                        comment: comment_before(&node),
                        copies: node.last_child().as_ref().and_then(plain_path),
                        code: parsed.code(&node),
                    });
                }
                SyntaxKind::NODE_INHERIT => return Some(inherit_site(&node, token.text())),
                SyntaxKind::NODE_INHERIT_FROM | SyntaxKind::NODE_ATTR_SET => return None,
                _ => {}
            }
        }

        None
    }

    pub fn lambda(&mut self, position: &RawPos) -> Option<LambdaSite> {
        let parsed = self.parsed(&position.file)?;
        let offset = parsed.offset(position)?;
        let token = parsed.root.token_at_offset(offset).right_biased()?;
        let lambda = token.parent_ancestors().find(|node| {
            node.kind() == SyntaxKind::NODE_LAMBDA && node.text_range().start() == offset
        })?;

        let mut plain = None;
        let mut current = lambda.clone();
        let mut definition = lambda;

        loop {
            if let Some(comment) = comment_before(&current) {
                if comment.doc {
                    return Some(LambdaSite {
                        comment: Some(comment),
                        code: parsed.code(&definition),
                    });
                }

                plain.get_or_insert(comment);
            }

            if current.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
                break;
            }

            let Some(parent) = current.parent() else {
                break;
            };

            #[expect(
                clippy::wildcard_enum_match_arm,
                reason = "SyntaxKind is non-exhaustive with dozens of kinds and only these matter here"
            )]
            match parent.kind() {
                SyntaxKind::NODE_PAREN
                | SyntaxKind::NODE_LET_IN
                | SyntaxKind::NODE_WITH
                | SyntaxKind::NODE_ASSERT
                | SyntaxKind::NODE_LAMBDA => current = parent,
                SyntaxKind::NODE_ATTRPATH_VALUE => {
                    definition = parent.clone();
                    current = parent;
                }
                _ => break,
            }
        }

        Some(LambdaSite {
            comment: plain,
            code: parsed.code(&definition),
        })
    }

    fn parsed(&mut self, file: &str) -> Option<&Parsed> {
        self.files
            .entry(file.to_owned())
            .or_insert_with(|| match std::fs::read_to_string(file) {
                Ok(text) => Some(Parsed::new(text)),
                Err(error) => {
                    log::warn!("cannot read {file}: {error}");
                    None
                }
            })
            .as_ref()
    }
}

impl Parsed {
    fn new(text: String) -> Self {
        let root = rnix::Root::parse(&text).syntax();
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(offset, _)| offset + 1))
            .collect();

        Self {
            text,
            root,
            line_starts,
        }
    }

    fn offset(&self, position: &RawPos) -> Option<TextSize> {
        let line = usize::try_from(position.line).ok()?.checked_sub(1)?;
        let column = usize::try_from(position.column).ok()?.checked_sub(1)?;
        let offset = self.line_starts.get(line)? + column;

        (offset < self.text.len())
            .then(|| TextSize::try_from(offset).ok())
            .flatten()
    }

    fn code(&self, node: &SyntaxNode) -> String {
        let start = usize::from(node.text_range().start());
        let line_start = self
            .text
            .get(..start)
            .and_then(|before| before.rfind('\n'))
            .map_or(0, |at| at + 1);

        let indent = " ".repeat(start.saturating_sub(line_start));
        let text = format!("{indent}{}", node.text());
        let lines: Vec<&str> = text.lines().collect();
        let common = lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);

        let mut shown: Vec<&str> = lines
            .iter()
            .take(CODE_LINES)
            .map(|line| line.get(common..).unwrap_or("").trim_end())
            .collect();

        let hidden = lines.len().saturating_sub(CODE_LINES);
        let more = format!("# … {hidden} more lines");
        if hidden > 0 {
            shown.push(&more);
        }

        shown.join("\n")
    }
}

fn inherit_site(node: &SyntaxNode, name: &str) -> AttrSite {
    let from = node
        .children()
        .find(|child| child.kind() == SyntaxKind::NODE_INHERIT_FROM)
        .and_then(|from| from.children().next())
        .and_then(|expr| plain_path(&expr));

    let names = node
        .children()
        .filter(|child| child.kind() != SyntaxKind::NODE_INHERIT_FROM)
        .count();

    AttrSite {
        comment: (names == 1).then(|| comment_before(node)).flatten(),
        copies: from.map(|from| format!("{from}.{name}")),
        code: String::new(),
    }
}

fn plain_path(node: &SyntaxNode) -> Option<String> {
    if !matches!(
        node.kind(),
        SyntaxKind::NODE_SELECT | SyntaxKind::NODE_IDENT
    ) {
        return None;
    }

    let has_default = node
        .descendants_with_tokens()
        .any(|element| element.kind() == SyntaxKind::TOKEN_OR);

    let text: String = node.text().to_string().split_whitespace().collect();
    let valid = !has_default
        && !text.is_empty()
        && text
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '\'' | '-'));

    valid.then_some(text)
}

fn comment_before(node: &SyntaxNode) -> Option<Comment> {
    let mut plain: Vec<String> = Vec::new();
    let mut blank = false;
    let mut element = node.prev_sibling_or_token();

    while let Some(current) = element {
        let NodeOrToken::Token(token) = &current else {
            break;
        };

        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "SyntaxKind is non-exhaustive with dozens of kinds and only these matter here"
        )]
        match token.kind() {
            SyntaxKind::TOKEN_WHITESPACE => {
                if token.text().matches('\n').count() > 1 {
                    blank = true;
                }
            }
            SyntaxKind::TOKEN_COMMENT => {
                let text = token.text();
                if text.starts_with("/**") && !text.starts_with("/**/") && plain.is_empty() {
                    return Some(Comment {
                        text: doc_text(text),
                        doc: true,
                    });
                }

                if blank {
                    break;
                }

                plain.push(plain_text(text));
            }
            _ => break,
        }

        element = current.prev_sibling_or_token();
    }

    plain.reverse();
    let text = plain.join("\n").trim().to_owned();
    (!text.is_empty()).then_some(Comment { text, doc: false })
}

pub fn doc_text(comment: &str) -> String {
    let inner = comment
        .strip_prefix("/**")
        .and_then(|rest| rest.strip_suffix("*/"))
        .unwrap_or(comment);

    dedent(inner)
}

fn plain_text(comment: &str) -> String {
    comment.strip_prefix('#').map_or_else(
        || {
            dedent(
                comment
                    .strip_prefix("/*")
                    .and_then(|rest| rest.strip_suffix("*/"))
                    .unwrap_or(comment),
            )
        },
        |line| line.strip_prefix(' ').unwrap_or(line).trim_end().to_owned(),
    )
}

fn dedent(text: &str) -> String {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("").trim();
    let rest: Vec<&str> = lines.collect();
    let common = rest
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    std::iter::once(first)
        .chain(
            rest.iter()
                .map(|line| line.get(common..).unwrap_or("").trim_end()),
        )
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}
