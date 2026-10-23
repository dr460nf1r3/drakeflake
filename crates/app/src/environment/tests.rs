use super::*;

#[test]
fn a_style_the_user_set_is_kept() {
    let unused = || panic!("Breeze is only looked up for a replacement");

    assert_eq!(replacement_style("Material", false, unused), None);
    assert_eq!(replacement_style("Material", true, unused), None);
    assert_eq!(
        replacement_style("macOS", false, unused),
        None,
        "only macOS itself swaps the plain macOS style"
    );
}

#[test]
fn the_desktop_style_gives_way_to_breeze_or_fusion() {
    for current in ["", "org.kde.desktop"] {
        assert_eq!(
            replacement_style(current, false, || true),
            Some("org.kde.breeze")
        );
        assert_eq!(replacement_style(current, false, || false), Some("Fusion"));
    }
}

#[test]
fn macos_uses_its_own_style() {
    for current in ["", "org.kde.desktop", "macOS"] {
        assert_eq!(
            replacement_style(current, true, || true),
            Some(crate::macos::CONTROLS_STYLE),
            "{current:?}"
        );
    }
}

#[test]
fn a_qml_module_needs_its_qmldir() {
    let with = tempfile::tempdir().unwrap();
    let without = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(with.path().join("org/kde/breeze")).unwrap();
    std::fs::write(
        with.path().join("org/kde/breeze/qmldir"),
        "module org.kde.breeze\n",
    )
    .unwrap();
    std::fs::create_dir_all(without.path().join("org/kde/breeze")).unwrap();

    let dirs = |list: &[&tempfile::TempDir]| -> Vec<std::path::PathBuf> {
        list.iter().map(|dir| dir.path().to_path_buf()).collect()
    };

    assert!(module_in(&dirs(&[&without, &with]), "org/kde/breeze"));
    assert!(!module_in(&dirs(&[&without]), "org/kde/breeze"));
    assert!(!module_in(&dirs(&[&with]), "org/kde/kirigami"));
    assert!(!module_in(&[], "org/kde/breeze"));
}
