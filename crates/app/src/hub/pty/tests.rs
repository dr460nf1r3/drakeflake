mod input;
mod quiet;
mod report;

use std::io::Read as _;
use std::sync::Arc;

use super::slot::{TERMINAL_COLUMNS, TERMINAL_ROWS, window_size};
use super::spawn::spawn_in_pty;
use super::*;
use crate::hub::Hub;
use crate::hub::argv::arg;

#[test]
fn password_prompts_are_recognised() {
    for prompt in [
        "[sudo] password for nico:",
        "[sudo] password for nico: ",
        "doas (nico@box) password:",
        "Password:",
        "Password for nico:",
    ] {
        assert!(looks_like_password_prompt(prompt), "{prompt}");
    }

    for line in [
        "? Apply the config? (y/N)",
        "updated password policy",
        "Password changed.",
    ] {
        assert!(!looks_like_password_prompt(line), "{line}");
    }
}

#[test]
fn ansi_is_stripped() {
    assert_eq!(strip_ansi("\u{1b}[1;32mok\u{1b}[0m done"), "ok done");
    assert_eq!(strip_ansi("\u{1b}]0;title\u{7}text"), "text");
    assert_eq!(strip_ansi("plain"), "plain");
    assert_eq!(strip_ansi("\u{1b}"), "");
}

#[test]
fn colours_become_spans() {
    assert_eq!(ansi_to_html("plain <b>", &[]), "plain&nbsp;&lt;b&gt;");
    assert_eq!(
        ansi_to_html("\u{1b}[1;32mok\u{1b}[0m done", &[]),
        "<font color=\"#27ae60\"><b>ok</b></font>&nbsp;done"
    );
    assert_eq!(ansi_to_html("\u{1b}[2Kline", &[]), "line");
    let theme: Vec<String> = (0_u8..8_u8).map(|index| format!("#00000{index}")).collect();
    assert_eq!(
        ansi_to_html("\u{1b}[31mred\u{1b}[0m", &theme),
        "<font color=\"#000001\">red</font>"
    );
}

#[test]
fn palette_entries_outside_hex_colours_fall_back() {
    let theme = vec!["\"><img src=\"https://example.org/x\">".to_owned()];

    assert_eq!(
        ansi_to_html("\u{1b}[31mred\u{1b}[0m", &theme),
        "<font color=\"#da4453\">red</font>"
    );
}

#[test]
fn extended_colours_and_resets_map_to_basic_tags() {
    let mut theme: Vec<String> = (0_i32..8_i32)
        .map(|index| format!("#00000{index}"))
        .collect();

    theme[2] = String::new();

    for (text, html) in [
        ("\u{1b}[38;5;9mx", "<font color=\"#000001\">x</font>"),
        ("\u{1b}[38;2;255;0;0mx", "<font color=\"#000007\">x</font>"),
        ("\u{1b}[38mx", "x"),
        ("\u{1b}[94mx", "<font color=\"#000004\">x</font>"),
        ("\u{1b}[32mx", "<font color=\"#27ae60\">x</font>"),
        ("\u{1b}[1mb\u{1b}[22mn", "<b>b</b>n"),
        (
            "\u{1b}[1;31mx\u{1b}[39my",
            "<font color=\"#000001\"><b>x</b></font><b>y</b>",
        ),
        ("a & b\u{7}\t", "a&nbsp;&amp;&nbsp;b"),
        ("\u{1b}[mx", "x"),
    ] {
        assert_eq!(ansi_to_html(text, &theme), html, "{text:?}");
    }
}

#[test]
fn osc_and_other_escapes_are_stripped() {
    for (text, plain) in [
        ("\u{1b}]8;;https://x\u{1b}\\link\u{1b}]8;;\u{1b}\\", "link"),
        ("\u{1b}(B\u{1b}[mplain", "plain"),
        ("\u{1b}7saved\u{1b}8", "saved"),
        ("\u{1b}]0;never ends", ""),
        ("a\u{1b}[", "a"),
    ] {
        assert_eq!(strip_ansi(text), plain, "{text:?}");
    }
}

#[test]
fn redraws_and_questions() {
    assert!(looks_like_question("Apply the config? [y/N]"));
    assert!(looks_like_question("> Do you want to continue?"));
    assert!(!looks_like_question("building foo"));
}

#[test]
fn question_cores_drop_markers_and_choices() {
    for (prompt, core) in [
        ("? Apply the config? (y/N)", "Apply the config?"),
        (
            "> Do you want to continue? [Y/n] ",
            "Do you want to continue?",
        ),
        ("✔ Proceed ( y / N ) now", "Proceed"),
        ("Continue? [y/N]", "Continue?"),
        ("×   Overwrite?", "Overwrite?"),
        ("Delete (yes/no)?", "Delete (yes/no)?"),
        ("[sudo] password for nico:", "[sudo] password for nico:"),
        ("(y/n)", ""),
        ("", ""),
    ] {
        assert_eq!(question_core(prompt), core, "{prompt}");
    }
}

#[test]
fn runs_in_a_terminal() {
    let (mut child, mut master) = spawn_in_pty(
        OsStr::new("sh"),
        &[arg("-c"), arg("test -t 0 && echo tty && stty size")],
        Path::new("/"),
        window_size(100, 30),
    )
    .unwrap();

    let mut output = String::new();

    #[expect(
        clippy::verbose_file_reads,
        reason = "a pseudo-terminal has no path to read from"
    )]
    master.read_to_string(&mut output).ok();
    assert!(child.wait().unwrap().success());
    assert!(output.contains("tty"));
    assert!(output.contains("30 100"), "{output}");
}

#[test]
fn questions_are_answered_through_the_hub() {
    let hub = Arc::new(Hub::default());
    let runner = Exclusive::detached(&hub);
    let worker = std::thread::spawn(move || {
        runner.stream_pty(
            OsStr::new("sh"),
            &[
                arg("-c"),
                arg("printf 'Continue? [y/N] '; read answer; echo got:$answer"),
            ],
            Path::new("/"),
        )
    });

    let mut asked = String::new();
    let mut output = Vec::new();
    let started = Instant::now();

    while !worker.is_finished() && started.elapsed() < Duration::from_secs(10) {
        for message in hub.mail.console.take() {
            match message {
                ConsoleMsg::Prompt(Some(question)) => {
                    asked = question.text;
                    hub.answer(true);
                }
                ConsoleMsg::Screen {
                    changes,
                    session: _,
                } => output.extend(changes.rows),
                ConsoleMsg::Prompt(None) | ConsoleMsg::Lines(_) => {}
            }
        }

        std::thread::sleep(Duration::from_millis(50));
    }

    assert!(worker.join().unwrap().is_success(), "the script failed");
    for message in hub.mail.console.take() {
        if let ConsoleMsg::Screen {
            changes,
            session: _,
        } = message
        {
            output.extend(changes.rows);
        }
    }

    assert_eq!(asked, "Continue? [y/N]");
    assert!(
        output.iter().any(|line| line.contains("got:y")),
        "{output:?}"
    );
}

#[test]
fn a_missing_working_directory_is_an_error_naming_it() {
    let error = spawn_in_pty(
        OsStr::new("true"),
        &[],
        Path::new("/nonexistent/flake-dir"),
        window_size(80, 24),
    )
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(
        error.to_string().contains("/nonexistent/flake-dir"),
        "{error}"
    );
}

#[test]
fn output_arriving_after_the_exit_is_still_collected() {
    let (sender, receiver) = mpsc::channel();
    sender
        .send(Output::Chunk("last error\n".to_owned()))
        .unwrap();
    sender.send(Output::Closed).unwrap();
    sender.send(Output::Chunk("never".to_owned())).unwrap();

    let mut screen = Emulator::new(140, 40);
    drain_after_exit(&mut screen, &receiver);
    screen.take_changes();
    assert_eq!(screen.rows(), ["last error".to_owned()]);
}

#[test]
fn a_gone_reader_does_not_end_the_wait_for_the_program() {
    let hub = Arc::new(Hub::default());
    let (sender, receiver) = mpsc::channel();
    drop(sender);

    let mut child = std::process::Command::new("sleep")
        .arg("0.3")
        .spawn()
        .unwrap();

    let started = Instant::now();
    let mut screen = Emulator::new(80, 24);
    let mut display = Display {
        shown: true,
        screen: &mut screen,
    };

    let status = Exclusive::detached(&hub)
        .pump_pty("sleep", &mut child, &receiver, &mut display)
        .unwrap();

    assert!(status.success());
    assert!(started.elapsed() >= Duration::from_millis(250));
}

#[test]
fn the_last_output_of_a_quick_program_is_shown() {
    let hub = Arc::new(Hub::default());
    let outcome = Exclusive::detached(&hub).stream_pty(
        OsStr::new("sh"),
        &[arg("-c"), arg("echo first; sleep 0.2; echo last; exit 3")],
        Path::new("/"),
    );

    let output: Vec<String> = hub
        .mail
        .console
        .take()
        .into_iter()
        .filter_map(|message| match message {
            ConsoleMsg::Screen {
                changes,
                session: _,
            } => Some(changes.rows),
            ConsoleMsg::Prompt(_) | ConsoleMsg::Lines(_) => None,
        })
        .flatten()
        .collect();

    assert_eq!(outcome, Outcome::Failed);
    assert!(output.iter().any(|row| row.contains("last")), "{output:?}");
}

#[test]
fn cancel_interrupts_a_running_program() {
    let hub = Arc::new(Hub::default());
    let run = Arc::new(Exclusive::detached(&hub));
    let runner = Arc::clone(&run);
    let worker = std::thread::spawn(move || {
        runner.stream_pty(OsStr::new("sleep"), &[arg("30")], Path::new("/"))
    });

    std::thread::sleep(Duration::from_millis(500));
    run.request_cancel();
    let started = Instant::now();

    while !worker.is_finished() && started.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(50));
    }

    assert!(worker.is_finished(), "Ctrl-C did not end the program");
    assert_eq!(worker.join().unwrap(), Outcome::Cancelled);
}

#[test]
#[ignore = "needs DRAKEFLAKE_NOM (nom binary) and DRAKEFLAKE_NOM_INSTALLABLE"]
fn live_nix_output_monitor() {
    let (Ok(nom), Ok(installable)) = (
        std::env::var("DRAKEFLAKE_NOM"),
        std::env::var("DRAKEFLAKE_NOM_INSTALLABLE"),
    ) else {
        return;
    };

    let (mut child, mut master) = spawn_in_pty(
        OsStr::new(&nom),
        &[
            arg("build"),
            arg(installable),
            arg("--rebuild"),
            arg("--no-link"),
        ],
        Path::new("/"),
        window_size(TERMINAL_COLUMNS, TERMINAL_ROWS),
    )
    .unwrap();

    let mut raw = Vec::new();

    #[expect(
        clippy::verbose_file_reads,
        reason = "a pseudo-terminal has no path to read from"
    )]
    master.read_to_end(&mut raw).ok();

    child.wait().ok();
    let text = String::from_utf8_lossy(&raw);

    let mut screen = Emulator::new(usize::from(TERMINAL_COLUMNS), usize::from(TERMINAL_ROWS));
    screen.feed(&text);
    let changes = screen.take_changes().unwrap();

    assert!(changes.total > 0);
}
