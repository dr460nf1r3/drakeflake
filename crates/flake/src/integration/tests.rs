use std::path::PathBuf;

use drakeflake_exec::{LONG_TIMEOUT, SHORT_TIMEOUT};
use drakeflake_render::{OptionName, PackageAttr};
use serde_json::Value;

use super::{copied_target, nix, target};
use crate::lock::{UpdateCheck, check_updates, read_lock};
use crate::nix::Query;
use crate::{SystemKind, Target, eval_value, package_meta, run_nix};

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST with network"]
fn vm_eval_hostname() {
    let Some(target) = target() else {
        return;
    };

    let value = eval_value(&nix(), &target, &OptionName::from("networking.hostName")).unwrap();
    assert_eq!(value, Value::String(target.host.to_string()));
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST with network"]
fn vm_build_hello() {
    let Some(target) = target() else {
        return;
    };

    let installable = format!("{}.pkgs.hello", target.config_ref().unwrap());
    let dir = tempfile::TempDir::new().expect("temp dir for the out-link");
    let link = dir.path().join("hello").to_string_lossy().into_owned();

    let output = run_nix(
        &nix(),
        target.local_dir().as_deref(),
        [
            "build",
            &installable,
            "--out-link",
            &link,
            "--print-out-paths",
        ],
        LONG_TIMEOUT,
    )
    .unwrap();

    let path: PathBuf = output.lines().next().unwrap_or_default().trim().into();

    assert!(path.join("bin/hello").is_file(), "{output}");

    let run = std::process::Command::new(path.join("bin/hello"))
        .output()
        .expect("run the built hello");

    assert!(run.status.success(), "{run:?}");

    assert!(
        String::from_utf8_lossy(&run.stdout).contains("Hello"),
        "{run:?}"
    );
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST with network"]
fn vm_image_formats_listed() {
    let Some(target) = target() else {
        return;
    };

    let formats: Vec<String> = target
        .eval_json(
            &nix(),
            &Query::new("image formats", LONG_TIMEOUT)
                .attr(".config.system.build.images")
                .apply("builtins.attrNames"),
        )
        .unwrap();

    assert!(!formats.is_empty(), "no image formats for {}", target.host);

    assert!(formats.iter().all(|format| !format.is_empty()));
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST with network"]
fn vm_package_meta_hello() {
    let Some(target) = target() else {
        return;
    };

    let hello = package_meta(&nix(), &target, &PackageAttr::from("hello")).unwrap();
    assert_eq!(hello["mainProgram"], "hello");
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE with network"]
fn vm_lock_check_runs() {
    let Some(target) = target() else {
        return;
    };

    let Some(dir) = target.local_dir() else {
        return;
    };

    let inputs = read_lock(&dir).unwrap();
    assert!(
        inputs.iter().any(|input| input.name == "nixpkgs"),
        "{inputs:?}"
    );

    let checks = check_updates(nix().context(), &inputs);
    assert_eq!(checks.len(), inputs.len(), "{checks:?}");
    let failed = checks
        .iter()
        .filter(|(_, check)| matches!(check, UpdateCheck::Failed { error: _ }))
        .count();

    assert_eq!(failed, 0, "{checks:?}");
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST with network"]
fn vm_toplevels_seeded() {
    let Some(target) = target() else {
        return;
    };

    let second = std::env::var("DRAKEFLAKE_TEST_LIFECYCLE_FLAKE")
        .ok()
        .map(|flake| Target {
            flake,
            host: "lifecyclehost".into(),
            kind: SystemKind::NixOs,
        });

    for one in std::iter::once(&target).chain(second.as_ref()) {
        let installable = format!("{}.config.system.build.toplevel", one.config_ref().unwrap());

        let path = run_nix(
            &nix(),
            one.local_dir().as_deref(),
            ["eval", "--raw", &installable],
            LONG_TIMEOUT,
        )
        .unwrap();

        let path = path.trim();

        run_nix(&nix(), None, ["path-info", path], SHORT_TIMEOUT).unwrap();
    }
}

#[test]
#[ignore = "needs DRAKEFLAKE_TEST_FLAKE and DRAKEFLAKE_TEST_HOST with network"]
fn vm_flake_update_copy() {
    let Some((target, _copy)) = copied_target() else {
        return;
    };

    let Some(dir) = target.local_dir() else {
        return;
    };

    run_nix(
        &nix(),
        Some(dir.as_path()),
        ["flake", "update", "nixpkgs"],
        LONG_TIMEOUT,
    )
    .unwrap();

    let inputs = read_lock(&dir).unwrap();
    assert!(
        inputs.iter().any(|input| input.name == "nixpkgs"),
        "{inputs:?}"
    );
}
