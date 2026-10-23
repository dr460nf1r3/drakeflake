use super::*;

#[test]
fn copies_are_writable() {
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("image.iso");
    std::fs::write(&image, "image").unwrap();
    std::fs::set_permissions(&image, std::os::unix::fs::PermissionsExt::from_mode(0o444)).unwrap();
    let copy = dir.path().join("copy.iso");

    copy_image(&image.to_string_lossy(), &copy).unwrap();
    assert_eq!(std::fs::read_to_string(&copy).unwrap(), "image");
    assert!(!std::fs::metadata(&copy).unwrap().permissions().readonly());
    assert!(copy_image("/nonexistent/image.iso", &copy).is_err());
}
