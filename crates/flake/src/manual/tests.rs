use super::*;
use crate::flake_target::SystemKind;

#[test]
fn manual_expression_per_kind() {
    let expr = |kind| {
        manual_expr(&Target {
            flake: "/etc/nix-darwin".to_owned(),
            host: "mac".into(),
            kind,
        })
    };

    let darwin = expr(SystemKind::Darwin);
    assert!(darwin.contains("flake.darwinConfigurations.\"mac\".extendModules"));
    let flake = Path::new("/etc/nix-darwin").canonicalize().map_or_else(
        |_| "/etc/nix-darwin".to_owned(),
        |path| path.to_string_lossy().into_owned(),
    );

    assert!(darwin.contains(&format!("builtins.getFlake \"{flake}\"")));
    assert!(!darwin.contains("__"));
    assert!(expr(SystemKind::NixOs).contains("flake.nixosConfigurations.\"mac\""));
}

#[test]
fn options_json_is_found_in_the_nixos_or_darwin_manual() {
    let store = tempfile::tempdir().unwrap();
    let output = |name: &str, doc: Option<&str>| {
        let out = store.path().join(name);
        let dir = out.join("share/doc").join(doc.unwrap_or("other"));
        std::fs::create_dir_all(&dir).unwrap();
        if doc.is_some() {
            std::fs::write(dir.join("options.json"), "{}").unwrap();
        }

        out.to_string_lossy().into_owned()
    };

    let nixos = output("nixos-manual", Some("nixos"));
    let darwin = output("darwin-manual", Some("darwin"));
    let empty = output("empty", None);

    assert_eq!(
        options_json_in(&format!("warning: Git tree is dirty\n  {nixos}  \n")).unwrap(),
        Path::new(&nixos).join("share/doc/nixos/options.json")
    );
    assert_eq!(
        options_json_in(&darwin).unwrap(),
        Path::new(&darwin).join("share/doc/darwin/options.json")
    );
    assert!(matches!(
        options_json_in(&empty),
        Err(FlakeError::NoOptionsJson(out)) if out == empty
    ));
    assert!(matches!(
        options_json_in("warning: no output\n"),
        Err(FlakeError::NoOutputPath)
    ));
}
