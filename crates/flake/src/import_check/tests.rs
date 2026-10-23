use super::*;

#[test]
fn imported_file_is_recognised_by_its_path_in_the_flake() {
    let files = vec![
        "/nix/store/abc-source/hosts/box/default.nix".to_owned(),
        "/nix/store/abc-source/modules/system/drakeflake.nix".to_owned(),
    ];

    assert!(lists_file(
        &files,
        Path::new("modules/system/drakeflake.nix")
    ));
    assert!(!lists_file(&files, Path::new("drakeflake.nix")));
    assert!(!lists_file(&files, Path::new("nix/drakeflake.nix")));

    let folder = vec!["/nix/store/abc-source/modules/system".to_owned()];
    assert!(lists_file(&folder, Path::new("modules/system/default.nix")));
    assert!(!lists_file(
        &folder,
        Path::new("modules/system/drakeflake.nix")
    ));
}
