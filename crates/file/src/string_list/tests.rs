use drakeflake_plan::{InputMode, ManagedOption, ManagedState};

use super::{LIST_TYPE, list_input, listed};

const OPTION: &str = "environment.persistence.\"/persist\".directories";

fn state_with(option: ManagedOption) -> ManagedState {
    let mut state = ManagedState::default();
    state.options.insert(OPTION.into(), option);
    state
}

fn entries(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|&text| text.to_owned()).collect()
}

#[test]
fn lists_survive_a_round_trip() {
    let saved = entries(&["/var/lib/foo", "/srv/my data"]);
    let state = state_with(ManagedOption::typed(LIST_TYPE, &list_input(&saved)));

    assert_eq!(listed(&state, OPTION), Some(saved));
    assert_eq!(listed(&state, "other.option"), Some(Vec::new()));
}

#[test]
fn other_forms_are_left_alone() {
    let typed = || ManagedOption::typed(LIST_TYPE, "/a");

    let expression = ManagedOption {
        mode: InputMode::Expression,
        ..typed()
    };

    let hosted = ManagedOption {
        host: Some("box".into()),
        ..typed()
    };

    for option in [expression, hosted] {
        assert_eq!(listed(&state_with(option), OPTION), None);
    }
}
