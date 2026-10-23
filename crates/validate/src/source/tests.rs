use drakeflake_render::OptionName;

use super::*;

fn lines(text: &str, option: &str) -> Vec<usize> {
    definition_sites(text, &OptionName::from(option))
        .iter()
        .map(|site| site.line)
        .collect()
}

#[test]
fn finds_dotted_and_nested_definitions() {
    let text = "{ config, lib, ... }:\n{\n  services.openssh.enable = true;\n  services = {\n    openssh = {\n      ports = [ 22 ];\n    };\n  };\n  networking.firewall.allowedTCPPorts = [ 22 ];\n}\n";

    assert_eq!(lines(text, "services.openssh.enable"), vec![3]);
    assert_eq!(lines(text, "services.openssh.ports"), vec![6]);

    assert_eq!(lines(text, "services.openssh"), vec![3, 5]);
    assert_eq!(lines(text, "services.nginx.enable"), Vec::<usize>::new());
}

#[test]
fn finds_dotted_definitions_under_config() {
    let text = "{ config, ... }:\n{\n  config.services.openssh.enable = true;\n  config.services.openssh.ports = [ 22 ];\n  config.services = { nginx.enable = true; };\n}\n";

    assert_eq!(lines(text, "services.openssh.enable"), vec![3]);
    assert_eq!(lines(text, "services.openssh"), vec![3, 4]);
    assert_eq!(lines(text, "services.nginx.enable"), vec![5]);
}

#[test]
fn follows_module_wrappers() {
    let text = r#"{ config, lib, pkgs, ... }:
let port = 22; in
{
  imports = [ ./hardware.nix ];
  config = lib.mkMerge [
{ services.openssh.enable = lib.mkForce true; }
(lib.mkIf (config.networking.hostName == "box") {
  services.openssh.settings.PermitRootLogin = "no";
})
  ];
}
"#;

    assert_eq!(lines(text, "services.openssh.enable"), vec![6]);
    assert_eq!(
        lines(text, "services.openssh.settings.PermitRootLogin"),
        vec![8]
    );
}

#[test]
fn reports_columns_and_quoted_names() {
    let text = "{\n  boot.kernel.sysctl.\"net.ipv4.ip_forward\" = 1;\n}\n";
    assert_eq!(
        definition_sites(
            text,
            &OptionName::from("boot.kernel.sysctl.net.ipv4.ip_forward")
        ),
        Vec::new(),
        "a quoted name is one attribute, not a path"
    );
    assert_eq!(
        definition_sites(text, &OptionName::from("boot.kernel.sysctl")),
        vec![Site { line: 2, column: 3 }]
    );
}

#[test]
fn does_not_match_text_in_strings_or_comments() {
    let text = "{\n  # services.openssh.enable = true;\n  description = \"services.openssh.enable = x\";\n}\n";
    assert_eq!(
        definition_sites(text, &OptionName::from("services.openssh.enable")),
        Vec::<Site>::new()
    );
}

#[test]
fn follows_with_if_and_inherit() {
    let text = "{ pkgs, lib, openssh, ... }:
with lib;
{
  services = if true then {
    nginx.enable = true;
  } else {
    nginx.enable = false;
  };
  programs = { inherit openssh; };
}
";

    assert_eq!(lines(text, "services.nginx.enable"), vec![5, 7]);
    assert_eq!(lines(text, "programs.openssh"), vec![9]);
    assert_eq!(lines(text, "programs.openssh.enable"), Vec::<usize>::new());
}

#[test]
fn dynamic_names_are_not_guessed() {
    let text = "{ name, ... }:\n{\n  services.${name}.enable = true;\n  services.\"${name}\".port = 1;\n  services.\"\".x = 2;\n}\n";

    assert_eq!(lines(text, "services.openssh.enable"), Vec::<usize>::new());
    assert_eq!(lines(text, "services.openssh.port"), Vec::<usize>::new());

    assert_eq!(lines(text, "services..x"), vec![5]);
}

#[test]
fn imports_are_found_through_parentheses_and_let() {
    let text = "args:\nlet extra = ./x.nix; in\n({\n  imports = [ (./a.nix) ~/b.nix /etc/c.nix extra ];\n})\n";

    assert_eq!(import_paths(text), vec!["./a.nix", "~/b.nix", "/etc/c.nix"]);
    assert_eq!(import_paths("# only a comment\n"), Vec::<String>::new());
    assert_eq!(import_paths("{ x = 1; }"), Vec::<String>::new());
}

#[test]
fn columns_count_characters_not_bytes() {
    assert_eq!(site_at("ä = 1;\nöü = 2;", 10), Site { line: 2, column: 2 });
    assert_eq!(site_at("abc", 99), Site { line: 1, column: 4 });
}

#[test]
fn invalid_files_do_not_panic() {
    assert!(definition_sites("{ a = ", &OptionName::from("a")).len() <= 1);
    assert!(import_paths("{ imports = [ ./a.nix").len() <= 1);
}

#[test]
fn lists_literal_imports() {
    let text = "{ lib, ... }:
{
  imports = [ ./drakeflake.nix ../modules/x.nix ]
++ lib.optional (builtins.pathExists ./extra.nix) ./extra.nix;
  config.imports = [ ./not-top-level.nix ];
  # imports = [ ./commented.nix ];
}
";

    assert_eq!(
        import_paths(text),
        vec!["./drakeflake.nix", "../modules/x.nix", "./extra.nix"]
    );
}
