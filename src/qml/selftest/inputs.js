.pragma library
.import "../main/kinds.js" as Kinds


function steps(test) {
    return [
        {
            name: "inputs page",
            run: () => test.app.showSection("inputs"),
            timeout: 120000,
            until: () => test.app.inputsModel.inputs.length > 0 && !test.app.inputsModel.checking,
            then: () => {
                const inputs = test.app.inputsModel.inputs;
                test.log("inputs: " + inputs.map(input => input.name + "=" + (input.check ? input.check.state : "none")).join(", "));
                test.check(inputs.some(input => input.name === "nixpkgs"), "nixpkgs input missing");
                test.check(inputs.every(input => !!input.check || !!input.follows), "an input was not checked");
                if (test.offline) {
                    const checked = inputs.filter(input => !input.follows);
                    test.check(checked.length > 0, "no inputs checked while offline");
                    test.check(checked.every(input => input.check.state === "failed" || input.check.state === "not-checkable"), "offline checks are not failures: " + JSON.stringify(checked.map(input => input.check)));
                    for (const input of checked) {
                        if (input.check.state === "failed")
                            test.check(input.check.error !== "", "failed check without an error");
                        else
                            test.check(input.check.reason !== "", "unpinned check without a reason");
                    }
                }
                test.check(test.app.systemModel.update_lock(["--commit-lock-file"]) !== "", "invalid input name accepted");
            },
            delay: 500
        },
        {
            name: "highlighting follows the desktop theme",
            run: () => {
                test.check(test.app.matchingHighlightTheme("Catppuccin-Mocha-Mauve") === "Catppuccin Mocha", "Catppuccin global theme not matched");
                test.check(test.app.matchingHighlightTheme("org.kde.breezedark.desktop") === "Breeze Dark", "Breeze Dark not matched");
                test.check(test.app.matchingHighlightTheme("org.kde.breeze.desktop") === "", "plain Breeze should fall back to brightness");
                const inputs = test.app.pageStack.items[0];
                if (inputs.toggle)
                    inputs.toggle("home-manager");
            },
            delay: 500
        },
        {
            name: "screenshot inputs",
            run: () => test.shot("09e-inputs")
        },
        {
            name: "update an input (Home Manager flake only)",
            skip: () => test.app.systemModel.home_users.length === 0 || test.offline,
            run: () => test.check(test.app.systemModel.update_lock(["home-manager"]) === "", "update did not start"),
            timeout: 300000,
            until: () => !test.app.systemModel.running && test.app.systemModel.last_result !== "",
            then: () => test.check(test.app.systemModel.last_result === Kinds.outcomeOk, "nix flake update failed"),
            delay: 1000
        },
        {
            name: "lock re-read after update",
            skip: () => test.app.systemModel.home_users.length === 0 || test.offline,
            timeout: 120000,
            until: () => !test.app.inputsModel.checking && test.app.inputsModel.inputs.some(input => input.check && input.check.state === "up-to-date"),
            then: () => test.shot("09f-inputs-updated")
        },
        {
            name: "console offers rebuild after update",
            skip: () => test.app.systemModel.home_users.length === 0 || test.offline,
            run: () => {
                test.app.showSection("inputs");
                test.app.pageStack.currentItem.update(["home-manager"]);
            },
            timeout: 300000,
            until: () => !test.app.systemModel.running && test.app.systemModel.last_result !== "",
            then: () => {
                test.check(test.app.systemModel.last_result === Kinds.outcomeOk, "page-driven update failed");
                test.app.showSection("log");
                const log = test.app.pageStack.currentItem;
                const modes = test.app.systemModel.capabilities.rebuild;
                test.check(log.updateRebuildOffered === (modes.includes("switch") || modes.includes("build")), "console rebuild offer misses the capabilities");
            },
            delay: 1000
        }
    ];
}

function quit(test) {
    const guard = () => test.app.quitGuard;
    return [
        {
            name: "closing while idle asks nothing",
            run: () => {
                test.check(!test.app.systemModel.running, "a run is still active");
                test.check(!guard().dialog.visible, "the quit dialog is open while idle");
            }
        },
        {
            name: "closing during a run asks first",
            run: () => {
                guard().busy = true;
                test.app.close();
            },
            until: () => guard().dialog.visible,
            then: () => test.check(test.app.visible, "the window closed during a run"),
            delay: 600
        },
        {
            name: "screenshot quit dialog",
            run: () => test.shot("09g-quit-dialog")
        },
        {
            name: "keep running keeps the window",
            run: () => guard().keepRunning(),
            until: () => !guard().dialog.visible,
            then: () => {
                test.check(test.app.visible, "Keep running closed the window");
                test.check(!guard().quitting, "Keep running started quitting");
                guard().busy = Qt.binding(() => test.app.systemModel.running);
                test.check(!guard().busy, "the guard no longer follows the run");
            }
        }
    ];
}
