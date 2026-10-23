use super::*;

#[test]
fn largest_file_is_the_image() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("iso")).unwrap();
    std::fs::write(dir.path().join("iso/small.txt"), "x").unwrap();
    std::fs::write(dir.path().join("iso/image.iso"), vec![0_u8; 1024]).unwrap();
    let found = largest_file(dir.path());
    assert!(found.is_some_and(|path| path.ends_with("iso/image.iso")));
}

#[test]
fn declared_files_stay_inside_the_image() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("output");
    std::fs::create_dir_all(output.join("iso")).unwrap();
    std::fs::write(output.join("iso/image.iso"), "x").unwrap();
    std::fs::write(dir.path().join("secret"), "x").unwrap();

    assert!(inside_output(&output, "iso/image.iso").is_some());
    assert!(inside_output(&output, "../secret").is_none());
    assert!(inside_output(&output, &dir.path().join("secret").to_string_lossy()).is_none());
}
