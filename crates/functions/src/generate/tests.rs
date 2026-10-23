use std::path::Path;

use crate::source::{Sources, doc_text};
use crate::xml::{Collected, RawPos, collected};
use crate::{assemble, builtins};

const STRINGS: &str = r#"{ lib }:
{
  /**
    Concatenate a list of strings.

    # Type

    ```
    concatStrings :: [String] -> String
    ```
  */
  concatStrings = builtins.concatStringsSep "";

  /**
    Join with a separator.
  */
  concatSep = sep: list: builtins.concatStringsSep sep list;

  # Only a plain comment.
  plain = x: x;

  wrapped = (
    /** Documented inside the parentheses. */
    a: a
  );
}
"#;

const DEFAULT: &str = "{
  inherit (strings) concatStrings concatSep;
  inherit (builtins) add;
}
";

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}

fn position(file: &str, line: u32, column: u32) -> RawPos {
    RawPos {
        file: file.to_owned(),
        line,
        column,
    }
}

fn entry(path: &str, attr: RawPos, lambda: Option<RawPos>) -> Collected {
    Collected {
        path: path.to_owned(),
        attr: Some(attr),
        lambda,
        args: Vec::new(),
    }
}

#[test]
fn xml_entries_carry_positions_and_arguments() {
    let text = r#"<?xml version='1.0' encoding='utf-8'?>
<expr>
  <list>
    <attrs>
      <attr name="attr"><attrs>
        <attr name="column"><int value="9" /></attr>
        <attr name="file"><string value="/src/lib/default.nix" /></attr>
        <attr name="line"><int value="2" /></attr>
      </attrs></attr>
      <attr name="lambda">
        <function column="3" line="7" path="/src/lib/strings.nix">
          <attrspat><attr name="name" /><attr name="value" /></attrspat>
        </function>
      </attr>
      <attr name="path"><string value="lib.&quot;odd&amp;name&quot;" /></attr>
    </attrs>
    <attrs>
      <attr name="attr"><null /></attr>
      <attr name="lambda"><unevaluated /></attr>
      <attr name="path"><string value="lib.add" /></attr>
    </attrs>
  </list>
</expr>
"#;

    let found = collected(text).unwrap();

    assert_eq!(
        found,
        vec![
            Collected {
                path: "lib.\"odd&name\"".to_owned(),
                attr: Some(position("/src/lib/default.nix", 2, 9)),
                lambda: Some(position("/src/lib/strings.nix", 7, 3)),
                args: vec!["name".to_owned(), "value".to_owned()],
            },
            Collected {
                path: "lib.add".to_owned(),
                attr: None,
                lambda: None,
                args: Vec::new(),
            },
        ]
    );
    collected("<expr><list>").unwrap_err();
}

#[test]
fn xml_numeric_and_unknown_entities_and_plain_arguments() {
    let text = r#"<expr><list><attrs>
      <attr name="lambda"><function column="1" line="1" path="/f.nix"><varpat name="x" /></function></attr>
      <attr name="path"><string value="a&#x42;&#67; &bogus; &#xZZ; &amp" /></attr>
    </attrs></list></expr>"#;

    let found = collected(text).unwrap();

    assert_eq!(
        found
            .first()
            .map(|entry| (entry.path.as_str(), entry.args.clone())),
        Some(("aBC &bogus; &#xZZ; &amp", vec!["x".to_owned()]))
    );
}

#[test]
fn xml_without_a_list_or_a_path_is_refused() {
    for (text, error) in [
        ("<expr><attrs /></expr>", "the positions are not a list"),
        (
            "<expr><list><attrs><attr name=\"attr\"><null /></attr></attrs></list></expr>",
            "an entry has no path",
        ),
    ] {
        assert_eq!(collected(text).unwrap_err().to_string(), error);
    }
}

#[test]
fn doc_comments_lose_their_indentation() {
    assert_eq!(
        doc_text("/**\n    First.\n\n    ```\n      code\n    ```\n  */"),
        "First.\n\n```\n  code\n```"
    );
    assert_eq!(doc_text("/** One line. */"), "One line.");
}

#[test]
fn comments_are_found_at_attributes_and_lambdas() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "strings.nix", STRINGS);
    let mut sources = Sources::default();

    let concat = sources.attr(&position(&file, 12, 3));
    let comment = concat.as_ref().and_then(|site| site.comment.clone());
    assert!(
        comment
            .as_ref()
            .is_some_and(|comment| comment.doc && comment.text.starts_with("Concatenate"))
    );
    assert_eq!(concat.and_then(|site| site.copies), None);

    let separated = sources.lambda(&position(&file, 17, 15));
    let separated_doc = separated.as_ref().and_then(|site| site.comment.clone());
    assert_eq!(
        separated_doc.map(|found| found.text),
        Some("Join with a separator.".to_owned())
    );
    assert!(separated.is_some_and(|site| site.code.starts_with("concatSep = sep: list:")));

    let plain = sources
        .lambda(&position(&file, 20, 11))
        .and_then(|site| site.comment);

    assert_eq!(
        plain.map(|found| (found.doc, found.text)),
        Some((false, "Only a plain comment.".to_owned()))
    );

    let wrapped = sources
        .lambda(&position(&file, 24, 5))
        .and_then(|site| site.comment);

    assert!(wrapped.is_some_and(|found| found.doc));

    assert!(sources.lambda(&position(&file, 99, 1)).is_none());
    assert!(
        sources
            .attr(&position("/does/not/exist.nix", 1, 1))
            .is_none()
    );
}

#[test]
fn copies_are_plain_paths_without_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let file = write(
        dir.path(),
        "copies.nix",
        "{\n  add = builtins.add;\n  fallback = x.b or c;\n}\n",
    );

    let mut sources = Sources::default();

    let add = sources.attr(&position(&file, 2, 3));
    assert_eq!(
        add.and_then(|site| site.copies),
        Some("builtins.add".to_owned())
    );
    let fallback = sources.attr(&position(&file, 3, 3));
    assert_eq!(fallback.and_then(|site| site.copies), None);
}

#[test]
fn aliases_share_the_documentation() {
    let dir = tempfile::tempdir().unwrap();
    let strings = write(dir.path(), "strings.nix", STRINGS);
    let default = write(dir.path(), "default.nix", DEFAULT);
    let root = dir.path().to_string_lossy().into_owned();
    let builtin = builtins::parse(r#"{"add": {"args": ["e1", "e2"], "doc": "Return the sum."}}"#)
        .unwrap_or_default();

    let functions = assemble::assemble(
        vec![
            entry("lib.strings.concatStrings", position(&strings, 12, 3), None),
            entry("lib.concatStrings", position(&default, 2, 21), None),
            entry(
                "lib.strings.concatSep",
                position(&strings, 17, 3),
                Some(position(&strings, 17, 15)),
            ),
            entry(
                "lib.concatSep",
                position(&default, 2, 35),
                Some(position(&strings, 17, 15)),
            ),
            entry("lib.add", position(&default, 3, 22), None),
        ],
        builtin,
        &root,
    );

    let find = |path: &str| functions.iter().find(|item| item.path == path);

    let alias = find("lib.concatStrings");
    assert_eq!(
        alias.map(|item| item.doc_from.as_str()),
        Some("lib.strings.concatStrings")
    );
    assert_eq!(
        alias.map(|item| item.signature.as_str()),
        Some("concatStrings :: [String] -> String")
    );
    assert_eq!(
        alias
            .and_then(|item| item.attr.as_ref())
            .map(|position| position.file.as_str()),
        Some("default.nix")
    );

    let sep = find("lib.concatSep");
    assert_eq!(
        sep.map(|item| item.aliases.clone()),
        Some(vec!["lib.strings.concatSep".to_owned()])
    );
    assert_eq!(
        sep.map(|item| item.doc.as_str()),
        Some("Join with a separator.")
    );

    let add = find("lib.add");
    assert_eq!(add.map(|item| item.doc_from.as_str()), Some("builtins.add"));
    assert_eq!(
        find("builtins.add").map(|item| item.aliases.clone()),
        Some(vec!["lib.add".to_owned()])
    );
}

#[test]
fn builtins_skip_constants_and_link_the_manual() {
    let dump = r#"{
        "map": {"args": ["f", "list"], "doc": "See [operators](@docroot@/language/operators.md#update)."},
        "true": {"doc": "Primitive value."},
        "fetchClosure": {"args": ["args"], "doc": "", "experimental-feature": "fetch-closure"}
    }"#;

    let found = builtins::parse(dump).unwrap();

    assert_eq!(found.len(), 2);
    let map = found
        .iter()
        .find(|function| function.path == "builtins.map");

    assert_eq!(
        map.map(|function| function.doc.as_str()),
        Some("See [operators](https://nix.dev/manual/nix/latest/language/operators.html#update).")
    );
    assert!(
        found
            .iter()
            .any(|function| function.experimental == "fetch-closure" && function.builtin)
    );
}

#[test]
fn arguments_need_values_and_the_required_paths() {
    let parse = |args: &[&str]| crate::parse_args(args.iter().map(|arg| (*arg).to_owned()));

    let parsed = parse(&[
        "--positions",
        "p.xml",
        "--output",
        "out.json",
        "--date",
        "1700000000",
    ])
    .unwrap();

    assert_eq!(
        (parsed.positions, parsed.output, parsed.date),
        ("p.xml".into(), "out.json".into(), 1_700_000_000)
    );

    for (args, error) in [
        (
            &["--positions", "p.xml"][..],
            "--positions and --output are required",
        ),
        (
            &["--positions", "p.xml", "--output"],
            "--output needs a value",
        ),
        (&["--date", "yesterday"], "--date is not a number"),
        (&["--verbose", "yes"], "unknown option --verbose"),
    ] {
        let message = format!("{:#}", parse(args).unwrap_err());
        assert!(message.starts_with(error), "{args:?}: {message}");
    }
}
