use crate::finding::Finding;
use crate::source::{Kind, Lang, Source};

const RULE: &str = "comment-style";

pub fn check(source: &Source<'_>, findings: &mut Vec<Finding>) {
    for comment in source
        .comments
        .iter()
        .filter(|comment| !comment.slashes && !comment.directive)
    {
        let fix = match (source.lang, comment.kind) {
            (_, Kind::Block) => "Write it as `//` lines.",
            (Lang::Rust, Kind::Doc) => "Write it as `///` lines.",
            (Lang::Rust, Kind::InnerDoc) => "Write it as `//!` lines.",
            _ => continue,
        };

        findings.push(
            Finding::new(
                comment.start(),
                RULE,
                "Block comment.".to_owned(),
                "One comment style per purpose, so the style says nothing on its own and tools treat every comment alike.",
            )
            .fix(fix),
        );
    }
}
