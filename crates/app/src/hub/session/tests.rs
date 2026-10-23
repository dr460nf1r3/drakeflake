use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use drakeflake_flake::SystemKind;

use super::*;
use crate::hub::Generation;

fn target(host: &str) -> Target {
    Target {
        flake: "/etc/nixos".to_owned(),
        host: host.into(),
        kind: SystemKind::NixOs,
    }
}

fn loaded(session: &Session, host: &str) {
    session.update(|state| {
        state.target = Some(target(host));
        state.fresh = Some(format!("fingerprint of {host}"));
        state.index_key = Some(format!("index of {host}"));
    });
}

#[test]
fn a_new_load_forgets_everything_of_the_old_one_at_once() {
    let session = Session::default();
    loaded(&session, "box");

    let start = session.begin_load(Some("  ".to_owned()), |current| {
        current.is_some_and(|loaded| loaded.host == "box")
    });

    let state = session.snapshot();
    assert_eq!(start.generation, Generation(1));
    assert!(start.same_system);
    assert_eq!(state.target, None);
    assert_eq!(state.fresh, None);
    assert_eq!(state.index_key, None);
    assert_eq!(state.managed_setting, None, "blank means the default file");
}

#[test]
fn a_superseded_load_cannot_store_its_target() {
    let session = Session::default();
    let first = session.begin_load(None, |_| false).generation;
    let second = session.begin_load(None, |_| false).generation;

    assert!(!session.is_current(first));
    assert!(!session.store_target_if_current(first, &target("old")));
    assert_eq!(session.target(), None);

    assert!(session.store_target_if_current(second, &target("new")));
    assert_eq!(
        session.target().map(|loaded| loaded.host),
        Some("new".into())
    );
}

#[test]
fn a_reader_never_sees_a_target_with_the_fingerprint_of_another() {
    let session = Arc::new(Session::default());
    let stop = Arc::new(AtomicBool::new(false));

    let writer = {
        let session = Arc::clone(&session);
        let stop = Arc::clone(&stop);

        std::thread::spawn(move || {
            let mut round = 0_u32;
            while !stop.load(Ordering::SeqCst) {
                round += 1;
                loaded(&session, &format!("host{}", round % 7));
                session.begin_load(None, |_| false);
            }
        })
    };

    for _ in 0_u32..20_000 {
        let state = session.snapshot();

        if let Some(target) = state.target {
            assert_eq!(state.fresh, Some(format!("fingerprint of {}", target.host)));
            assert_eq!(state.index_key, Some(format!("index of {}", target.host)));
        } else {
            assert_eq!(state.fresh, None, "a fingerprint without its target");
            assert_eq!(state.index_key, None, "an index key without its target");
        }
    }

    stop.store(true, Ordering::SeqCst);
    writer.join().unwrap();
}

#[test]
fn the_written_layout_reports_a_change_once() {
    let session = Session::default();

    assert!(
        session.note_written_layout(),
        "the first write has no layout before"
    );
    assert!(!session.note_written_layout());

    session.set_layout(LayoutConfig {
        mode: LayoutMode::Single,
        rules: Vec::new(),
    });
    assert!(session.note_written_layout());
}

#[test]
fn the_managed_file_needs_a_target() {
    let session = Session::default();
    assert!(matches!(session.managed_file(), Err(HubError::NoTarget)));

    let dir = tempfile::tempdir().unwrap();
    session.update(|state| {
        state.target = Some(Target {
            flake: dir.path().to_string_lossy().into_owned(),
            host: "box".into(),
            kind: SystemKind::NixOs,
        });
    });

    assert_eq!(
        session.managed_file().unwrap(),
        dir.path().join("drakeflake.nix")
    );
}
