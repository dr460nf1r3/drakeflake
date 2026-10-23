pub fn looks_like_password_prompt(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    lower.contains("password") && (lower.ends_with(':') || lower.ends_with(": "))
}

pub fn looks_like_question(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.ends_with('?') || lower.contains("[y/n]") || lower.contains("(y/n)")
}

pub fn question_core(text: &str) -> String {
    let without_choices = choices_start(text)
        .and_then(|start| text.get(..start))
        .map_or(text, str::trim_end);

    let without_marker = without_choices
        .strip_prefix(['?', '>', '✔', '✓', '×'])
        .map_or(without_choices, str::trim_start);

    without_marker.trim().to_owned()
}

fn choices_start(text: &str) -> Option<usize> {
    text.char_indices()
        .filter(|(_, ch)| matches!(ch, '(' | '['))
        .map(|(start, _)| start)
        .find(|&start| text.get(start + 1..).is_some_and(is_choices))
}

fn is_choices(rest: &str) -> bool {
    let mut chars = rest.chars().filter(|ch| !ch.is_whitespace());
    matches!(
        (chars.next(), chars.next(), chars.next(), chars.next()),
        (Some('y' | 'Y'), Some('/'), Some('n' | 'N'), Some(')' | ']'))
    )
}
