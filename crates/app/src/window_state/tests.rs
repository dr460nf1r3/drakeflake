use std::io::Write as _;

use super::*;

fn write_temp(content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

#[test]
fn missing_file_means_unset() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing/window.json");

    assert_eq!(load_from(&path), Geometry::default());
}

#[test]
fn broken_file_means_unset() {
    let file = write_temp("{unclosed");

    assert_eq!(load_from(file.path()), Geometry::default());
}

#[test]
fn default_is_unset_without_touching_the_disk() {
    assert_eq!(WindowState::default().geometry(), Geometry::default());
}

#[test]
fn absurd_geometries_are_rejected() {
    assert!(geometry_valid(30_i32, 40_i32, 800_i32, 600_i32));
    assert!(!geometry_valid(0_i32, 0_i32, 0_i32, 600_i32));
    assert!(!geometry_valid(0_i32, 0_i32, 800_i32, -1_i32));
    assert!(!geometry_valid(i32::MIN, i32::MAX, i32::MAX, i32::MAX));
}

#[test]
fn geometry_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("window.json");
    let geometry = Geometry {
        x: Some(100_i32),
        y: Some(120_i32),
        width: Some(1_152_i32),
        height: Some(736_i32),
    };

    save_to(&geometry, &path).unwrap();
    assert_eq!(load_from(&path), geometry);

    assert!(std::fs::read_dir(dir.path()).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("tmp-")
    }));
}

#[test]
fn saving_creates_missing_folders() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a/b/window.json");
    let geometry = Geometry {
        width: Some(800_i32),
        height: Some(600_i32),
        ..Geometry::default()
    };

    save_to(&geometry, &nested).unwrap();
    assert_eq!(load_from(&nested), geometry);
}
