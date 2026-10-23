use std::cell::Cell;
use std::fs;

use super::*;

const PROPERTIES: &[&str] = &["icon.name:", "source:", "icon:", "iconName:"];

fn qml_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();

        if path.is_dir() {
            qml_files(&path, found);
            continue;
        }

        if path
            .extension()
            .is_some_and(|ext| ext == "qml" || ext == "js")
        {
            found.push(path);
        }
    }
}

fn icon_names(line: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut parts = line.split('"');
    let mut before = parts.next().unwrap_or_default();

    while let (Some(inside), Some(after)) = (parts.next(), parts.next()) {
        let lead = before.trim_end();

        let compared = lead.ends_with("===") || lead.ends_with("!==") || lead.ends_with('(');
        let icon_like = inside
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_lowercase())
            && inside
                .chars()
                .all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-');

        if !compared && icon_like {
            names.push(inside.to_owned());
        }

        before = after;
    }

    names
}

#[test]
fn every_icon_the_qml_uses_has_a_symbol() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/qml");
    let mut files = Vec::new();
    qml_files(&root, &mut files);

    let mut missing = Vec::new();

    for file in files {
        for line in fs::read_to_string(&file).unwrap().lines() {
            let trimmed = line.trim_start();

            if !PROPERTIES
                .iter()
                .any(|property| trimmed.starts_with(property))
                || ["resolvedUrl", ".svg", ".png", "qrc:"]
                    .iter()
                    .any(|skip| trimmed.contains(skip))
            {
                continue;
            }

            for name in icon_names(trimmed) {
                if !SYMBOLS.iter().any(|(mapped, _)| *mapped == name) {
                    missing.push(format!("{name} ({})", file.display()));
                }
            }
        }
    }

    assert_eq!(missing, Vec::<String>::new());
}

#[test]
fn compared_strings_are_not_icons() {
    assert_eq!(
        icon_names(r#"icon.name: state === "missing" ? "data-error" : "search""#),
        vec!["data-error", "search"]
    );
}

#[test]
fn names_are_unique() {
    let mut names: Vec<&str> = SYMBOLS.iter().map(|(name, _)| *name).collect();
    names.sort_unstable();
    names.dedup();

    assert_eq!(names.len(), SYMBOLS.len());
}

#[test]
fn the_theme_inherits_breeze_and_lists_every_folder() {
    let index = index_theme();

    assert!(index.contains("Inherits=breeze\n"), "{index}");
    assert!(
        index.contains("[16x16/actions]\nSize=16\nScale=1\n"),
        "{index}"
    );
    assert!(
        index.contains("[48x48@2/actions]\nSize=48\nScale=2\n"),
        "{index}"
    );
    assert_eq!(
        index.matches("Type=Fixed").count(),
        SIZES.len() * SCALES.len()
    );
}

#[test]
fn light_and_dark_get_their_own_folders() {
    let cache = Path::new("/cache");

    assert_ne!(search_dir(cache, true), search_dir(cache, false));
    assert!(search_dir(cache, true).starts_with("/cache/icons"));
}

#[test]
fn a_rendered_theme_is_reused() {
    let dir = tempfile::tempdir().unwrap();
    let drawn = Cell::new(0_usize);
    let render = |_: &str, _: u32, _: u32, path: &Path| {
        drawn.set(drawn.get() + 1);
        fs::write(path, b"png").is_ok()
    };

    install(dir.path(), render).unwrap();
    let first = drawn.get();

    assert_eq!(first, SYMBOLS.len() * SIZES.len() * SCALES.len());
    assert!(
        dir.path()
            .join(THEME_NAME)
            .join("16x16@2/actions/go-next.png")
            .is_file()
    );

    install(dir.path(), render).unwrap();

    assert_eq!(drawn.get(), first, "rendered twice");
}

#[test]
fn missing_symbols_still_complete_the_theme() {
    let dir = tempfile::tempdir().unwrap();

    install(dir.path(), |symbol, _, _, path| {
        symbol != "scroll" && fs::write(path, b"png").is_ok()
    })
    .unwrap();

    assert!(dir.path().join(THEME_NAME).join("index.theme").is_file());
}
