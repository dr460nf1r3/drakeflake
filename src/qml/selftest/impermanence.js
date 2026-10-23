.pragma library

const SYSTEM_DIRS = "environment.persistence.\"/persist\".directories";
const HOME_DIRS = "home-manager.users.alice.home.persistence.\"/persist\".directories";

function model(test) {
    return test.app.persistenceModel;
}

function page(test) {
    return test.app.pageStack.items[0];
}

function persisted(test, path) {
    return model(test).persisted.find(row => row.path === path);
}

function shown(test, path) {
    return model(test).ephemeral.some(row => row.path === path);
}

function open(test, paths) {
    for (const path of paths) {
        if (!model(test).ephemeral.some(row => row.path === path && row.expanded))
            model(test).toggle(path);
    }
}

function rowButton(test, list, path, name) {
    let row = null;
    const walk = item => {
        if (!item || row)
            return;
        if (item.modelData && item.modelData.path === path)
            row = item;
        for (const child of item.children || [])
            walk(child);
    };
    walk(test.findChild(page(test), list));
    const button = row ? test.findChild(row, name) : null;
    test.check(button !== null && button.visible && button.enabled, "no " + name + " button for " + path);
    return button;
}

function steps(test) {
    const noModule = () => test.app.systemModel.home_users.length === 0;

    return [
        {
            name: "impermanence not used",
            skip: () => !noModule(),
            run: () => test.app.showSection("impermanence"),
            timeout: 300000,
            until: () => model(test).loaded && !model(test).loading,
            then: () => {
                test.check(!model(test).available, "impermanence found in a flake without it");
                test.check(test.findChild(page(test), "impermanenceMissing").visible, "no hint that the module is missing");
                test.shot("12z-impermanence-missing");
            },
            delay: 500
        },
        {
            name: "impermanence page",
            skip: noModule,
            run: () => test.app.showSection("impermanence"),
            timeout: 300000,
            until: () => model(test).loaded && !model(test).loading,
            then: () => {
                test.check(model(test).available, "impermanence not found: " + model(test).error);
                test.check(model(test).storage === "/persist", "storage is " + model(test).storage);
                const log = persisted(test, "/var/log");
                test.check(log && log.kind === "dir" && log.locked, "/var/log not listed as set outside the generated file");
                test.check(log.source === "configuration.nix", "/var/log source is " + log.source);
                const id = persisted(test, "/etc/machine-id");
                test.check(id && id.kind === "file", "/etc/machine-id not listed as a file");
                test.check(!persisted(test, "/home/alice/Documents"), "a Home Manager entry shown for the system");
            },
            delay: 300
        },
        {
            name: "impermanence scan",
            skip: noModule,
            run: () => model(test).scan(""),
            until: () => model(test).scanned && !model(test).scanning,
            then: () => {
                test.check(model(test).scan_error === "", "scan failed: " + model(test).scan_error);
                test.check(!model(test).compared, "compared without a snapshot");
                open(test, ["/etc", "/var", "/var/lib"]);
                test.check(shown(test, "/etc/hosts"), "/etc/hosts not listed as removed");
                test.check(!shown(test, "/etc/machine-id"), "persisted /etc/machine-id listed as removed");
                test.check(!shown(test, "/var/log"), "persisted /var/log listed as removed");
                test.check(shown(test, "/var/lib/bluetooth"), "/var/lib/bluetooth not listed");
                test.shot("12a-impermanence");
            },
            delay: 500
        },
        {
            name: "impermanence persist a folder",
            skip: noModule,
            run: () => rowButton(test, "ephemeralList", "/var/lib/bluetooth", "persist").clicked(),
            until: () => persisted(test, "/var/lib/bluetooth") !== undefined,
            then: () => {
                test.check(test.app.changeModel.option_state(SYSTEM_DIRS) === "set", "persisting did not stage " + SYSTEM_DIRS);
                test.check(test.app.changeModel.staged_input(SYSTEM_DIRS) === "/var/lib/bluetooth", "staged list is " + test.app.changeModel.staged_input(SYSTEM_DIRS));
                test.check(persisted(test, "/var/lib/bluetooth").state === "add", "the new entry is not marked as staged");
                test.check(!shown(test, "/var/lib/bluetooth"), "a persisted folder is still listed as removed");
                test.shot("12b-impermanence-staged");
            },
            delay: 500
        },
        {
            name: "impermanence locked and unpersist",
            skip: noModule,
            run: () => {
                const locked = model(test).unpersist(SYSTEM_DIRS, "/var/log");
                test.check(locked.input === "/var/lib/bluetooth", "dropping an entry the file does not hold changed the list: " + locked.input);
                rowButton(test, "persistedList", "/var/lib/bluetooth", "unpersist").clicked();
            },
            until: () => persisted(test, "/var/lib/bluetooth") === undefined,
            then: () => {
                test.check(test.app.changeModel.option_state(SYSTEM_DIRS) === "", "the emptied list is still staged");
                test.check(shown(test, "/var/lib/bluetooth"), "the folder did not move back");
            }
        },
        {
            name: "impermanence home manager",
            skip: noModule,
            run: () => test.findChild(page(test), "impermanenceScope").activated(1),
            until: () => model(test).scope === "home",
            then: () => {
                test.check(test.findChild(page(test), "impermanenceScope").currentIndex === 1, "the scope box does not show alice");
                test.check(model(test).home === "/home/alice", "home folder is " + model(test).home);
                const docs = persisted(test, "/home/alice/Documents");
                test.check(docs && docs.scope === "home" && docs.locked, "Documents not listed for alice");
                test.check(!persisted(test, "/var/log"), "system entries shown for alice");
                test.check(shown(test, "/home/alice/.config") && !shown(test, "/home/alice/Documents"), "home folder listing is wrong");
                page(test).stage(model(test).persist("/home/alice/.config", "dir"));
                test.check(test.app.changeModel.staged_input(HOME_DIRS) === ".config", "Home Manager list is " + test.app.changeModel.staged_input(HOME_DIRS));
                test.check(!shown(test, "/home/alice/.config"), ".config still listed as removed");
                test.shot("12c-impermanence-home");
            },
            delay: 500
        },
        {
            name: "impermanence home manager unstaged",
            skip: noModule,
            run: () => test.app.changeModel.unstage("option:" + HOME_DIRS),
            until: () => persisted(test, "/home/alice/.config") === undefined,
            then: () => {
                test.findChild(page(test), "impermanenceScope").activated(0);
                test.check(model(test).scope === "system", "the scope did not go back to the system");
            }
        },
        {
            name: "impermanence blank snapshot",
            skip: noModule,
            run: () => {
                test.check(model(test).snapshot_offered === true, "the blank snapshot is not offered");
                model(test).scan("root-blank");
            },
            until: () => !model(test).scanning && model(test).compared,
            then: () => {
                open(test, ["/etc"]);
                test.check(!shown(test, "/etc/hosts"), "a file of the blank snapshot is listed as removed");
                test.check(shown(test, "/etc/NetworkManager"), "a folder missing from the snapshot is not listed");
            },
            delay: 300
        }
    ];
}
