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
#[cfg(test)]
mod tests;
mod untranslated;
mod variant_label;

use crate::finding::{Finding, apply_allows};
use crate::source::{Lang, Source, parse};

pub fn lint_text(lang: Lang, text: &str) -> Vec<Finding> {
    let prepared = lang.prepare(text);
    let Some(tree) = parse(lang, &prepared) else {
        return Vec::new();
    };

    let source = Source::new(lang, &prepared, &tree);
    let mut findings = Vec::new();

    artifacts::check(&source, &mut findings);
    blank_before::check(&source, &mut findings);
    comment_lines::check(&source, &mut findings);
    comment_style::check(&source, &mut findings);
    commented_code::check(&source, &mut findings);
    doc_restating::check(&source, &mut findings);
    filler::check(&source, &mut findings);
    function_types::check(&source, &mut findings);
    header_tour::check(&source, &mut findings);
    restating::check(&source, &mut findings);
    semicolon::check(&source, &mut findings);
    separate::check(&source, &mut findings);
    untranslated::check(&source, &mut findings);
    variant_label::check(&source, &mut findings);

    apply_allows(&source, findings)
}

fn excerpt(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= 50 {
        return flat;
    }

    let short: String = flat.chars().take(47).collect();
    format!("{short}...")
}
