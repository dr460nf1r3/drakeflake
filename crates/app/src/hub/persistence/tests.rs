use super::offers_snapshot;

#[test]
fn only_a_btrfs_root_offers_the_blank_snapshot() {
    assert!(offers_snapshot(Some("btrfs")));
    assert!(!offers_snapshot(Some("tmpfs")));
    assert!(!offers_snapshot(Some("ext4")));
    assert!(!offers_snapshot(None));
}
