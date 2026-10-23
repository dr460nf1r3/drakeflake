use std::path::Path;

use super::DocsModel;
use super::guide::{Block, Entry, Tone, anchor_of, contents, is_page_name, page};

const BASE: &str = "/guide";

fn blocks(markdown: &str) -> Vec<Block> {
    page("x.md", markdown, Path::new(BASE)).blocks
}

fn text(text: &str) -> Block {
    Block::Text {
        text: text.to_owned(),
    }
}

#[test]
fn summary_entries_keep_their_part() {
    let summary = "# Summary\n\n[Introduction](introduction.md)\n\n# Using the app\n\n- [Options](options.md)\n- [Broken](../etc/passwd)\n";

    assert_eq!(
        contents(summary),
        vec![
            Entry {
                part: String::new(),
                title: "Introduction".to_owned(),
                file: "introduction.md".to_owned(),
            },
            Entry {
                part: "Using the app".to_owned(),
                title: "Options".to_owned(),
                file: "options.md".to_owned(),
            },
        ]
    );
}

#[test]
fn page_names_are_plain_markdown_files() {
    assert!(is_page_name("home-manager.md"));
    assert!(!is_page_name("../secret.md"));
    assert!(!is_page_name("images/x.md"));
    assert!(!is_page_name("options.txt"));
    assert!(!is_page_name(".md"));
}

#[test]
fn anchors_match_mdbook() {
    assert_eq!(anchor_of("Change an option"), "change-an-option");
    assert_eq!(anchor_of("Where is this set?"), "where-is-this-set");
    assert_eq!(anchor_of("3. Import"), "3-import");
    assert_eq!(anchor_of("The `nh` [tool](https://x)"), "the-nh-tool");
}

#[test]
fn the_first_heading_is_the_title() {
    let parsed = page(
        "x.md",
        "# Title\n\nIntro.\n\n## Usage\n\n## Usage",
        Path::new(BASE),
    );

    assert_eq!(parsed.title, "Title");
    assert_eq!(
        parsed.blocks,
        vec![
            text("Intro."),
            Block::Heading {
                level: 2,
                text: "Usage".to_owned(),
                anchor: "usage".to_owned(),
            },
            Block::Heading {
                level: 2,
                text: "Usage".to_owned(),
                anchor: "usage-1".to_owned(),
            },
        ]
    );
}

#[test]
fn code_blocks_are_split_out() {
    assert_eq!(
        blocks("Before.\n\n```nix\n  # not a heading\n  x = 1;\n```\nAfter."),
        vec![
            text("Before."),
            Block::Code {
                language: "nix".to_owned(),
                text: "# not a heading\nx = 1;".to_owned(),
            },
            text("After."),
        ]
    );
}

#[test]
fn local_images_resolve_against_the_guide() {
    assert_eq!(
        blocks("![The Overview](images/overview.png)"),
        vec![Block::Image {
            source: format!("file://{BASE}/images/overview.png"),
            alt: "The Overview".to_owned(),
            width: 0,
            natural_width: 0,
            natural_height: 0,
        }]
    );
    assert_eq!(
        blocks("<img src=\"images/logo.svg\" alt=\"Logo\" width=\"160\">"),
        vec![Block::Image {
            source: format!("file://{BASE}/images/logo.svg"),
            alt: "Logo".to_owned(),
            width: 160,
            natural_width: 0,
            natural_height: 0,
        }]
    );
}

#[test]
fn png_images_carry_their_size_for_the_reserved_space() {
    let dir = tempfile::tempdir().unwrap();
    let images = dir.path().join("images");
    std::fs::create_dir_all(&images).unwrap();
    std::fs::write(images.join("shot.png"), png(320, 200)).unwrap();

    assert_eq!(
        page("x.md", "![Shot](images/shot.png)", dir.path()).blocks,
        vec![Block::Image {
            source: format!("file://{}/images/shot.png", dir.path().display()),
            alt: "Shot".to_owned(),
            width: 0,
            natural_width: 320,
            natural_height: 200,
        }]
    );
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82];
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 2, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

#[test]
fn other_images_become_their_alt_text() {
    assert_eq!(blocks("![remote](https://x/y.png)"), vec![text("remote")]);
    assert_eq!(blocks("![up](../../etc/x.png)"), vec![text("up")]);
    assert_eq!(blocks("![root](/etc/x.png)"), vec![text("root")]);
}

#[test]
fn quotes_become_notes() {
    assert_eq!(
        blocks("Text.\n> **Warning:** Deletes\nthings.\n\n> **Tip:** Hi.\n\nMore."),
        vec![
            text("Text."),
            Block::Note {
                tone: Tone::Warning,
                text: "**Warning:** Deletes\nthings.".to_owned(),
            },
            Block::Note {
                tone: Tone::Info,
                text: "**Tip:** Hi.".to_owned(),
            },
            text("More."),
        ]
    );
}

fn note(text: &str) -> Block {
    Block::Note {
        tone: Tone::Info,
        text: text.to_owned(),
    }
}

#[test]
fn a_heading_ends_a_quote() {
    assert_eq!(
        blocks("> note\n## Next\nlazy"),
        vec![
            note("note"),
            Block::Heading {
                level: 2,
                text: "Next".to_owned(),
                anchor: "next".to_owned(),
            },
            text("lazy"),
        ]
    );
}

#[test]
fn an_image_ends_a_quote() {
    assert_eq!(
        blocks("> note\n![Shot](images/shot.png)"),
        vec![
            note("note"),
            Block::Image {
                source: format!("file://{BASE}/images/shot.png"),
                alt: "Shot".to_owned(),
                width: 0,
                natural_width: 0,
                natural_height: 0,
            },
        ]
    );
}

#[test]
fn a_paragraph_tag_ends_a_quote() {
    assert_eq!(
        blocks("> note\n<p align=\"center\">\nafter"),
        vec![note("note"), text("after")]
    );
}

#[test]
fn encoded_or_cut_image_paths_become_their_alt_text() {
    assert_eq!(blocks("![dots](images/%2e%2e/x.png)"), vec![text("dots")]);
    assert_eq!(blocks("![query](images/x.png?y)"), vec![text("query")]);
    assert_eq!(blocks("![part](images/x.png#y)"), vec![text("part")]);
    assert_eq!(blocks("![back](images\\x.png)"), vec![text("back")]);
}

#[test]
fn anchors_keep_every_lowercase_char() {
    assert_eq!(anchor_of("İ"), "i\u{307}");
}

#[test]
fn the_note_tone_reaches_qml_in_lowercase() {
    let json = serde_json::to_value(blocks("> **Warning:** x\n\n> y")).unwrap();

    assert_eq!(json[0]["kind"], "note");
    assert_eq!(json[0]["tone"], "warning");
    assert_eq!(json[1]["tone"], "info");
}

#[test]
fn paragraph_tags_are_dropped() {
    assert_eq!(
        blocks("<p align=\"center\">\n</p>\n\n<p align=\"center\"><em>NixOS, tamed.</em></p>"),
        vec![text("<em>NixOS, tamed.</em>")]
    );
}

#[test]
fn other_tags_starting_with_p_stay() {
    assert_eq!(
        blocks("<pre>\n\n<picture>x</p>"),
        vec![text("<pre>\n\n<picture>x</p>")]
    );
}

#[test]
fn errors_do_not_name_paths() {
    let model = DocsModel::at(Path::new(BASE).to_path_buf());

    for error in [
        model.page("../Cargo.toml".to_owned())["error"].clone(),
        model.page("missing.md".to_owned())["error"].clone(),
    ] {
        let shown = error.as_str().unwrap();
        assert!(!shown.contains('/') && !shown.contains("Cargo"), "{shown}");
    }
}

#[test]
fn the_model_reads_the_real_guide() {
    let model = DocsModel::new();
    let contents = model.contents();
    let entries = contents["entries"].as_array().unwrap();

    assert_eq!(contents["error"], "");
    assert!(entries.len() > 5, "{contents}");

    for entry in entries {
        let file = entry["file"].as_str().unwrap();
        let page = model.page(file.to_owned());
        assert_eq!(page["error"], "", "{file}");
        assert_eq!(
            page["title"], entry["title"],
            "{file}: title and SUMMARY.md differ"
        );
    }
}

#[test]
fn the_model_refuses_paths() {
    let model = DocsModel::at(Path::new(BASE).to_path_buf());

    assert_ne!(model.page("../Cargo.toml".to_owned())["error"], "");
    assert_ne!(model.page("missing.md".to_owned())["error"], "");
    assert_ne!(model.contents()["error"], "");
}
