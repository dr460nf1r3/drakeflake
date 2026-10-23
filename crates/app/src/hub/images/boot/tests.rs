use super::*;

#[test]
fn efi_follows_the_format_not_the_path() {
    assert!(is_efi_format("qemu-efi"));
    assert!(is_efi_format("raw-efi"));
    assert!(!is_efi_format("qemu"));
    assert!(!is_efi_format("iso"));
    assert!(!is_efi_format("legacy-efiboot"));
}

#[test]
fn qemu_values_escape_commas() {
    assert_eq!(qemu_value("a,file=b"), "a,,file=b");
}

#[test]
fn isos_boot_as_cd_and_disks_as_throwaway_snapshots() {
    let cases = [
        (
            "/nix/store/x/image.iso",
            vec!["-cdrom", "/nix/store/x/image.iso"],
        ),
        (
            "/nix/store/x/IMAGE.ISO",
            vec!["-cdrom", "/nix/store/x/IMAGE.ISO"],
        ),
        (
            "/nix/store/x/disk.qcow2",
            vec!["-drive", "file=/nix/store/x/disk.qcow2,snapshot=on"],
        ),
        // A comma in the path must not start a second option (`format=…`).
        (
            "/nix/store/x/a,format=raw.img",
            vec!["-drive", "file=/nix/store/x/a,,format=raw.img,snapshot=on"],
        ),
    ];

    for (file, tail) in cases {
        let command = vm_command(Path::new("qemu"), file);
        let tail: Vec<OsString> = tail.into_iter().map(arg).collect();
        assert_eq!(command.first(), Some(&arg("qemu")));
        assert!(command.ends_with(&tail), "{file}: {command:?}");
    }
}
