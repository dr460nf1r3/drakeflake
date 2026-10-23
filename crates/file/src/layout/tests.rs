use super::*;

fn layout(mode: LayoutMode) -> LayoutConfig {
    LayoutConfig {
        mode,
        rules: Vec::new(),
    }
}

#[test]
fn modes() {
    let area_module = layout(LayoutMode::AreaModule);
    let in_area_module = |option: &str| file_for(&area_module, option);

    assert_eq!(
        in_area_module("services.openssh.enable").as_deref(),
        Some("services/openssh.nix")
    );
    assert_eq!(
        in_area_module("networking.hostName").as_deref(),
        Some("networking.nix")
    );
    assert_eq!(
        in_area_module("boot.kernel.sysctl.\"net.ipv4.ip_forward\"").as_deref(),
        Some("boot/kernel.nix")
    );
    assert_eq!(packages_file(&area_module).as_deref(), Some("packages.nix"));

    assert_eq!(
        in_area_module("home-manager.users.alice.programs.git.enable").as_deref(),
        Some("home-manager/alice/programs/git.nix")
    );
    assert_eq!(
        in_area_module("home-manager.users.alice.home.stateVersion").as_deref(),
        Some("home-manager/alice/home.nix")
    );

    let area = layout(LayoutMode::Area);
    assert_eq!(
        file_for(&area, "home-manager.users.bob.programs.git.enable").as_deref(),
        Some("home-manager/bob/programs.nix")
    );
    assert_eq!(
        file_for(&area, "services.openssh.enable").as_deref(),
        Some("services.nix")
    );

    let single = layout(LayoutMode::Single);
    assert_eq!(file_for(&single, "services.openssh.enable"), None);
    assert_eq!(packages_file(&single), None);
}

#[test]
fn rules_win_and_most_specific_rule_applies() {
    let config = LayoutConfig {
        mode: LayoutMode::Single,
        rules: LayoutConfig::parse_rules(
            "# remote access\nservices.openssh = remote-access\nservices = services.nix\nenvironment.systemPackages = apps/desktop.nix\n",
        )
        .unwrap(),
    };

    assert_eq!(
        file_for(&config, "services.openssh.enable").as_deref(),
        Some("remote-access.nix")
    );
    assert_eq!(
        file_for(&config, "services.printing.enable").as_deref(),
        Some("services.nix")
    );

    assert_eq!(file_for(&config, "servicesX.a"), None);
    assert_eq!(file_for(&config, "networking.hostName"), None);
    assert_eq!(packages_file(&config).as_deref(), Some("apps/desktop.nix"));
}

#[test]
fn segments_are_file_name_safe() {
    assert_eq!(segment("a.b c"), "a_b_c");
    assert_eq!(segment(""), "_");
}
