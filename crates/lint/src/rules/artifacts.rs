use std::sync::LazyLock;

use regex::Regex;

use super::excerpt;
use crate::finding::Finding;
use crate::pattern::re;
use crate::source::{Lang, Source};

const RULE: &str = "no-llm-artifacts";

static ARTIFACT: LazyLock<Regex> = LazyLock::new(|| {
    re(concat!(
        r"(?i)\.\.\.\s*(existing|rest of|previous|other|remaining)\b",
        r"|\b(remains?|stays?) (the )?(same|unchanged)\b",
        r"|\bTODO:?\s*implement\b",
        r"|\byour (code|logic|implementation) (goes )?here\b",
        r"|\bomitted for brevity\b",
        r"|\bsame as (above|before)\b",
        r"|\badd (more )?\w+ as needed\b",
        r"|\bcontinue from here\b",
        r"|\bsee implementation above\b",
        r"|\bexisting code\b",
    ))
});
static NOT_IMPLEMENTED: LazyLock<Regex> = LazyLock::new(|| re("(?i)not( yet)? implemented"));

pub fn check(source: &Source<'_>, findings: &mut Vec<Finding>) {
    for comment in source.comments.iter().filter(|comment| !comment.directive) {
        let text = comment.text();
        if ARTIFACT.is_match(&text) {
            findings.push(
                Finding::new(
                    comment.start(),
                    RULE,
                    format!("Placeholder comment \"{}\".", excerpt(&text)),
                    "Elision markers mean code was skipped or truncated. The file may be incomplete.",
                )
                .fix("Write the missing code or delete the comment."),
            );
        }
    }

    if source.lang == Lang::Rust {
        return;
    }

    for throw in source.nodes_of_kind("throw_statement") {
        let message = throw
            .named_child(0)
            .filter(|value| value.kind() == "new_expression")
            .and_then(|value| value.child_by_field_name("arguments"))
            .and_then(|arguments| arguments.named_child(0))
            .map(|argument| source.node_text(argument));

        if message.is_some_and(|text| NOT_IMPLEMENTED.is_match(text)) {
            findings.push(
                Finding::new(
                    throw.start_position(),
                    RULE,
                    "This function only throws \"not implemented\".".to_owned(),
                    "Stubs load and pass review but fail at runtime.",
                )
                .fix("Implement it, or remove it and its callers until it is needed."),
            );
        }
    }
}
