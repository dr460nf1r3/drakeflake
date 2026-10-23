use crate::{
    Block, FORMAT_VERSION, Filter, Function, FunctionIndex, IndexFile, Position, Shape, Source,
    TypeKind, UnknownTypeKind, blocks, signature_from_doc,
};

const CONCAT_DOC: &str = "Concatenate a list of strings.

# Type

```
concatStrings :: [String] -> String
```

# Examples
:::{.example}
## `lib.strings.concatStrings` usage example

```nix
concatStrings [\"foo\" \"bar\"]
=> \"foobar\"
```

:::";

fn function(path: &str, doc: &str) -> Function {
    Function {
        path: path.to_owned(),
        doc: doc.to_owned(),
        signature: signature_from_doc(doc),
        ..Function::default()
    }
}

fn index() -> FunctionIndex {
    FunctionIndex::from_file(IndexFile {
        version: FORMAT_VERSION,
        source: Source::default(),
        functions: vec![
            function("lib.strings.concatStrings", CONCAT_DOC),
            function(
                "lib.attrsets.mapAttrs",
                "Map attributes.\n\n# Type\n\n```\nmapAttrs :: (String -> Any -> Any) -> AttrSet -> AttrSet\n```",
            ),
            function("lib.concatStrings", CONCAT_DOC),
            function("builtins.map", "Apply the function *f* to each element."),
        ],
    })
}

#[test]
fn signature_comes_from_the_type_section() {
    assert_eq!(
        signature_from_doc(CONCAT_DOC),
        "concatStrings :: [String] -> String"
    );
    assert_eq!(signature_from_doc("No type here.\n\n```nix\nx\n```"), "");
    assert_eq!(signature_from_doc("# Type\n\n# Examples\n```\nx\n```"), "");
}

#[test]
fn shapes_compare_the_first_argument_and_the_result() {
    let concat = Shape::of("concatStrings :: [String] -> String");
    assert_eq!(concat.takes, Some(TypeKind::List));
    assert_eq!(concat.returns, Some(TypeKind::String));

    let map = Shape::of("mapAttrs :: (String -> Any -> Any) -> AttrSet -> AttrSet");
    assert_eq!(map.takes, Some(TypeKind::Function));
    assert_eq!(map.returns, Some(TypeKind::Attrset));

    let generic = Shape::of("id :: a -> a");
    assert_eq!(generic.takes, Some(TypeKind::Any));

    assert_eq!(Shape::of("version :: String"), Shape::default());
    let unnamed = Shape::of("{ a :: Int } -> String");
    assert_eq!(unnamed.takes, Some(TypeKind::Attrset));
    assert_eq!(unnamed.returns, Some(TypeKind::String));
    let named = Shape::of("f :: { a :: Int } -> String");
    assert_eq!(named.takes, Some(TypeKind::Attrset));
    assert_eq!("attrset".parse(), Ok(TypeKind::Attrset));
    assert_eq!(TypeKind::Attrset.to_string(), "attrset");
    assert_eq!("nope".parse::<TypeKind>(), Err(UnknownTypeKind));
}

#[test]
fn blocks_split_headings_text_and_code() {
    let found = blocks(CONCAT_DOC, "concatStrings :: [String] -> String");

    assert_eq!(
        found,
        vec![
            Block::Text {
                text: "Concatenate a list of strings.".to_owned()
            },
            Block::Heading {
                level: 1,
                text: "Examples".to_owned()
            },
            Block::Heading {
                level: 2,
                text: "`lib.strings.concatStrings` usage example".to_owned()
            },
            Block::Code {
                language: "nix".to_owned(),
                text: "concatStrings [\"foo\" \"bar\"]\n=> \"foobar\"".to_owned()
            },
        ]
    );
}

#[test]
fn blocks_keep_the_type_section_without_a_signature() {
    let found = blocks(CONCAT_DOC, "");
    assert!(found.contains(&Block::Heading {
        level: 1,
        text: "Type".to_owned()
    }));
}

#[test]
fn blocks_label_notes_and_empty_function_links() {
    let found = blocks(
        "::: {.note}\nSee [](#function-library-lib.lists.map).\n:::",
        "",
    );

    assert_eq!(
        found,
        vec![Block::Text {
            text: "**Note:**\nSee [`lib.lists.map`](#function-library-lib.lists.map).".to_owned()
        }]
    );
}

#[test]
fn search_ranks_names_and_applies_filters() {
    let index = index();
    let paths = |hits: Vec<usize>| -> Vec<String> {
        hits.into_iter()
            .filter_map(|hit| index.get(hit).map(|function| function.path.clone()))
            .collect()
    };

    assert_eq!(
        paths(index.search("concatstrings", &Filter::default())),
        vec!["lib.concatStrings", "lib.strings.concatStrings"]
    );

    let strings = Filter {
        namespace: "lib.strings".to_owned(),
        ..Filter::default()
    };

    assert_eq!(
        paths(index.search("", &strings)),
        vec!["lib.strings.concatStrings"]
    );

    let takes_function = Filter {
        takes: Some(TypeKind::Function),
        ..Filter::default()
    };

    assert_eq!(
        paths(index.search("", &takes_function)),
        vec!["lib.attrsets.mapAttrs"]
    );

    assert_eq!(
        paths(index.search("element", &Filter::default())),
        vec!["builtins.map"]
    );
    assert_eq!(
        index.search("nothing-like-this", &Filter::default()),
        Vec::<usize>::new()
    );
}

#[test]
fn namespaces_count_nested_functions() {
    let namespaces = index().namespaces();
    let lib = namespaces.iter().find(|namespace| namespace.name == "lib");

    assert_eq!(
        lib.map(|namespace| (namespace.count, namespace.direct)),
        Some((3, 1))
    );
    assert!(
        namespaces
            .iter()
            .any(|namespace| namespace.name == "lib.strings")
    );
    assert!(!namespaces.iter().any(|namespace| namespace.name.is_empty()));
}

#[test]
fn a_newer_format_is_named_even_when_its_fields_changed() {
    let text = r#"{"version": 999, "functions": "now a map"}"#;

    assert_eq!(
        FunctionIndex::from_json_str(text).unwrap_err().to_string(),
        format!("the function index has format 999, this program reads format {FORMAT_VERSION}")
    );
}

#[test]
fn other_format_versions_are_refused() {
    let text = r#"{"version": 999, "source": {}, "functions": []}"#;
    FunctionIndex::from_json_str(text).unwrap_err();

    let current = format!(
        r#"{{"version": {FORMAT_VERSION}, "source": {{}}, "functions": [{{"path": "lib.id"}}]}}"#
    );

    assert_eq!(
        FunctionIndex::from_json_str(&current)
            .map(|index| index.len())
            .ok(),
        Some(1)
    );
}

#[test]
fn lookups_stay_aligned_after_sorting_and_deduplicating() {
    let index = FunctionIndex::from_file(IndexFile {
        version: FORMAT_VERSION,
        source: Source::default(),
        functions: vec![
            function(
                "lib.zip",
                "Zip.\n\n# Type\n\n```\nzip :: [a] -> [b] -> [c]\n```",
            ),
            function("lib.id", "Identity."),
            function("lib.zip", "A later duplicate."),
            function("builtins.add", "Add two numbers."),
        ],
    });

    assert_eq!(index.len(), 3);
    let zip = index.position("lib.zip").unwrap();
    assert_eq!(index.summary(zip), "Zip.");
    assert_eq!(index.shape(zip).takes, Some(TypeKind::List));
    assert_eq!(
        index
            .by_path("builtins.add")
            .map(|found| found.doc.as_str()),
        Some("Add two numbers.")
    );
    assert_eq!(index.position("lib.missing"), None);
    assert_eq!(index.summary(99), "");
}

#[test]
fn summaries_are_the_first_paragraph_on_one_line() {
    let index = FunctionIndex::from_file(IndexFile {
        version: FORMAT_VERSION,
        source: Source::default(),
        functions: vec![
            function("a.text", "\n\n  First line\n  second line.\n\nMore."),
            function("b.fence", "Before the code\n```nix\nx\n```"),
            function("c.note", "Before the note\n::: {.note}\nx\n:::"),
            function("d.heading", "# Inputs\nx"),
        ],
    });

    let summaries: Vec<&str> = (0..index.len()).map(|at| index.summary(at)).collect();
    assert_eq!(
        summaries,
        [
            "First line second line.",
            "Before the code",
            "Before the note",
            ""
        ]
    );
}

#[test]
fn a_namespace_filter_matches_whole_names_only() {
    let index = index();
    let paths = |filter: &Filter| -> Vec<String> {
        index
            .search("", filter)
            .into_iter()
            .filter_map(|hit| index.get(hit).map(|function| function.path.clone()))
            .collect()
    };

    let within = |namespace: &str| Filter {
        namespace: namespace.to_owned(),
        ..Filter::default()
    };

    assert_eq!(paths(&within("lib.str")), Vec::<String>::new());
    assert_eq!(
        paths(&within("lib")),
        [
            "lib.attrsets.mapAttrs",
            "lib.concatStrings",
            "lib.strings.concatStrings"
        ]
    );
    let returns_attrs = Filter {
        returns: Some(TypeKind::Attrset),
        ..Filter::default()
    };

    assert_eq!(paths(&returns_attrs), ["lib.attrsets.mapAttrs"]);
    assert!(Filter::default().is_empty() && !within("lib").is_empty());
}

#[test]
fn namespaces_with_enough_functions_get_a_card() {
    let functions = ["a", "b", "c", "d"]
        .iter()
        .map(|name| function(&format!("lib.lists.{name}"), ""))
        .chain(
            ["x", "y", "z"]
                .iter()
                .map(|name| function(&format!("lib.{name}"), "")),
        )
        .collect();

    let namespaces = FunctionIndex::from_file(IndexFile {
        version: FORMAT_VERSION,
        source: Source::default(),
        functions,
    })
    .namespaces();

    let cards: Vec<(&str, usize, usize, bool)> = namespaces
        .iter()
        .map(|space| (space.name.as_str(), space.count, space.direct, space.card))
        .collect();

    assert_eq!(cards, [("lib", 7, 3, false), ("lib.lists", 4, 4, true)]);
}

#[test]
fn unreadable_or_broken_index_files_are_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("functions.json");

    let missing = FunctionIndex::load(&path).unwrap_err();
    assert!(
        format!("{missing:#}").starts_with("cannot read"),
        "{missing:#}"
    );

    std::fs::write(&path, "{\"version\": 1").unwrap();
    let broken = FunctionIndex::load(&path).unwrap_err();
    assert!(format!("{broken:#}").starts_with("the function index is not valid JSON"));
}

#[test]
fn positions_link_into_nixpkgs_only() {
    let inside = Position {
        file: "lib/strings.nix".to_owned(),
        line: 12,
        column: 3,
    };

    assert_eq!(
        inside.url("abc").as_deref(),
        Some("https://github.com/NixOS/nixpkgs/blob/abc/lib/strings.nix#L12")
    );
    assert_eq!(inside.url(""), None);

    let outside = Position {
        file: "/etc/nixos/x.nix".to_owned(),
        ..inside
    };

    assert_eq!(outside.url("abc"), None);
}
