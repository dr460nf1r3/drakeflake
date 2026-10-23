use std::fs::File;
use std::io::BufRead as _;
use std::sync::Arc;

use super::super::spawn::open_master;
use super::super::*;
use crate::hub::Hub;

fn attached_terminal(hub: &Hub) -> std::io::BufReader<File> {
    use rustix::pty::{grantpt, ptsname, unlockpt};

    let master = open_master().unwrap();
    grantpt(&master).unwrap();
    unlockpt(&master).unwrap();
    let name = ptsname(&master, Vec::new()).unwrap();
    let slave = File::options()
        .read(true)
        .write(true)
        .open(Path::new(&*name.to_string_lossy()))
        .unwrap();

    hub.operations().pty().attach(Some(File::from(master)));
    std::io::BufReader::new(slave)
}

fn set_echo(terminal: &std::io::BufReader<File>, echo: bool) {
    use rustix::termios::{LocalModes, OptionalActions, tcgetattr, tcsetattr};

    let mut attributes = tcgetattr(terminal.get_ref()).unwrap();
    attributes.local_modes.set(LocalModes::ECHO, echo);
    tcsetattr(terminal.get_ref(), OptionalActions::Now, &attributes).unwrap();
}

fn read_line(terminal: &mut std::io::BufReader<File>) -> String {
    let mut line = String::new();
    terminal.read_line(&mut line).unwrap();
    line
}

fn prompts(hub: &Hub) -> Vec<Option<Prompt>> {
    hub.mail
        .console
        .take()
        .into_iter()
        .filter_map(|message| match message {
            ConsoleMsg::Prompt(prompt) => Some(prompt),
            ConsoleMsg::Lines(_)
            | ConsoleMsg::Screen {
                session: _,
                changes: _,
            } => None,
        })
        .collect()
}

#[test]
fn passwords_go_only_to_a_terminal_that_hides_them() {
    let hub = Hub::default();
    let mut terminal = attached_terminal(&hub);

    set_echo(&terminal, true);
    hub.answer_secret("not-sent");
    hub.answer(true);
    assert_eq!(
        read_line(&mut terminal),
        "y\n",
        "the echoed password was held back"
    );

    set_echo(&terminal, false);
    hub.answer_secret("hunter2");
    assert_eq!(read_line(&mut terminal), "hunter2\n");

    assert_eq!(
        prompts(&hub),
        vec![None; 3],
        "every answer closes the question"
    );
}

#[test]
fn answers_without_a_program_only_close_the_question() {
    let hub = Hub::default();

    hub.answer(false);
    hub.answer_secret("hunter2");

    assert_eq!(prompts(&hub), vec![None; 2]);
}

#[test]
fn quiet_questions_are_offered_once() {
    let hub = Arc::new(Hub::default());
    let run = Exclusive::detached(&hub);
    let quiet = Instant::now().checked_sub(PROMPT_QUIET * 2).unwrap();
    let mut prompted = String::new();
    let mut screen = Emulator::new(140, 40);

    screen.feed("building\r\nApply the config? [y/N] ");
    run.offer_question(&screen, Instant::now(), &mut prompted);
    assert_eq!(
        prompts(&hub),
        Vec::<Option<Prompt>>::new(),
        "output is still coming"
    );

    run.offer_question(&screen, quiet, &mut prompted);
    run.offer_question(&screen, quiet, &mut prompted);
    assert_eq!(
        prompts(&hub),
        vec![Some(Prompt {
            text: "Apply the config? [y/N]".to_owned(),
            password: false,
            core: "Apply the config?".to_owned(),
        })]
    );

    screen.feed("\r\nbuilt\r\n");
    run.offer_question(&screen, quiet, &mut prompted);
    assert_eq!(
        prompts(&hub),
        Vec::<Option<Prompt>>::new(),
        "no unfinished line"
    );
}

#[test]
fn password_prompts_in_echoing_output_are_not_offered() {
    let hub = Arc::new(Hub::default());
    let run = Exclusive::detached(&hub);
    let quiet = Instant::now().checked_sub(PROMPT_QUIET * 2).unwrap();
    let mut screen = Emulator::new(140, 40);
    screen.feed("[sudo] password for nico: ");

    run.offer_question(&screen, quiet, &mut String::new());
    assert_eq!(
        prompts(&hub),
        Vec::<Option<Prompt>>::new(),
        "no program reads it"
    );

    let terminal = attached_terminal(&hub);
    set_echo(&terminal, false);
    run.offer_question(&screen, quiet, &mut String::new());
    assert_eq!(
        prompts(&hub),
        vec![Some(Prompt {
            text: "[sudo] password for nico:".to_owned(),
            password: true,
            core: "[sudo] password for nico:".to_owned(),
        })]
    );
}

fn columns_and_rows(hub: &Hub) -> (u16, u16) {
    let size = hub.operations().pty().size();
    (size.ws_col, size.ws_row)
}

#[test]
fn the_terminal_size_follows_the_console_within_bounds() {
    let hub = Hub::default();

    assert_eq!(
        columns_and_rows(&hub),
        (140, 40),
        "before the Console has a size"
    );
    for ((columns, rows), expected) in [
        ((120, 30), (120, 30)),
        ((10, 2), (40, 10)),
        ((1000, 1000), (400, 200)),
        ((u16::MAX, u16::MAX), (400, 200)),
    ] {
        hub.set_terminal_size(columns, rows);
        assert_eq!(columns_and_rows(&hub), expected, "{columns} × {rows}");
    }

    let terminal = attached_terminal(&hub);
    hub.set_terminal_size(100, 25);
    let resized = rustix::termios::tcgetwinsize(terminal.get_ref()).unwrap();
    assert_eq!(
        (resized.ws_col, resized.ws_row),
        (100, 25),
        "the running program follows"
    );
}
