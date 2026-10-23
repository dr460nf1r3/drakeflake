use super::*;

const PORTS: &str = "list of 16 bit unsigned integer; between 0 and 65535 (both inclusive)";

#[test]
fn old_state_json_still_parses() {
    let old: ManagedOption = serde_json::from_str(r#"{"type":"boolean","input":"true"}"#).unwrap();
    assert_eq!(old, ManagedOption::typed("boolean", "true"));
    assert_eq!(
        serde_json::to_string(&old).unwrap(),
        r#"{"type":"boolean","input":"true"}"#,
        "defaults are not written"
    );

    let new = ManagedOption {
        priority: Some(Priority::Before),
        mode: InputMode::Expression,
        host: Some("box".into()),
        ..ManagedOption::typed(PORTS, "[ 1 ]")
    };

    let json = serde_json::to_string(&new).unwrap();
    assert!(json.contains(r#""priority":"before""#), "{json}");
    assert!(json.contains(r#""mode":"expression""#), "{json}");
    assert_eq!(serde_json::from_str::<ManagedOption>(&json).unwrap(), new);
}

#[test]
fn names_round_trip_through_display_and_from_str() {
    for name in [
        "default", "normal", "override", "force", "merge", "before", "after", "replace",
    ] {
        assert_eq!(name.parse::<Priority>().unwrap().to_string(), name);
    }

    for name in ["typed", "expression"] {
        assert_eq!(name.parse::<InputMode>().unwrap().to_string(), name);
    }

    "later".parse::<Priority>().unwrap_err();
    "raw".parse::<InputMode>().unwrap_err();
}
