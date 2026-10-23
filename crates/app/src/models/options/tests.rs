mod tree;
mod values;

use std::collections::HashSet;

use super::rows::{OptionRow, short_path, tree_shape};
use super::tree::visible_nodes;

#[test]
fn store_paths_are_shortened() {
    assert_eq!(
        short_path("/nix/store/abc123-source/hosts/laptop.nix"),
        "hosts/laptop.nix"
    );
    assert_eq!(
        short_path("/etc/nixos/configuration.nix"),
        "/etc/nixos/configuration.nix"
    );
    assert_eq!(short_path("/nix/store/only-name"), "/nix/store/only-name");
}

#[test]
fn tree_shape_lists_folders_and_options_depth_first() {
    let names = ["programs.vim", "services", "services.xserver.enable"];
    let shape = tree_shape(&names);
    let paths: Vec<&str> = shape.iter().map(|row| row.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "programs",
            "programs.vim",
            "services",
            "services.xserver",
            "services.xserver.enable"
        ]
    );

    let by_path = |path: &str| shape.iter().find(|row| row.path == path).unwrap();
    assert_eq!(by_path("services").depth, 0);
    assert!(by_path("services").is_option);
    assert!(by_path("services").has_children);
    assert_eq!(by_path("services").option_count, 2);
    assert!(!by_path("services.xserver").is_option);
    assert_eq!(by_path("services.xserver.enable").depth, 2);
    assert!(!by_path("services.xserver.enable").has_children);
}

#[test]
fn namespaces_keep_their_json_shape() {
    let counts = vec![("services".to_owned(), 1234), ("boot".to_owned(), 7)];
    let rows = super::namespace_rows(counts.clone());
    let by_hand: Vec<serde_json::Value> = counts
        .into_iter()
        .map(|(name, count)| serde_json::json!({ "name": name, "count": count }))
        .collect();

    assert_eq!(rows, serde_json::Value::Array(by_hand));
}

#[test]
fn collapsed_folders_hide_their_contents() {
    let node = |name: &str, has_children: bool| OptionRow {
        name: name.to_owned(),
        has_children,
        ..OptionRow::default()
    };

    let nodes = [
        node("services", true),
        node("services.openssh", true),
        node("services.openssh.enable", false),
        node("services.xserver", true),
        node("services.xserver.enable", false),
        node("servicesExtra", false),
    ];

    let shown = |collapsed: &[&str]| {
        let collapsed: HashSet<String> = collapsed.iter().map(|name| (*name).to_owned()).collect();
        visible_nodes(&nodes, &collapsed)
            .into_iter()
            .map(|row| format!("{}{}", row.name, if row.open { "" } else { " (closed)" }))
            .collect::<Vec<_>>()
    };

    assert_eq!(shown(&[]).len(), 6);
    assert_eq!(
        shown(&["services.openssh"]),
        [
            "services",
            "services.openssh (closed)",
            "services.xserver",
            "services.xserver.enable",
            "servicesExtra"
        ]
    );

    assert_eq!(shown(&["services"]), ["services (closed)", "servicesExtra"]);
}
