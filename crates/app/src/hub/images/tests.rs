use super::*;

#[test]
fn format_names() {
    assert!(is_format_name("iso-installer"));
    assert!(is_format_name("qemu_efi"));
    assert!(!is_format_name("iso; rm"));
    assert!(!is_format_name(""));
}

#[test]
fn unknown_formats_are_refused_before_anything_runs() {
    let hub = super::super::test_support::hub_with(None);

    assert_eq!(
        hub.build_image("", "iso;rm".to_owned())
            .map_err(|error| error.to_string()),
        Err("\"iso;rm\" is not an image format".to_owned())
    );
    assert!(!hub.operations().is_running());
}

#[cfg(target_os = "linux")]
#[test]
fn images_are_built_for_nixos_hosts_of_the_selected_flake() {
    let selected = Target {
        flake: "/etc/nixos".to_owned(),
        host: drakeflake_render::HostName::from("box"),
        kind: SystemKind::NixOs,
    };

    let hub = super::super::test_support::hub_with(Some(selected.clone()));

    assert_eq!(
        image_target(hub.session(), "").map_err(|error| error.to_string()),
        Ok(selected)
    );
    assert_eq!(
        image_target(hub.session(), "installer").map_err(|error| error.to_string()),
        Ok(Target {
            flake: "/etc/nixos".to_owned(),
            host: drakeflake_render::HostName::from("installer"),
            kind: SystemKind::NixOs,
        })
    );
    assert_eq!(
        image_target(super::super::test_support::hub_with(None).session(), "")
            .map_err(|error| error.to_string()),
        Err("No system is selected yet".to_owned())
    );

    let darwin = super::super::test_support::hub_with(Some(Target {
        kind: SystemKind::Darwin,
        ..Target::default()
    }));

    assert!(
        image_target(darwin.session(), "").is_err(),
        "a darwin host builds images"
    );
}

#[test]
fn only_a_missing_images_attribute_means_no_image_builder() {
    let missing = |attr: &str| FlakeError::MissingAttribute {
        attr: attr.to_owned(),
        summary: format!("error: attribute '{attr}' missing"),
    };

    assert!(image_list_error(&missing("images")).starts_with("This NixOS has no image builder"));
    assert_eq!(
        image_list_error(&missing("other")),
        "error: attribute 'other' missing"
    );
    assert_eq!(
        image_list_error(&FlakeError::Nix("error: images are broken".to_owned())),
        "error: images are broken"
    );
}
