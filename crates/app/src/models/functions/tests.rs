use std::sync::Arc;

use drakeflake_functions::{Function, FunctionIndex, IndexFile, Position, Source};
use serde_json::{Value, json};

use super::FunctionModel;
use super::rows::source_json;
use crate::hub::FunctionsMsg;

fn function(path: &str, signature: &str) -> Function {
    Function {
        path: path.to_owned(),
        doc: format!("Does {path}.\n\nMore text."),
        signature: signature.to_owned(),
        ..Function::default()
    }
}

fn position(file: &str) -> Position {
    Position {
        file: file.to_owned(),
        line: 10,
        column: 3,
    }
}

fn loaded() -> FunctionModel {
    let functions = vec![
        Function {
            builtin: true,
            ..function("builtins.map", "map :: (a -> b) -> [a] -> [b]")
        },
        Function {
            attr: Some(position("lib/strings.nix")),
            lambda: Some(position("/home/me/strings.nix")),
            ..function(
                "lib.strings.concatStrings",
                "concatStrings :: [String] -> String",
            )
        },
        function(
            "pkgs.writeText",
            "writeText :: String -> String -> Derivation",
        ),
    ];

    let index = FunctionIndex::from_file(IndexFile {
        version: 1,
        source: Source {
            rev: "abc".to_owned(),
            ..Source::default()
        },
        functions,
    });

    let mut model = FunctionModel::default();
    model
        .hub
        .mail()
        .functions
        .post(FunctionsMsg::Loaded(Arc::new(index)));
    model.drain();
    model
}

fn shown(model: &FunctionModel) -> Vec<String> {
    model.rows.iter().map(|row| row.path.clone()).collect()
}

#[test]
fn details_link_to_the_manual_and_the_source() {
    let model = loaded();
    let details = |path: &str| model.details(path.to_owned());

    assert_eq!(
        details("builtins.map")["manual"],
        "https://nix.dev/manual/nix/latest/language/builtins.html#builtins-map"
    );
    let concat = details("lib.strings.concatStrings");
    assert_eq!(
        concat["manual"],
        "https://nixos.org/manual/nixpkgs/unstable/#function-library-lib.strings.concatStrings"
    );
    assert_eq!(
        concat["attr"]["url"],
        "https://github.com/NixOS/nixpkgs/blob/abc/lib/strings.nix#L10"
    );

    assert_eq!(concat["lambda"]["url"], "");
    assert_eq!(
        (concat["namespace"].clone(), concat["summary"].clone()),
        (
            json!("lib.strings"),
            json!("Does lib.strings.concatStrings.")
        )
    );

    let write_text = details("pkgs.writeText");
    assert_eq!(
        (write_text["manual"].clone(), write_text["attr"].clone()),
        (json!(""), Value::Null)
    );
    assert_eq!(
        details("lib.nope"),
        json!({ "known": false, "path": "lib.nope" })
    );
}

#[test]
fn source_links_need_a_revision() {
    let unknown = source_json(&Source::default());
    assert_eq!(unknown["url"], "");
    let known = source_json(&Source {
        rev: "abc".to_owned(),
        date: 5,
        nix: "2.30".to_owned(),
    });

    assert_eq!(known["url"], "https://github.com/NixOS/nixpkgs/tree/abc");
    assert_eq!(
        (known["date"].clone(), known["nix"].clone()),
        (json!(5_i32), json!("2.30"))
    );
}

#[test]
fn the_list_stays_empty_until_searching_or_filtering() {
    let mut model = loaded();
    assert_eq!(
        (model.state(), model.size(), model.total()),
        ("ready".to_owned(), 3_i32, 0_i32)
    );
    assert!(!model.is_browsing());

    model.set_returns("derivation".to_owned());
    assert_eq!(shown(&model), ["pkgs.writeText"]);
    model.set_returns("no such kind".to_owned());
    assert_eq!(model.returns(), "");
    assert_eq!(model.total(), 0_i32);

    model.set_namespace("lib".to_owned());
    assert_eq!(shown(&model), ["lib.strings.concatStrings"]);
    model.search("map".to_owned());
    assert_eq!(model.total(), 0_i32, "the namespace still applies");

    model.reset();
    assert_eq!(
        (model.query(), model.namespace(), model.total()),
        (String::new(), String::new(), 0_i32)
    );
    assert_eq!(
        model.top(" ".to_owned(), 5),
        json!({ "total": 0_i32, "rows": [] })
    );
    assert_eq!(
        model.top("concat".to_owned(), 5)["rows"][0]["path"],
        "lib.strings.concatStrings"
    );
}

#[test]
fn a_missing_or_broken_index_is_reported() {
    let mut model = FunctionModel::default();
    assert_eq!(model.state(), "idle");

    model
        .hub
        .mail()
        .functions
        .post(FunctionsMsg::Missing("/share/functions.json".to_owned()));
    model.drain();
    assert_eq!(
        (model.state(), model.error()),
        ("missing".to_owned(), "/share/functions.json".to_owned())
    );

    model
        .hub
        .mail()
        .functions
        .post(FunctionsMsg::Failed("bad json".to_owned()));
    model.drain();
    assert_eq!(
        (model.state(), model.error()),
        ("error".to_owned(), "bad json".to_owned())
    );
}
