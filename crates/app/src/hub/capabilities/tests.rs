use super::*;

#[test]
fn nixos_on_linux_has_every_mode() {
    let nixos = Capabilities::of(SystemKind::NixOs, SystemKind::NixOs);

    assert!(nixos.native && nixos.generations && nixos.images);
    assert_eq!(
        nixos.generation_modes,
        [Mode::Switch, Mode::Test, Mode::Boot]
    );
    assert!(nixos.apply.contains(&Mode::Test) && nixos.apply.contains(&Mode::Boot));
    assert_eq!(nixos.review, [Mode::Switch, Mode::Boot, Mode::Test]);
}

#[test]
fn darwin_on_a_mac_only_builds_and_switches() {
    let darwin = Capabilities::of(SystemKind::Darwin, SystemKind::Darwin);

    assert!(darwin.native && darwin.generations && !darwin.images);
    assert_eq!(darwin.generation_modes, [Mode::Switch]);
    assert_eq!(darwin.apply, [Mode::Save, Mode::Build, Mode::Switch]);
    assert_eq!(darwin.review, [Mode::Switch]);
    assert_eq!(darwin.rebuild, [Mode::Build, Mode::Switch]);
}

#[test]
fn the_other_platform_is_save_only() {
    for (kind, native) in [
        (SystemKind::Darwin, SystemKind::NixOs),
        (SystemKind::NixOs, SystemKind::Darwin),
    ] {
        let other = Capabilities::of(kind, native);

        assert!(!other.native && !other.generations && !other.images);
        assert_eq!(other.generation_modes, Vec::<Mode>::new());
        assert_eq!(other.apply, [Mode::Save]);
        assert_eq!(other.review, Vec::<Mode>::new());
        assert_eq!(other.rebuild, Vec::<Mode>::new());
    }
}

#[test]
fn json_names_the_modes() {
    let json = Capabilities::of(SystemKind::Darwin, SystemKind::Darwin).to_json();

    assert_eq!(json["apply"], json!(["save", "build", "switch"]));
    assert_eq!(json["native"], json!(true));
    assert_eq!(json["platform"], json!("darwin"));
    assert_eq!(json["default_flake"], json!("/etc/nix-darwin"));
    assert_eq!(json["generations"], json!(true));
    assert_eq!(json["generation_modes"], json!(["switch"]));
    assert_eq!(json["images"], json!(false));
}
