use super::excerpt;
use crate::finding::Finding;
use crate::source::{Kind, Source, looks_like_code};
use crate::words::{WHY_WORDS, code_words, mostly_covered, prose_words};

const RULE: &str = "no-restating-comment";

pub fn check(source: &Source<'_>, findings: &mut Vec<Finding>) {
    for unit in source.units() {
        if !matches!(unit.kind(), Kind::Line | Kind::Block) {
            continue;
        }

        let text = unit.text().split_whitespace().collect::<Vec<_>>().join(" ");
        if WHY_WORDS.is_match(&text) || looks_like_code(source.lang, &text) {
            continue;
        }

        let code = source.adjacent_code(&unit);
        if !code.is_empty() && mostly_covered(&prose_words(&text), &code_words(&code)) {
            findings.push(
                Finding::new(
                    unit.start(),
                    RULE,
                    format!("Comment \"{}\" restates the code next to it.", excerpt(&text)),
                    "What-comments repeat the code, double the reading and go stale when the code changes.",
                )
                .fix("Delete it. If the code is not clear on its own, rename or extract until it is. Keep a comment only for a non-obvious reason."),
            );
        }
    }
}
