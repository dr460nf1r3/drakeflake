use super::*;

#[test]
fn locate_output_to_attributes() {
    assert_eq!(
        attributes("ripgrep.out\nripgrep-all.out\npython313Packages.foo.bin\nripgrep.out\n"),
        vec!["ripgrep", "ripgrep-all", "python313Packages.foo"]
    );
    let os = if cfg!(target_os = "macos") {
        "-darwin"
    } else {
        "-linux"
    };

    assert!(database_url().ends_with(os));
}

#[test]
fn other_suffixes_are_part_of_the_attribute() {
    assert_eq!(
        attributes("  \n  nodePackages.prettier \nqt6.qtbase.dev\nqt6.qtbase.out\n\n"),
        vec!["nodePackages.prettier", "qt6.qtbase"]
    );
    assert_eq!(attributes(""), Vec::<String>::new());
}

#[test]
fn only_program_names_are_looked_up() {
    let hub = Hub::default();

    for command in ["", "/usr/bin/rg", "../rg", "rg --help", "rg\n"] {
        assert_eq!(
            locate(hub.services(), &hub.nix(), command),
            Err(CommandLookupError::Failed(
                "Type the name of a program, e.g. rg".to_owned()
            )),
            "{command:?}"
        );
    }
}
