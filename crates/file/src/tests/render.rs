use std::path::{Path, PathBuf};

use drakeflake_config::LayoutConfig;
use drakeflake_exec::Context;
use drakeflake_plan::{
    HEADER, ManagedOption, ManagedState, Rendered, STATE_PREFIX, WritePlan, generated_files,
};

use super::{featured, find_in_path, option, sample};
use crate::state::IMPORT_MARKER_COMMENT;
use crate::*;

#[test]
fn renders_sorted_with_priorities() {
    let rendered = sample().render().unwrap();

    assert!(rendered.starts_with(HEADER));
    assert!(rendered.contains("environment.systemPackages = [ pkgs.kdePackages.kate pkgs.vim ];"));
    assert!(rendered.contains("a.opt = lib.mkOverride 90 true;"));
    assert!(rendered.contains("b.opt = lib.mkOverride 90 2;"));
    assert!(rendered.contains("networking.firewall.allowedTCPPorts = [ 22 80 ];"));

    let pos_a = rendered.find("a.opt").unwrap();
    let pos_b = rendered.find("b.opt").unwrap();
    assert!(pos_a < pos_b);
}

#[test]
fn state_roundtrips_through_the_file() {
    let state = sample();
    let rendered = state.render().unwrap();

    assert_eq!(ManagedState::parse(&rendered).unwrap(), Some(state));
    assert_eq!(ManagedState::parse("{ }").unwrap(), None);
    ManagedState::parse("# drakeflake-state: {broken").unwrap_err();
}

#[test]
fn invalid_inputs_are_reported_not_rendered() {
    let mut state = ManagedState::default();
    state.options.insert("x".into(), option("boolean", "maybe"));
    state.packages.insert("not a package".into());

    let errors = state.render().unwrap_err();
    assert_eq!(errors.len(), 2);
}

#[test]
fn empty_state_renders_valid_module() {
    let rendered = ManagedState::default().render().unwrap();
    assert!(rendered.contains(&format!(
        "{{ config, lib, pkgs, ... }}:\n{{\n  {IMPORT_MARKER_COMMENT}\n  {IMPORT_MARKER}\n}}"
    )));
}

#[test]
fn strings_cannot_inject_nix() {
    let mut state = ManagedState::default();
    state.options.insert(
        "networking.hostName".into(),
        option("string", "x\"; system.evil = true; \"${builtins.abort 1}"),
    );

    let rendered = state.render().unwrap();
    assert!(rendered.contains(
        "networking.hostName = lib.mkOverride 90 \"x\\\"; system.evil = true; \\\"\\${builtins.abort 1}\";"
    ));
}

#[test]
fn folder_entry_keeps_its_parts_beside_it() {
    let state = sample();
    let rendered = state
        .render_layout(
            &LayoutConfig::default(),
            &settings_folder(Path::new("/f/default.nix")),
        )
        .unwrap();

    assert!(rendered.parts.iter().all(|part| !part.path.starts_with("/")
        && !part.path.to_string_lossy().starts_with("drakeflake")));
    assert!(rendered.entry.contains("    ./"), "{}", rendered.entry);
    assert!(
        rendered
            .parts
            .iter()
            .all(|part| part.text.contains("imported by default.nix"))
    );
    assert_eq!(
        settings_folder(Path::new("/f/drakeflake.nix")),
        "drakeflake"
    );
}

#[test]
fn split_layout_renders_and_plans() {
    let state = sample();
    let layout = LayoutConfig::default();
    let rendered = state.render_layout(&layout, "drakeflake").unwrap();

    let paths: Vec<String> = rendered
        .parts
        .iter()
        .map(|part| part.path.display().to_string())
        .collect();

    assert_eq!(
        paths,
        vec![
            "drakeflake/a.nix",
            "drakeflake/b.nix",
            "drakeflake/networking/firewall.nix",
            "drakeflake/packages.nix",
        ]
    );
    assert!(
        rendered
            .entry
            .contains("    ./drakeflake/networking/firewall.nix")
    );
    assert!(!rendered.entry.contains("mkOverride"), "entry only imports");
    assert_eq!(
        ManagedState::parse(&rendered.entry).unwrap(),
        Some(state.clone())
    );
    assert_eq!(generated_files(&rendered.entry).len(), 4);

    let firewall = &rendered.parts[2].text;
    assert!(firewall.starts_with(HEADER));
    assert!(firewall.contains("networking.firewall.allowedTCPPorts = [ 22 80 ];"));

    let entry = Path::new("/flake/drakeflake.nix");
    let single = state.render().unwrap();
    let plan = WritePlan::for_rendered(
        entry,
        &Rendered {
            entry: single,
            parts: Vec::new(),
        },
        Some(&rendered.entry),
        Path::new("/flake/drakeflake"),
    );

    assert_eq!(plan.write.len(), 1);
    assert_eq!(plan.remove.len(), 4);
    assert!(
        plan.remove
            .contains(&PathBuf::from("/flake/drakeflake/packages.nix"))
    );

    if find_in_path("nix-instantiate").is_some() {
        check_syntax(&Context::system(), &rendered.entry).unwrap();
        for part in &rendered.parts {
            check_syntax(&Context::system(), &part.text)
                .unwrap_or_else(|err| panic!("{}: {err}", part.path.display()));
        }
    }
}

#[test]
fn renders_nested_prioritised_expression_and_host_settings() {
    let state = featured();
    let rendered = state.render().unwrap();
    let body = rendered.split_once("}:\n").unwrap().1;

    assert_eq!(
        body,
        "{\n  \
           config = lib.mkMerge [\n    \
             {\n      \
               # No effect: lets DrakeFlake check that this file is imported.\n      \
               _module.args = { };\n      \
               networking = {\n        \
                 domain = lib.mkOverride 90 (config.networking.hostName + \"-vm\");\n        \
                 hostName = lib.mkOverride 90 \"x\\\"; system.evil = true; \\\"\\${builtins.abort 1}\";\n      \
               };\n      \
               services.openssh = {\n        \
                 enable = lib.mkOverride 90 true;\n        \
                 ports = lib.mkBefore [ 2222 ];\n      \
               };\n    \
             }\n    \
             (lib.mkIf (config.networking.hostName == \"dragons-strix\") {\n      \
               services.printing.enable = lib.mkForce true;\n    \
             })\n  \
           ];\n\
         }\n"
    );
    assert_eq!(ManagedState::parse(&rendered).unwrap(), Some(state.clone()));

    let split = state
        .render_layout(&LayoutConfig::default(), "drakeflake")
        .unwrap();

    assert!(
        !split.entry.contains("lib.mkMerge"),
        "entry has no host settings"
    );
    assert!(split.entry.contains(&format!("  {IMPORT_MARKER}\n")));

    let printing = split
        .parts
        .iter()
        .find(|part| part.path == Path::new("drakeflake/services/printing.nix"))
        .unwrap();

    assert!(
        printing
            .text
            .contains("  config = lib.mkMerge [\n    (lib.mkIf")
    );
    assert!(!printing.text.contains("_module.args"));

    if find_in_path("nix-instantiate").is_some() {
        check_syntax(&Context::system(), &rendered).unwrap();
        for part in &split.parts {
            check_syntax(&Context::system(), &part.text)
                .unwrap_or_else(|err| panic!("{}: {err}", part.path.display()));
        }
    }
}

#[test]
fn host_scope_is_refused_for_the_host_name() {
    let mut state = ManagedState::default();
    state.options.insert(
        "networking.hostName".into(),
        ManagedOption {
            host: Some("box".into()),
            ..option("string", "box")
        },
    );

    assert_eq!(state.render().unwrap_err().len(), 1);
}

#[test]
fn old_state_lines_still_load() {
    let old = format!(
        "{HEADER}{STATE_PREFIX}{{\"options\":{{\"a.b\":{{\"type\":\"boolean\",\"input\":\"true\"}}}},\"packages\":[\"vim\"]}}\n{{ }}\n"
    );

    let state = ManagedState::parse(&old).unwrap().unwrap();

    assert_eq!(state.options["a.b"], option("boolean", "true"));
    assert!(
        state
            .render()
            .unwrap()
            .contains("a.b = lib.mkOverride 90 true;")
    );
}

#[test]
fn expressions_are_checked_in_context() {
    if find_in_path("nix-instantiate").is_none() {
        return;
    }

    check_expression(&Context::system(), "lib.mkIf config.x.enable pkgs.hello").unwrap();
    check_expression(&Context::system(), "1 # trailing comment").unwrap();
    assert!(check_expression(&Context::system(), "1 +").is_err());
    assert!(check_expression(&Context::system(), "1); evil = true; x = (2").is_err());
}

#[test]
fn syntax_check() {
    if find_in_path("nix-instantiate").is_none() {
        return;
    }

    check_syntax(&Context::system(), &sample().render().unwrap()).unwrap();
    assert!(check_syntax(&Context::system(), "{ a = ; }").is_err());
}
