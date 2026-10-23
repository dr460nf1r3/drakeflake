use super::*;

fn emulator() -> Emulator {
    Emulator::new(40, 10)
}

fn shown(emulator: &mut Emulator) -> Vec<String> {
    emulator
        .take_changes()
        .map(|changes| changes.rows)
        .unwrap_or_default()
}

#[test]
fn a_fresh_terminal_has_no_changes() {
    assert_eq!(emulator().take_changes(), None);
}

#[test]
fn plain_lines_are_reported_once() {
    let mut terminal = emulator();
    terminal.feed("first\r\nlast");

    assert_eq!(
        terminal.take_changes(),
        Some(Changes {
            first: 0,
            rows: vec!["first".to_owned(), "last".to_owned()],
            total: 2,
        })
    );
    assert_eq!(terminal.take_changes(), None);
}

#[test]
fn trailing_blanks_are_not_rows() {
    let mut terminal = emulator();
    terminal.feed("padded   \r\n");

    assert_eq!(shown(&mut terminal), ["padded".to_owned()]);
}

#[test]
fn colours_and_bold_come_back_as_ansi() {
    let mut terminal = emulator();
    terminal.feed("\x1b[1;32mok\x1b[m and \x1b[38;5;9mbright");

    assert_eq!(
        shown(&mut terminal),
        ["\x1b[1;32mok\x1b[m and \x1b[38;5;9mbright".to_owned()]
    );
}

#[test]
fn true_colours_come_back_as_ansi() {
    let mut terminal = emulator();
    terminal.feed("\x1b[48;2;1;2;3mback");

    assert_eq!(shown(&mut terminal), ["\x1b[48;2;1;2;3mback".to_owned()]);
}

#[test]
fn inverse_comes_back_as_ansi() {
    let mut terminal = emulator();
    terminal.feed("\x1b[7;31mX");

    assert_eq!(shown(&mut terminal), ["\x1b[7;31mX".to_owned()]);
}

#[test]
fn a_cursor_move_rewrites_the_row() {
    let mut terminal = emulator();
    terminal.feed("old\r\n");
    shown(&mut terminal);

    terminal.feed("\x1b[1Anew\r\n");

    assert_eq!(shown(&mut terminal), ["new".to_owned()]);
}

#[test]
fn an_erased_row_is_empty() {
    let mut terminal = emulator();
    terminal.feed("abcdef\r\x1b[Kxy");

    assert_eq!(shown(&mut terminal), ["xy".to_owned()]);
}

#[test]
fn a_charset_selection_is_not_text() {
    let mut terminal = emulator();
    terminal.feed("\x1b(B\x1b[mplain");

    assert_eq!(shown(&mut terminal), ["plain".to_owned()]);
}

#[test]
fn a_synchronized_update_lands_whole() {
    let mut terminal = emulator();
    terminal.feed("\x1b[?2026h\x1b[2Ktree\r\nleaves\x1b[?2026l");

    assert_eq!(
        shown(&mut terminal),
        ["tree".to_owned(), "leaves".to_owned()]
    );
}

#[test]
fn wide_characters_take_one_cell() {
    let mut terminal = emulator();
    terminal.feed("あ\r\n");

    assert_eq!(shown(&mut terminal), ["あ".to_owned()]);
}

#[test]
fn only_touched_rows_are_reported() {
    let mut terminal = emulator();
    terminal.feed("a\r\nb\r\nc\r\n");
    shown(&mut terminal);

    terminal.feed("\x1b[2;5Hx");

    assert_eq!(
        terminal.take_changes(),
        Some(Changes {
            first: 1,
            rows: vec!["b   x".to_owned(), "c".to_owned()],
            total: 3,
        })
    );
}

#[test]
fn the_cursor_line_is_the_unfinished_one() {
    let mut terminal = emulator();
    terminal.feed("Apply? [y/N] ");

    assert_eq!(terminal.unfinished_line(), Some("Apply? [y/N]".to_owned()));

    terminal.feed("\r\nbuilt\r\n");

    assert_eq!(terminal.unfinished_line(), None);
}

#[test]
fn resizing_keeps_the_grid_usable() {
    let mut terminal = emulator();
    terminal.feed("kept\r\n");
    terminal.resize(20, 5);

    let changes = terminal.take_changes().unwrap();
    assert_eq!(changes.total, 1);
    assert!(changes.rows.iter().any(|row| row.contains("kept")));
}
