use super::{Hunk, common_ends, hunks};

fn rows(text: &[&str]) -> Vec<String> {
    text.iter().map(|&row| row.to_owned()).collect()
}

const fn hunk(old_start: usize, removed: usize, new_start: usize, added: usize) -> Hunk {
    Hunk {
        old_start,
        removed,
        new_start,
        added,
    }
}

#[test]
fn a_log_line_above_the_tree_is_one_insert() {
    let old = ["building a", "┏ tree", "┗ 1 running"];
    let new = rows(&["building a", "building b", "┏ tree", "┗ 1 running"]);
    assert_eq!(hunks(&old, &new), vec![hunk(1, 0, 1, 1)]);
}

#[test]
fn a_log_line_and_the_clock_in_one_redraw() {
    let old = ["┏ tree", "┃ a", "┃ b", "┗ ⏱ 11s"];
    let new = rows(&["building b", "┏ tree", "┃ a", "┃ b", "┗ ⏱ 12s"]);
    assert_eq!(hunks(&old, &new), vec![hunk(0, 0, 0, 1), hunk(3, 1, 4, 1)]);
}

#[test]
fn a_changed_row_in_the_middle() {
    let old = ["one", "two", "three"];
    let new = rows(&["one", "2", "three"]);
    assert_eq!(hunks(&old, &new), vec![hunk(1, 1, 1, 1)]);
}

#[test]
fn rows_removed_and_added() {
    assert_eq!(
        hunks(&["a", "b", "c"], &rows(&["a", "c"])),
        vec![hunk(1, 1, 1, 0)]
    );
    assert_eq!(hunks(&[], &rows(&["a"])), vec![hunk(0, 0, 0, 1)]);
    assert_eq!(hunks(&["a"], &[]), vec![hunk(0, 1, 0, 0)]);
    assert_eq!(hunks(&["a"], &rows(&["a"])), Vec::<Hunk>::new());
}

#[test]
fn repeated_rows_are_not_counted_twice() {
    let old = ["x", "x"];
    let new = rows(&["x", "x", "x"]);
    assert_eq!(common_ends(&old, &new), (2, 0));
    assert_eq!(hunks(&old, &new), vec![hunk(2, 0, 2, 1)]);
}

#[test]
fn huge_redraws_replace_the_changed_middle_whole() {
    let old_rows: Vec<String> = (0_i32..202_i32).map(|row| format!("old {row}")).collect();
    let mut old: Vec<&str> = vec!["top"];
    old.extend(old_rows.iter().map(String::as_str));
    old.push("bottom");
    let mut new = vec!["top".to_owned()];
    new.extend((0_i32..201_i32).map(|row| format!("new {row}")));
    new.push("bottom".to_owned());

    assert_eq!(hunks(&old, &new), vec![hunk(1, 202, 1, 201)]);
}
