use crate::finding::Finding;
use crate::source::{Kind, Source, looks_like_code};

const RULE: &str = "no-commented-out-code";

pub fn check(source: &Source<'_>, findings: &mut Vec<Finding>) {
    for unit in source.units() {
        let prose = matches!(unit.kind(), Kind::Line | Kind::Block);
        if prose && looks_like_code(source.lang, &unit.text()) {
            findings.push(
                Finding::new(
                    unit.start(),
                    RULE,
                    "Commented-out code.".to_owned(),
                    "Dead code in comments rots, confuses readers about what runs, and git already keeps the history.",
                )
                .fix("Delete it. If it documents a deliberate alternative, replace it with a one-line comment that says why."),
            );
        }
    }
}
