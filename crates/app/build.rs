use std::io::Write as _;

#[derive(Clone, Copy)]
enum Headers {
    Public,
    Private,
}

fn main() {
    emit("cargo::rerun-if-env-changed=QMAKE");
    for source in ["src/icon_theme.cpp", "src/macos.mm", "src/linux_blur.cpp"] {
        emit(&format!("cargo::rerun-if-changed={source}"));
    }

    fallback_env("DRAKEFLAKE_QML_FALLBACK_PATH", "QMLLINT_IMPORT_PATH");
    fallback_env(
        "DRAKEFLAKE_ICON_RCC_FALLBACK_DIR",
        "DRAKEFLAKE_ICON_RCC_DIR",
    );
    fallback_env("DRAKEFLAKE_QT_PLUGIN_FALLBACK_PATH", "QT_PLUGIN_PATH");

    compile_qt("icon_theme", "src/icon_theme.cpp", Headers::Public, &[]);

    match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => {
            compile_qt("macos", "src/macos.mm", Headers::Private, &["-fobjc-arc"]);
            emit("cargo::rustc-link-lib=framework=AppKit");
            emit("cargo::rustc-link-lib=framework=UniformTypeIdentifiers");
            macos_resources(&qmake_query("QT_HOST_LIBEXECS"));
        }
        // Qt's wl_surface lives in private platform headers only.
        Ok("linux") => compile_qt("linux_blur", "src/linux_blur.cpp", Headers::Private, &[]),
        Ok(_) | Err(_) => {}
    }
}

fn fallback_env(name: &str, source: &str) {
    emit(&format!("cargo::rerun-if-env-changed={source}"));
    emit(&format!(
        "cargo::rustc-env={name}={}",
        std::env::var(source).unwrap_or_default()
    ));
}

fn compile_qt(name: &str, source: &str, headers: Headers, flags: &[&str]) {
    let public = qmake_query("QT_INSTALL_HEADERS");
    let mut build = cc::Build::new();

    build
        .cpp(true)
        .file(source)
        .include(&public)
        // No cc defaults because the Nix wrapper owns arch, sysroot and hardening flags.
        .no_default_flags(true)
        .inherit_rustflags(false);

    for flag in flags {
        build.flag(flag);
    }

    match headers {
        Headers::Public => {}
        Headers::Private => {
            let version = qmake_query("QT_VERSION");

            for dir in [
                format!("{public}/QtCore"),
                format!("{public}/QtGui"),
                format!("{public}/QtCore/{version}"),
                format!("{public}/QtGui/{version}"),
            ] {
                build.include(dir);
            }
        }
    }

    build.compile(name);
}

fn macos_resources(libexecs: &str) {
    let style =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/qml/style/DrakeFlakeMac");

    let mut style_files = String::new();
    for name in [
        "qmldir",
        "ItemDelegate.qml",
        "Label.qml",
        "ScrollView.qml",
        "ToolButton.qml",
    ] {
        // rerun-if-changed on a folder ignores edits to files, so list each file.
        emit(&format!(
            "cargo::rerun-if-changed={}",
            style.join(name).display()
        ));
        style_files.push_str("<file alias=\"");
        style_files.push_str(name);
        style_files.push_str("\">");
        style_files.push_str(&style.join(name).to_string_lossy());
        style_files.push_str("</file>");
    }

    let manifest = format!(
        "<RCC><qresource prefix=\"/qt/qml/DrakeFlakeMac\">{style_files}</qresource></RCC>\n"
    );

    let Some(out) = std::env::var_os("OUT_DIR").map(std::path::PathBuf::from) else {
        fail("cargo set no OUT_DIR");
    };

    let qrc = out.join("macos.qrc");
    if std::fs::write(&qrc, manifest).is_err() {
        fail("cannot write the macOS resource list");
    }

    let status = std::process::Command::new(format!("{libexecs}/rcc"))
        .arg("--binary")
        .arg(&qrc)
        .arg("-o")
        .arg(out.join("macos.rcc"))
        .status()
        .unwrap_or_else(|err| fail(&format!("cannot run rcc: {err}")));

    if !status.success() {
        fail("rcc failed for the macOS resources");
    }
}

fn emit(directive: &str) {
    let line = format!("{directive}\n");
    if std::io::stdout().write_all(line.as_bytes()).is_err() {
        fail("cannot write a cargo directive to stdout");
    }
}

fn fail(message: &str) -> ! {
    let line = format!("drakeflake-app build script: {message}\n");

    // Nothing is left to report to when stderr is gone.
    std::io::stderr().write_all(line.as_bytes()).ok();
    #[expect(
        clippy::exit,
        reason = "a build script reports failure through its exit code"
    )]
    std::process::exit(1);
}

fn qmake_query(var: &str) -> String {
    let qmake = std::env::var("QMAKE").unwrap_or_else(|_| "qmake6".to_owned());
    let out = std::process::Command::new(&qmake)
        .args(["-query", var])
        .output()
        .unwrap_or_else(|err| fail(&format!("cannot run {qmake}: {err}")));

    if !out.status.success() {
        fail(&format!(
            "{qmake} -query {var} failed, Qt headers not found"
        ));
    }

    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}
