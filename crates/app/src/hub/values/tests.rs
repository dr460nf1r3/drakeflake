use super::*;
use crate::hub::Generation;

fn request(name: &str, input: &str) -> CheckRequest {
    CheckRequest {
        name: name.into(),
        input: input.to_owned(),
        nix_value: input.to_owned(),
    }
}

#[test]
fn only_the_newest_check_per_option_runs() {
    let pending = vec![
        (Generation(1), request("a", "old a")),
        (Generation(1), request("b", "b")),
        (Generation(2), request("a", "new a")),
        (Generation(2), request("c", "c")),
    ];

    let kept: Vec<(Generation, OptionName, String)> = newest_per_option(pending)
        .into_iter()
        .map(|(generation, request)| (generation, request.name, request.input))
        .collect();

    assert_eq!(
        kept,
        vec![
            (Generation(1), "b".into(), "b".to_owned()),
            (Generation(2), "a".into(), "new a".to_owned()),
            (Generation(2), "c".into(), "c".to_owned()),
        ]
    );
    assert_eq!(newest_per_option(Vec::new()).len(), 0);
}

#[test]
fn only_expressions_get_a_syntax_check() {
    assert_eq!(
        expression_syntax_error(&Context::system(), "typed:{ a = ;"),
        None
    );
    assert!(expression_syntax_error(&Context::system(), "expression:{ a = ;").is_some());
}

#[test]
fn a_check_without_a_system_fails_instead_of_staying_pending() {
    let hub = Arc::new(Hub::default());
    request_check(&hub, request("a.b", "1"));

    let mut posted = Vec::new();
    for _ in 0_u32..200_u32 {
        posted = hub.mail.changes.take();

        if !posted.is_empty() {
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let [
        ChangesMsg::Checked {
            name,
            input,
            result,
        },
    ] = posted.as_slice()
    else {
        panic!("expected one finished check, got {} messages", posted.len());
    };

    assert_eq!((name.as_str(), input.as_str()), ("a.b", "1"));
    assert_eq!(result, &Err("No system is selected yet".to_owned()));
}
