use std::os::unix::fs::symlink;

use drakeflake_flake::{SystemKind, Target};

use super::*;

fn profiles(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let store = dir.join("store");
    let system_a = store.join("system-a");
    let gaming = store.join("system-a-gaming");
    let system_b = store.join("system-b");
    std::fs::create_dir_all(system_a.join("specialisation")).unwrap();
    std::fs::create_dir_all(&gaming).unwrap();
    std::fs::create_dir_all(&system_b).unwrap();
    std::fs::create_dir_all(store.join("abc-linux-6.6.1")).unwrap();
    std::fs::create_dir_all(store.join("def-firmware")).unwrap();
    symlink(&gaming, system_a.join("specialisation/gaming")).unwrap();
    symlink(
        store.join("abc-linux-6.6.1/bzImage"),
        system_a.join("kernel"),
    )
    .unwrap();
    symlink(store.join("def-firmware/bzImage"), system_b.join("kernel")).unwrap();

    let profiles = dir.join("profiles");
    std::fs::create_dir_all(&profiles).unwrap();
    symlink(&system_a, profiles.join("system-1-link")).unwrap();
    symlink(&system_b, profiles.join("system-2-link")).unwrap();
    symlink(store.join("collected"), profiles.join("system-3-link")).unwrap();
    symlink(&system_a, profiles.join("system-x-link")).unwrap();
    symlink(&system_a, profiles.join("home-manager-1-link")).unwrap();
    symlink("system-2-link", profiles.join("system")).unwrap();

    (
        profiles,
        system_a.canonicalize().unwrap(),
        gaming.canonicalize().unwrap(),
    )
}

#[test]
fn running_and_booted_systems_are_marked() {
    let dir = tempfile::tempdir().unwrap();
    let (profiles, system_a, gaming) = profiles(dir.path());

    let listed = generations_in(
        &profiles,
        &Pointers {
            current: Some(PathBuf::from("system-2-link")),
            running: Some(gaming),
            booted: Some(system_a.clone()),
        },
    );

    assert_eq!(
        listed.iter().map(|found| found.id).collect::<Vec<_>>(),
        vec![2, 1],
        "dangling links and other profiles are left out"
    );
    let (newest, oldest) = (&listed[0], &listed[1]);
    assert!(newest.current && !newest.running && !newest.booted);
    assert_eq!(newest.kernel, "firmware");
    assert_eq!(newest.specialisations, Vec::<String>::new());
    assert!(!oldest.current && oldest.running && oldest.booted);
    assert_eq!(oldest.running_specialisation.as_deref(), Some("gaming"));
    assert_eq!(oldest.kernel, "Linux 6.6.1");
    assert_eq!(oldest.path, system_a.to_string_lossy());

    let plain = generations_in(
        &profiles,
        &Pointers {
            current: None,
            running: Some(system_a),
            booted: None,
        },
    );

    assert!(plain[1].running && plain[1].running_specialisation.is_none());
    assert!(!plain[0].running && !plain.iter().any(|found| found.current));
}

#[test]
fn missing_profiles_list_nothing() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(list_generations(&dir.path().join("missing")), Vec::new());
}

#[test]
fn generations_of_another_platform_are_not_touched() {
    let hub = Arc::new(Hub::default());
    let foreign = match SystemKind::native() {
        SystemKind::NixOs => SystemKind::Darwin,
        SystemKind::Darwin => SystemKind::NixOs,
    };

    hub.session().update(|state| {
        state.target = Some(Target {
            flake: "/etc/nixos".to_owned(),
            host: "other".into(),
            kind: foreign,
        });
    });

    for action in [GenerationAction::Compare, GenerationAction::Switch] {
        assert_eq!(
            hub.generation_action(1, action)
                .map_err(|error| error.to_string()),
            Err("This machine's generations are not this host's.".to_owned())
        );
    }

    assert!(!hub.operations().is_running());
}

#[test]
fn reads_profiles() {
    let dir = std::env::temp_dir().join(format!("drakeflake-generations-{}", std::process::id()));
    let system = dir.join("store-system");
    std::fs::create_dir_all(system.join("specialisation/gaming")).unwrap();
    std::fs::write(system.join("nixos-version"), "26.11.20261001\n").unwrap();
    let kernel_dir = dir.join("abc123-linux-7.2.8-cachyos");
    std::fs::create_dir_all(&kernel_dir).unwrap();
    symlink(kernel_dir.join("bzImage"), system.join("kernel")).unwrap();
    symlink(&system, dir.join("system-7-link")).unwrap();
    symlink(&system, dir.join("system-8-link")).unwrap();
    symlink("system-8-link", dir.join("system")).unwrap();

    let generations = list_generations(&dir);
    std::fs::remove_dir_all(&dir).unwrap();

    assert_eq!(
        generations
            .iter()
            .map(|generation| generation.id)
            .collect::<Vec<_>>(),
        vec![8, 7]
    );
    assert!(generations[0].current && !generations[1].current);
    assert_eq!(generations[0].version, "26.11.20261001");
    assert_eq!(generations[0].kernel, "Linux 7.2.8-cachyos");
    assert_eq!(generations[0].specialisations, vec!["gaming".to_owned()]);
}

#[test]
fn generation_actions() {
    assert_eq!("compare".parse(), Ok(GenerationAction::Compare));
    assert_eq!("boot".parse(), Ok(GenerationAction::Boot));
    assert_eq!(
        "rm -rf".parse::<GenerationAction>(),
        Err(UnknownAction("rm -rf".to_owned()))
    );
}

#[test]
fn live_profiles_do_not_fail() {
    let _generations = list_generations(Path::new(PROFILES));
}

#[test]
fn version_from_nixos_or_darwin() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(system_version(dir.path()), None);
    std::fs::write(dir.path().join("darwin-version"), "25.11.abcdef\n").unwrap();
    assert_eq!(system_version(dir.path()).as_deref(), Some("25.11.abcdef"));
    std::fs::write(dir.path().join("nixos-version"), "26.11.20261001").unwrap();
    assert_eq!(
        system_version(dir.path()).as_deref(),
        Some("26.11.20261001")
    );
}

#[test]
fn only_activations_have_a_mode() {
    assert_eq!(GenerationAction::Compare.mode(), None);
    assert_eq!(GenerationAction::Boot.mode(), Some(Mode::Boot));
}
