use std::sync::mpsc;

#[cfg(target_os = "linux")]
use drakeflake_flake::{SystemKind, Target};

#[cfg(target_os = "linux")]
use super::test_support::hub_with;
use super::*;

#[test]
fn option_messages_reach_every_view_of_their_tree() {
    let hub = Hub::default();
    hub.mail.post_options(OptionsMsg::Reset);
    hub.mail.post_home(OptionsMsg::Reset);
    hub.mail.post_home(OptionsMsg::Reset);

    let counts = [
        hub.mail.options.take().len(),
        hub.mail.configured.take().len(),
        hub.mail.tree.take().len(),
        hub.mail.home.take().len(),
        hub.mail.home_tree.take().len(),
    ];

    assert_eq!(counts, [1, 1, 1, 2, 2]);
}

#[test]
fn shutdown_cancels_the_load_and_the_running_operation() {
    let hub = Arc::new(Hub::default());
    let load = hub.evaluation().load_token();
    let wait = Duration::from_secs(10);
    let (release, held) = mpsc::channel::<()>();
    let (started, running) = mpsc::channel::<()>();
    let (seen, cancelled) = mpsc::channel::<bool>();

    hub.spawn_exclusive("held", Reporting::Shown, move |run| {
        started.send(()).unwrap();
        held.recv_timeout(wait).unwrap();
        seen.send(run.is_cancelled()).unwrap();
        Outcome::Cancelled
    })
    .unwrap();
    running.recv_timeout(wait).unwrap();

    hub.shutdown();
    release.send(()).unwrap();

    assert!(load.is_cancelled(), "evaluations keep running");
    assert!(
        cancelled.recv_timeout(wait).unwrap(),
        "the running operation was not cancelled"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_darwin_host_on_linux_can_only_be_saved() {
    assert!(hub_with(None).capabilities().native);

    let darwin = hub_with(Some(Target {
        flake: "/etc/nix-darwin".to_owned(),
        host: drakeflake_render::HostName::from("mac"),
        kind: SystemKind::Darwin,
    }));

    let capabilities = darwin.capabilities();
    assert!(!capabilities.native);
    assert_eq!(capabilities.apply, &[Mode::Save]);
}
