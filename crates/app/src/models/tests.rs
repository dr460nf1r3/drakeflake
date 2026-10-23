use super::*;

#[test]
fn one_line_shortens_on_char_boundaries() {
    assert_eq!(one_line("a\n  b", 10), "a b");
    assert_eq!(one_line("äöüäöü", 4), "äöü…");
    assert_eq!(one_line("", 3), "");
    assert_eq!(one_line("abc", 0), "…");
}

#[test]
fn counts_saturate() {
    assert_eq!(count_i32(usize::MAX), i32::MAX);
    assert_eq!(count_i32(3), 3_i32);
}
