use std::fs;
use std::path::{Path, PathBuf};

use drakeflake_config::{LayoutConfig, LayoutMode};
use drakeflake_exec::Context;
use drakeflake_fs::atomic_write;
use drakeflake_plan::{
    FILE_NAME, HEADER, ManagedState, Rendered, WritePlan, check_write_content, check_write_target,
};

use super::{find_in_path, sample};
use crate::imports::imports_file;
use crate::*;

#[test]
fn load_missing_and_foreign_files() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    let path = managed_path(&dir);

    assert_eq!(ManagedState::load(&path).unwrap(), ManagedState::default());

    fs::write(&path, "{ ... }: { }").unwrap();
    ManagedState::load(&path).unwrap_err();

    let state = sample();
    atomic_write(&path, &state.render().unwrap()).unwrap();
    assert_eq!(ManagedState::load(&path).unwrap(), state);

    let backups = dir.join("backups");
    let made = backup_to(&path, &backups).unwrap().unwrap();
    assert!(made.is_file());
    assert_eq!(backup_to(&dir.join("absent.nix"), &backups).unwrap(), None);

    atomic_write(&path, "{ }").unwrap();
    assert!(!is_imported(&dir, &path));
    fs::write(
        dir.join("configuration.nix"),
        "{ imports = [ ./drakeflake.nix ]; }",
    )
    .unwrap();
    assert!(is_imported(&dir, &path));
    assert!(!is_imported(&dir, &dir.join("modules/other.nix")));
    assert!(!is_imported(&dir, &dir.join("modules").join(FILE_NAME)));

    fs::create_dir_all(dir.join("hosts")).unwrap();
    fs::write(
        dir.join("hosts/laptop.nix"),
        "{ imports = [ ../modules/drakeflake.nix ]; }",
    )
    .unwrap();
    assert!(is_imported(&dir, &dir.join("modules").join(FILE_NAME)));

    // The Nix build sandbox has no git: then the state is honestly unknown.
    let expected = if find_in_path("git").is_some() {
        GitState::NotARepository
    } else {
        GitState::Unknown
    };

    assert_eq!(git_state(&Context::system(), &path), expected);
}

#[test]
fn a_folder_import_counts_for_its_default_nix() {
    let flake = tempfile::tempdir().unwrap();
    let folder = flake.path().join("modules/system");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join(FOLDER_ENTRY), "{ ... }: { }").unwrap();

    fs::write(
        flake.path().join("modules/default.nix"),
        "{ ... }:\n{\n  imports = [\n    ./acme\n    ./system\n  ];\n}\n",
    )
    .unwrap();

    assert!(is_imported(flake.path(), &folder.join(FOLDER_ENTRY)));
    assert!(!is_imported(flake.path(), &folder.join(FILE_NAME)));
}

#[test]
fn imports_are_read_from_the_syntax_tree() {
    let dir = Path::new("/f");
    let entry = Path::new("/f/drakeflake.nix");

    assert!(imports_file(
        "{ imports = [ ./drakeflake.nix ]; }",
        dir,
        entry
    ));
    assert!(imports_file(
        "{ lib, ... }: { imports = lib.optional true ./drakeflake.nix; }",
        dir,
        entry
    ));

    assert!(!imports_file(
        "{ # imports = [ ./drakeflake.nix ];\n}",
        dir,
        entry
    ));
    assert!(!imports_file(
        "{ description = \"see ./drakeflake.nix\"; }",
        dir,
        entry
    ));
    assert!(!imports_file(
        "{ imports = [ ./modules/drakeflake.nix ]; }",
        dir,
        entry
    ));
    assert!(imports_file(
        "{ imports = [ ./modules/drakeflake.nix ]; }",
        dir,
        Path::new("/f/modules/drakeflake.nix")
    ));
}

#[test]
fn a_layout_change_supersedes_the_other_entry() {
    let dir = tempfile::tempdir().unwrap();
    let old_entry = dir.path().join(FILE_NAME);
    let state = sample();
    let rendered = state
        .render_layout(&LayoutConfig::default(), &settings_folder(&old_entry))
        .unwrap();

    fs::write(&old_entry, &rendered.entry).unwrap();

    let superseded = superseded_files(&dir.path().join(FOLDER_ENTRY));

    assert!(superseded.contains(&old_entry));
    assert!(superseded.len() > 1, "its parts go too: {superseded:?}");
    assert!(
        superseded_files(&old_entry).is_empty(),
        "no default.nix yet"
    );
    assert_eq!(
        resolve_managed_path(
            None,
            Some(&format!("{}/", dir.path().display())),
            LayoutMode::Area
        )
        .unwrap(),
        dir.path().join(FOLDER_ENTRY)
    );
    assert_eq!(
        resolve_managed_path(
            None,
            Some(&format!("{}/", dir.path().display())),
            LayoutMode::Single
        )
        .unwrap(),
        old_entry
    );
}

#[test]
fn write_plan_applies_directly() {
    let temp = tempfile::tempdir().unwrap();
    let flake = temp.path().to_path_buf();
    fs::create_dir_all(&flake).unwrap();
    fs::write(flake.join("flake.nix"), "{ outputs = _: { }; }").unwrap();
    let entry = flake.join(FILE_NAME);

    let split = sample()
        .render_layout(&LayoutConfig::default(), "drakeflake")
        .unwrap();

    let plan = WritePlan::for_rendered(&entry, &split, None, &flake.join("drakeflake"));
    if let Err(err) = plan.clone().validate() {
        assert!(err.to_string().contains("nix-instantiate"), "{err}");
    }

    assert_eq!(
        write_files(&Context::system(), &plan, None).unwrap(),
        WriteMethod::Direct
    );
    assert!(flake.join("drakeflake/networking/firewall.nix").is_file());

    let single = Rendered {
        entry: sample().render().unwrap(),
        parts: Vec::new(),
    };

    let back = WritePlan::for_rendered(
        &entry,
        &single,
        Some(&split.entry),
        &flake.join("drakeflake"),
    );

    assert_eq!(
        write_files(&Context::system(), &back, None).unwrap(),
        WriteMethod::Direct
    );
    assert!(!flake.join("drakeflake/networking/firewall.nix").exists());
    assert!(
        !flake.join("drakeflake/networking").exists(),
        "empty dir removed"
    );
}

#[test]
fn helper_write_targets_are_restricted() {
    let temp = tempfile::tempdir().unwrap();
    let flake = temp.path().to_path_buf();
    fs::create_dir_all(flake.join("modules")).unwrap();
    fs::write(flake.join("flake.nix"), "{ outputs = _: { }; }").unwrap();

    let new_file = flake.join("modules").join(FILE_NAME);
    check_write_target(&new_file).unwrap();

    fs::write(&new_file, sample().render().unwrap()).unwrap();
    check_write_target(&new_file).unwrap();

    let foreign = flake.join("configuration.nix");
    fs::write(&foreign, "{ ... }: { }").unwrap();
    assert!(check_write_target(&foreign).is_err());

    assert!(check_write_target(Path::new("relative.nix")).is_err());
    assert!(check_write_target(&flake.join("notes.txt")).is_err());
    assert!(check_write_target(&flake.join("modules/../x.nix")).is_err());
    check_write_target(&flake.join("missing-dir/x.nix")).unwrap();
    assert!(check_write_target(Path::new("/etc/not-a-flake.nix")).is_err());

    #[cfg(unix)]
    {
        let link = flake.join("link.nix");
        std::os::unix::fs::symlink(&foreign, &link).unwrap();
        assert!(check_write_target(&link).is_err());
    }
}

#[test]
fn helper_write_content_is_restricted() {
    assert!(check_write_content("{ }").is_err());
    assert!(check_write_content(&format!("{HEADER}{{ {{")).is_err());

    if find_in_path("nix-instantiate").is_some() {
        check_write_content(&sample().render().unwrap()).unwrap();
    }
}

#[test]
fn output_path_resolution() {
    let temp = tempfile::tempdir().unwrap();
    let flake = temp.path().to_path_buf();
    fs::create_dir_all(flake.join("modules")).unwrap();
    let single = LayoutMode::Single;

    assert_eq!(
        resolve_managed_path(Some(&flake), None, single).unwrap(),
        flake.join(FILE_NAME)
    );
    assert_eq!(
        resolve_managed_path(Some(&flake), Some("  "), single).unwrap(),
        flake.join(FILE_NAME)
    );
    assert_eq!(
        resolve_managed_path(
            Some(&flake),
            Some(&flake.join("modules").to_string_lossy()),
            single
        )
        .unwrap(),
        flake.join("modules").join(FILE_NAME)
    );
    let custom = flake.join("modules/settings.nix");
    assert_eq!(
        resolve_managed_path(None, Some(&custom.to_string_lossy()), single).unwrap(),
        custom
    );
    assert!(matches!(
        resolve_managed_path(None, None, single),
        Err(FileError::NoLocalFlake)
    ));
    assert!(matches!(
        resolve_managed_path(Some(&flake), Some("relative.nix"), single),
        Err(FileError::OutputNotAbsolute(_))
    ));
    assert!(matches!(
        resolve_managed_path(Some(&flake), Some("/etc/not-nix.txt"), single),
        Err(FileError::OutputNotNix(_))
    ));
    assert_eq!(
        resolve_managed_path(None, Some("/new/dir/"), single).unwrap(),
        PathBuf::from("/new/dir").join(FILE_NAME)
    );

    assert!(is_inside(&flake, &custom));
    assert!(!is_inside(&flake, Path::new("/etc/settings.nix")));
}

#[test]
fn imports_match_through_a_symlinked_flake_folder() {
    let temp = tempfile::tempdir().unwrap();
    let real = temp.path().join("real");
    fs::create_dir_all(real.join("hosts")).unwrap();
    let alias = temp.path().join("alias");
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    fs::write(
        real.join("hosts/laptop.nix"),
        "{ imports = [ ../modules/drakeflake.nix ]; }",
    )
    .unwrap();

    // The output folder does not exist yet, and the flake is opened by its
    // symlinked name (`/etc/nix-darwin` on macOS).
    assert!(is_imported(&alias, &alias.join("modules").join(FILE_NAME)));
}
