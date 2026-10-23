use super::excerpt;
use crate::finding::Finding;
use crate::source::{Kind, Lang, Source, looks_like_code};
use crate::words::WHY_WORDS;

const RULE: &str = "no-header-tour";

pub fn check(source: &Source<'_>, findings: &mut Vec<Finding>) {
    if source.lang != Lang::Qml {
        return;
    }

    for unit in source.units() {
        if !matches!(unit.kind(), Kind::Line | Kind::Block) {
            continue;
        }

        if unit.start().row > 30 || unit.height() < 2 {
            continue;
        }

        let text = unit.text().split_whitespace().collect::<Vec<_>>().join(" ");
        if WHY_WORDS.is_match(&text) || looks_like_code(source.lang, &text) {
            continue;
        }

        findings.push(
            Finding::new(
                unit.start(),
                RULE,
                format!("Comment \"{}\" tours the layout below.", excerpt(&text)),
                "QML structure is visible in the code. A header tour doubles the reading and goes stale on rename.",
            )
            .fix("Delete it. Keep a comment only for a non-obvious reason."),
        );
    }
}
