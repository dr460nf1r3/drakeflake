use drakeflake_flake::SystemKind;

use super::*;

#[test]
fn cached_hosts_round_trip() {
    let cached = CachedHosts {
        key: "hosts v4 nixos 3 files 0123".to_owned(),
        hosts: vec![
            Host {
                name: "box".to_owned(),
                kind: SystemKind::NixOs,
            },
            Host {
                name: "mac".to_owned(),
                kind: SystemKind::Darwin,
            },
        ],
    };

    let text = serde_json::to_string(&cached).unwrap();
    assert!(text.contains(r#""kind":"darwin""#), "{text}");

    let read: CachedHosts = serde_json::from_str(&text).unwrap();
    assert_eq!(read.key, cached.key);
    assert_eq!(read.hosts, cached.hosts);
}
