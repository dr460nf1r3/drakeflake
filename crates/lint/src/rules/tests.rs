mod artifacts;
mod blank_before;
mod comment_lines;
mod comment_style;
mod commented_code;
mod doc_restating;
mod filler;
mod function_types;
mod header_tour;
mod restating;
mod semicolon;
mod separate;
mod suppression;
mod untranslated;
mod variant_label;

use super::lint_text;
use crate::source::Lang;

fn rules(lang: Lang, text: &str) -> Vec<&'static str> {
    lint_text(lang, text)
        .into_iter()
        .map(|finding| finding.rule)
        .collect()
}

fn count(lang: Lang, rule: &str, text: &str) -> usize {
    rules(lang, text)
        .into_iter()
        .filter(|found| *found == rule)
        .count()
}
