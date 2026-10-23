use super::*;

fn setting(path: &str, value: &str) -> Setting {
    Setting {
        path: path.split('.').map(str::to_owned).collect(),
        value: RenderedValue {
            text: value.to_owned(),
            verbatim: false,
        },
    }
}

fn lines(settings: &[Setting]) -> String {
    settings_lines(settings, 0).join("\n")
}

#[test]
fn shared_prefixes_nest() {
    let text = lines(&[
        setting("services.openssh.ports", "[ 2222 ]"),
        setting("networking.hostName", "\"x\""),
        setting("services.openssh.enable", "lib.mkOverride 90 true"),
    ]);

    assert_eq!(
        text,
        "networking.hostName = \"x\";\n\
         services.openssh = {\n  \
           enable = lib.mkOverride 90 true;\n  \
           ports = [ 2222 ];\n\
         };"
    );
}

#[test]
fn nesting_is_recursive_and_quotes_names() {
    let text = lines(&[
        setting("services.openssh.enable", "true"),
        setting("services.openssh.settings.PermitRootLogin", "\"no\""),
        setting("services.openssh.settings.X11Forwarding", "false"),
        setting("services.printing.enable", "true"),
        setting("users.users.my user.isNormalUser", "true"),
    ]);

    assert_eq!(
        text,
        "services = {\n  \
           openssh = {\n    \
             enable = true;\n    \
             settings = {\n      \
               PermitRootLogin = \"no\";\n      \
               X11Forwarding = false;\n    \
             };\n  \
           };\n  \
           printing.enable = true;\n\
         };\n\
         users.users.\"my user\".isNormalUser = true;"
    );
}

#[test]
fn a_setting_and_its_parent_stay_flat() {
    let text = lines(&[
        setting("a.b", "{ }"),
        setting("a.b.c", "1"),
        setting("x.y", "2"),
    ]);

    assert_eq!(text, "a = {\n  b = { };\n  b.c = 1;\n};\nx.y = 2;");
    assert_eq!(
        lines(&[setting("a", "{ }"), setting("a.c", "1")]),
        "a = { };\na.c = 1;"
    );
}

#[test]
fn multi_line_values_are_indented_unless_verbatim() {
    let mut nested = setting("a.x", "{\n  k = 1;\n}");
    let verbatim = Setting {
        path: vec!["a".to_owned(), "y".to_owned()],
        value: RenderedValue {
            text: "(\n''\n  kept\n''\n)".to_owned(),
            verbatim: true,
        },
    };

    nested.value.verbatim = false;

    assert_eq!(
        lines(&[nested, verbatim]),
        "a = {\n  x = {\n    k = 1;\n  };\n  y = (\n''\n  kept\n''\n);\n};"
    );
}

#[test]
fn host_settings_use_the_merge_form() {
    let body = Body {
        imports: vec!["part.nix".to_owned()],
        preamble: vec!["_module.args = { };".to_owned()],
        shared: vec![setting("a.b", "1")],
        hosts: BTreeMap::from([(
            "box \"1\"".into(),
            vec![setting("c.d", "2"), setting("c.e", "3")],
        )]),
    };

    assert_eq!(
        module_text("", &body),
        "{ config, lib, pkgs, ... }:\n{\n  \
           imports = [\n    ./part.nix\n  ];\n\n  \
           config = lib.mkMerge [\n    \
             {\n      \
               _module.args = { };\n      \
               a.b = 1;\n    \
             }\n    \
             (lib.mkIf (config.networking.hostName == \"box \\\"1\\\"\") {\n      \
               c = {\n        d = 2;\n        e = 3;\n      };\n    \
             })\n  \
           ];\n\
         }\n"
    );
}
