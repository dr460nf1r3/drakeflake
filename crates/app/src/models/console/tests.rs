use qtbridge::QListModel as _;
use serde_json::json;

use super::{ConsoleModel, MAX_LOG_LINES};
use crate::hub::term::Changes;
use crate::hub::{ConsoleMsg, ConsoleSession, Prompt};

fn post(model: &mut ConsoleModel, message: ConsoleMsg) {
    model.hub.mail().console.post(message);
    model.drain();
}

fn lines(model: &mut ConsoleModel, lines: &[&str]) {
    let lines = lines.iter().map(|&line| line.to_owned()).collect();
    post(model, ConsoleMsg::Lines(lines));
}

fn screen(
    model: &mut ConsoleModel,
    session: ConsoleSession,
    first: usize,
    rows: &[&str],
    total: usize,
) {
    let rows = rows.iter().map(|&row| row.to_owned()).collect();
    let changes = Changes { first, rows, total };
    post(model, ConsoleMsg::Screen { session, changes });
}

fn shown(model: &ConsoleModel) -> Vec<&str> {
    model.log.iter().map(|line| line.text.as_str()).collect()
}

#[test]
fn the_view_lists_the_newest_line_first() {
    let mut model = ConsoleModel::default();
    lines(&mut model, &["a", "b", "c"]);

    assert_eq!(model.get(0).map(|line| line.text.as_str()), Some("c"));
    assert_eq!(model.get(2).map(|line| line.text.as_str()), Some("a"));
    assert!(model.get(3).is_none());
    assert_eq!(model.log_text(), "a\nb\nc");

    model.clear_log();
    assert_eq!(model.line_count(), 0_i32);
}

#[test]
fn old_lines_are_dropped_beyond_the_limit() {
    let mut model = ConsoleModel::default();
    let many: Vec<String> = (0..MAX_LOG_LINES + 2).map(|n| n.to_string()).collect();
    post(&mut model, ConsoleMsg::Lines(many));

    assert_eq!(model.log.len(), MAX_LOG_LINES);
    assert_eq!(model.log.first().map(|line| line.text.as_str()), Some("2"));
}

#[test]
fn clearing_forgets_the_live_block() {
    let mut model = ConsoleModel::default();
    lines(&mut model, &["note"]);
    screen(&mut model, ConsoleSession(1), 0, &["a", "b"], 2);
    model.clear_log();

    screen(&mut model, ConsoleSession(1), 0, &["a", "b"], 2);
    screen(&mut model, ConsoleSession(1), 0, &["a", "c"], 2);
    assert_eq!(shown(&model), ["a", "c"]);
}

#[test]
fn dropped_lines_move_the_live_block_up() {
    let mut model = ConsoleModel::default();
    lines(&mut model, &["n1", "n2", "n3"]);
    screen(&mut model, ConsoleSession(1), 0, &["a", "b"], 2);
    let filler: Vec<String> = (0..MAX_LOG_LINES - 5).map(|n| n.to_string()).collect();
    post(&mut model, ConsoleMsg::Lines(filler));
    assert_eq!(model.log.len(), MAX_LOG_LINES);

    lines(&mut model, &["x", "y"]);
    assert_eq!(model.screen_block.map(|block| block.base), Some(1));

    screen(&mut model, ConsoleSession(1), 0, &["a", "c"], 2);
    assert_eq!(model.log.len(), MAX_LOG_LINES);
    assert_eq!(shown(&model).get(1..3), Some(["a", "c"].as_slice()));
}

#[test]
fn a_live_display_redraws_its_own_rows() {
    let mut model = ConsoleModel::default();
    lines(&mut model, &["note"]);
    screen(&mut model, ConsoleSession(1), 0, &["tree", "clock 1s"], 2);
    screen(&mut model, ConsoleSession(1), 1, &["clock 2s"], 2);
    assert_eq!(shown(&model), ["note", "tree", "clock 2s"]);

    lines(&mut model, &["after"]);
    screen(
        &mut model,
        ConsoleSession(1),
        0,
        &["build a", "tree", "clock 3s"],
        3,
    );
    assert_eq!(
        shown(&model),
        ["note", "build a", "tree", "clock 3s", "after"]
    );

    screen(&mut model, ConsoleSession(1), 0, &["tree"], 1);
    assert_eq!(shown(&model), ["note", "tree", "after"]);

    screen(&mut model, ConsoleSession(2), 0, &["tree"], 1);
    assert_eq!(shown(&model), ["note", "tree", "after", "tree"]);
}

#[test]
fn program_output_is_recoloured_with_the_theme() {
    let mut model = ConsoleModel::default();
    lines(&mut model, &["plain"]);
    screen(
        &mut model,
        ConsoleSession(1),
        0,
        &["\u{1b}[31mred\u{1b}[0m"],
        1,
    );
    let html = |console: &ConsoleModel| {
        console
            .log
            .iter()
            .map(|line| line.html.clone())
            .collect::<Vec<_>>()
    };

    assert_eq!(shown(&model), ["plain", "red"]);
    let before = html(&model);

    model.set_console_palette(json!([
        "#000", "#f00", "#0f0", "#ff0", "#00f", "#f0f", "#0ff", "#fff"
    ]));
    let after = html(&model);
    assert_eq!(
        after.first().map(String::as_str),
        Some(""),
        "plain lines have no markup"
    );
    assert_ne!(after.get(1), before.get(1));
    assert!(
        after.get(1).is_some_and(|line| line.contains("#f00")),
        "{after:?}"
    );

    model.set_console_palette(json!("red"));
    assert_eq!(html(&model), before);
}

#[test]
fn answering_closes_the_question() {
    let mut model = ConsoleModel::default();
    let question = Prompt {
        text: "Password for root:".to_owned(),
        password: true,
        core: "Password for root".to_owned(),
    };

    post(&mut model, ConsoleMsg::Prompt(Some(question)));
    assert!(model.prompt_password());
    assert_eq!(model.prompt_core(), "Password for root");

    model.answer(false);
    assert_eq!(
        (model.prompt(), model.prompt_core()),
        (String::new(), String::new())
    );
    post(&mut model, ConsoleMsg::Prompt(None));
    assert!(!model.prompt_password());
}
