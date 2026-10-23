use std::fmt;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use tree_sitter::Point;

use crate::pattern::re;
use crate::source::Source;

static ALLOW: LazyLock<Regex> =
    LazyLock::new(|| re(r"^\s*drakeflake-lint:\s*allow\(([a-z\-,\s]+)\)\s*(.*)$"));

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub at: Point,
    pub rule: &'static str,
    pub problem: String,
    pub why: &'static str,
    pub fix: String,
}

pub struct Located<'a> {
    pub path: &'a Path,
    pub finding: &'a Finding,
}

struct Allow {
    row: usize,
    rules: Vec<String>,
}

impl Finding {
    pub const fn new(at: Point, rule: &'static str, problem: String, why: &'static str) -> Self {
        Self {
            at,
            rule,
            problem,
            why,
            fix: String::new(),
        }
    }

    pub fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = fix.into();
        self
    }
}

impl fmt::Display for Located<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Finding {
            at,
            rule,
            problem,
            why,
            fix,
        } = self.finding;

        writeln!(
            f,
            "{}:{}:{}: {rule}: {problem}",
            self.path.display(),
            at.row + 1,
            at.column + 1
        )?;
        writeln!(f, "    Why: {why}")?;
        write!(f, "    How to fix: {fix}")
    }
}

pub fn apply_allows(source: &Source<'_>, findings: Vec<Finding>) -> Vec<Finding> {
    let mut allows = Vec::new();
    let mut kept = Vec::new();

    for comment in source.comments.iter().filter(|comment| comment.directive) {
        let text = comment.text();
        let Some(captures) = ALLOW.captures(&text) else {
            continue;
        };

        let rules = captures.get(1).map_or("", |group| group.as_str());
        let reason = captures.get(2).map_or("", |group| group.as_str()).trim();

        if reason.is_empty() {
            kept.push(
                Finding::new(
                    comment.start(),
                    "allow-without-reason",
                    "`drakeflake-lint: allow(…)` without a reason.".to_owned(),
                    "An exception nobody can explain is soon a habit.",
                )
                .fix("Add the reason after the closing parenthesis, in one sentence."),
            );
        }

        allows.push(Allow {
            row: comment.end.row,
            rules: rules
                .split(',')
                .map(|rule| rule.trim().to_owned())
                .collect(),
        });
    }

    kept.extend(findings.into_iter().filter(|finding| {
        !allows.iter().any(|allow| {
            (allow.row == finding.at.row || allow.row + 1 == finding.at.row)
                && allow.rules.iter().any(|rule| rule == finding.rule)
        })
    }));
    kept.sort_by_key(|finding| (finding.at.row, finding.at.column));
    kept
}
