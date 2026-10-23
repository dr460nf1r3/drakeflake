use serde_json::json;

use super::*;

#[test]
fn type_check_results_map_to_outcomes() {
    let cases = [
        (
            json!({ "result": "ok", "type": "boolean" }),
            TypeCheck::Valid,
        ),
        (
            json!({ "result": "invalid", "type": "signed integer" }),
            TypeCheck::Invalid("signed integer".to_owned()),
        ),
        (
            json!({ "result": "error", "type": "string" }),
            TypeCheck::Failed("evaluating the value failed (expected string)".to_owned()),
        ),
        (
            json!({ "result": "invalid" }),
            TypeCheck::Invalid("unspecified".to_owned()),
        ),
        (json!({ "result": "unknown" }), TypeCheck::Unknown),
        (json!({ "result": true }), TypeCheck::Unknown),
        (Value::Null, TypeCheck::Unknown),
    ];

    for (outcome, expected) in cases {
        assert_eq!(type_check_of(&outcome), expected, "{outcome}");
    }
}
