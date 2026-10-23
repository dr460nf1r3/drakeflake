.pragma library


function pages() {
    return [page("options", qsTr("Options"), qsTr("Search and change NixOS options"), qsTr("nixos settings configuration")), page("home", qsTr("Home Manager"), qsTr("Settings of the users on this host"), qsTr("user home dotfiles")), page("packages", qsTr("Packages"), qsTr("Add, remove or try packages"), qsTr("software programs apps install")), page("functions", qsTr("Functions"), qsTr("Nix functions of lib, builtins and pkgs, with documentation"), qsTr("noogle lib builtins function documentation type")), page("generations", qsTr("Generations"), qsTr("Go back to an earlier system"), qsTr("rollback boot previous")), page("actions", qsTr("Actions"), qsTr("Rebuild, create images, start in a VM"), qsTr("image iso vm qemu")), page("cleanup", qsTr("Cleanup"), qsTr("Free disk space"), qsTr("garbage collect gc disk space delete")), page("inputs", qsTr("Inputs"), qsTr("Check and update flake.lock"), qsTr("flake lock update upgrade")), page("impermanence", qsTr("Impermanence"), qsTr("What survives a reboot and what is removed"), qsTr("persistence persist tmpfs btrfs rollback ephemeral wipe")), page("changes", qsTr("Changes"), qsTr("Review and apply staged changes"), qsTr("staged apply save")), page("configured", qsTr("Configured"), qsTr("Everything set in your flake"), qsTr("set values defined")), page("history", qsTr("History"), qsTr("Earlier applies and their files"), qsTr("backup restore undo")), page("log", qsTr("Console"), qsTr("Output of the programs the app runs"), qsTr("log output terminal")), page("settings", qsTr("Settings"), qsTr("Flake, host, layout, tray and logging"), qsTr("preferences configure")), page("docs", qsTr("Documentation"), qsTr("The handbook: how to use each page"), qsTr("help manual handbook guide")), page("about", qsTr("About"), qsTr("Version, license and links"), qsTr("version license help"))];
}

function actions() {
    return [entry(qsTr("Rebuild and switch"), qsTr("Build the system and switch to it now"), qsTr("nixos-rebuild nh switch apply"), {
            kind: "rebuild",
            mode: "switch"
        }), entry(qsTr("Rebuild for the next boot"), qsTr("Build the system and use it from the next start"), qsTr("nixos-rebuild boot"), {
            kind: "rebuild",
            mode: "boot"
        }), entry(qsTr("Rebuild until the next reboot"), qsTr("Switch to it now without making it the default"), qsTr("nixos-rebuild test try"), {
            kind: "rebuild",
            mode: "test"
        }), entry(qsTr("Build only"), qsTr("Build the system without switching to it"), qsTr("nixos-rebuild build check"), {
            kind: "rebuild",
            mode: "build"
        }), entry(qsTr("Update all inputs"), qsTr("Lock every flake input to its newest version"), qsTr("flake update upgrade lock"), {
            kind: "update-inputs"
        }), entry(qsTr("Check inputs for updates"), qsTr("Ask upstream for newer versions"), qsTr("flake lock outdated"), section("inputs")), entry(qsTr("Clean up old generations"), qsTr("Delete old systems and free disk space"), qsTr("garbage collect gc"), section("cleanup")), entry(qsTr("Create an image"), qsTr("ISO, VM or cloud image of a host"), qsTr("iso qcow vm image"), imageSection()), entry(qsTr("Start in a VM"), qsTr("Try a host in QEMU"), qsTr("qemu virtual machine test"), imageSection()), entry(qsTr("Review and apply changes"), qsTr("Staged changes, preview and apply"), qsTr("staged save apply"), section("changes")), entry(qsTr("Setup assistant"), qsTr("Choose the flake, host and output file again"), qsTr("wizard first start import"), {
            kind: "setup"
        })];
}

function settings() {
    return [entry(qsTr("System flake"), qsTr("Settings › System"), qsTr("flake folder path"), section("settings")), entry(qsTr("Host"), qsTr("Settings › System"), qsTr("hostname machine"), section("settings")), entry(qsTr("Output file"), qsTr("Settings › System"), qsTr("generated file drakeflake.nix"), section("settings")), entry(qsTr("Reload data"), qsTr("Settings › System"), qsTr("refresh"), section("settings")), entry(qsTr("Rebuild caches"), qsTr("Settings › System"), qsTr("cache index"), section("settings")), entry(qsTr("File layout"), qsTr("Settings › Layout"), qsTr("split files parts rules"), section("settings")), entry(qsTr("Tray icon"), qsTr("Settings › Tray"), qsTr("notification autostart login"), section("settings")), entry(qsTr("Highlighting theme"), qsTr("Settings › Highlighting"), qsTr("colours colors dark light"), section("settings")), entry(qsTr("Log detail"), qsTr("Settings › Logging"), qsTr("debug log level"), section("settings"))];
}

function imageSection() {
    return Object.assign(section("actions"), {
        images: true
    });
}

function section(name) {
    return {
        kind: "section",
        section: name
    };
}

function page(name, title, subtitle, keywords) {
    return entry(title, subtitle, keywords, section(name));
}

function entry(title, subtitle, keywords, open) {
    return {
        title: title,
        subtitle: subtitle,
        keywords: keywords,
        open: open
    };
}
