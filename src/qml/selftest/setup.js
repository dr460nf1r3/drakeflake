.pragma library
.import "../main/kinds.js" as Kinds


function steps(test) {
    return [
        {
            name: "setup assistant",
            run: () => test.app.setupOpen = true,
            delay: 800
        },
        {
            name: "setup: flakes found in the background",
            timeout: 120000,
            until: () => !test.app.setupModel.flake_scanning,
            then: () => {
                const found = test.app.setupModel.flake_candidates;
                test.log("system flakes found: " + found.length);
                test.check(Array.isArray(found), "flake candidates are not a list");
            }
        },
        {
            name: "screenshot setup requirements",
            run: () => test.shot("10d-setup-requirements"),
            delay: 300
        },
        {
            name: "setup: use the flake",
            run: () => {
                const setup = test.app.setupItem;
                setup.step = 1;
                setup.flakePath = test.app.systemModel.flake;
                test.check(setup.flakeRoot.length > 0, "flake root not found for " + setup.flakePath);
            },
            delay: 500
        },
        {
            name: "screenshot setup flake choice",
            run: () => test.shot("10d2-setup-flake"),
            delay: 300
        },
        {
            name: "setup: load the flake",
            run: () => test.app.setupItem.loadFlake(),
            timeout: 600000,
            until: () => test.app.systemModel.target_ready || test.app.systemModel.target_error.length > 0
        },
        {
            name: "screenshot setup system",
            run: () => {
                test.check(test.app.systemModel.target_error.length === 0, "setup load failed: " + test.app.systemModel.target_error);
                test.check(test.app.setupItem.autoCommit, "automatic commits not preselected in the assistant");
                test.shot("10e-setup-system");
            },
            delay: 500
        },
        {
            name: "setup: next and check the import",
            run: () => {
                const setup = test.app.setupItem;
                test.check(setup.folderExists && setup.folderInFlake, "flake folder not accepted as output folder");
                setup.useSystem();
            },
            timeout: 600000,
            until: () => test.app.systemModel.target_ready
        },
        {
            name: "setup: import check",
            run: () => test.app.setupModel.check_import(),
            timeout: 600000,
            until: () => {
                const status = test.app.setupModel.import_status;
                if (status === Kinds.importChecking || status === Kinds.importIdle)
                    return false;

                const message = test.findChild(test.app.setupItem, "importedMessage");
                return status !== Kinds.importImported || (message !== null && message.opacity === 1 && !message.animating);
            }
        },
        {
            name: "screenshot setup import",
            run: () => {
                const setup = test.app.setupModel;
                console.warn("SELFTEST import line: " + setup.import_line() + ", status " + setup.import_status + " " + setup.import_error);
                test.check(setup.import_status === Kinds.importImported, "fixture imports drakeflake.nix but the check says " + setup.import_status + " " + setup.import_error);
                test.check(setup.import_line() === "imports = [ ./drakeflake.nix ];", "import line: " + setup.import_line());
                test.shot("10f-setup-import");
            },
            delay: 500
        },
        {
            name: "setup: finish",
            run: () => {
                test.app.setupItem.finished();
                test.check(!test.app.setupOpen && test.app.section === "overview" && !test.app.setupModel.needs_setup, "setup did not finish");
            },
            delay: 500
        }
    ];
}

function fromScratch(test) {
    return [
        {
            name: "scratch setup: drop the target",
            run: () => {
                const system = test.app.systemModel;
                test.scratchFlake = system.flake;
                test.scratchStaged = test.app.changeModel.count;
                system.retarget("/nonexistent-flake", "", "");
            },
            timeout: 120000,
            until: () => test.app.systemModel.target_error.length > 0
        },
        {
            name: "scratch setup: open the assistant",
            run: () => {
                test.app.setupOpen = true;
                test.check(test.app.systemModel.target_error.length > 0, "bad target recovered on its own");
            },
            delay: 800
        },
        {
            name: "scratch setup: use the flake",
            run: () => {
                const setup = test.app.setupItem;
                setup.step = 1;
                setup.flakePath = test.scratchFlake;
                test.check(setup.flakeRoot.length > 0, "flake root not found for " + setup.flakePath);
            },
            delay: 500
        },
        {
            name: "scratch setup: load the flake",
            run: () => test.app.setupItem.loadFlake(),
            timeout: 600000,
            until: () => test.app.systemModel.target_ready || test.app.systemModel.target_error.length > 0,
            then: () => test.check(test.app.systemModel.target_error.length === 0, "scratch load failed: " + test.app.systemModel.target_error)
        },
        {
            name: "scratch setup: use the system",
            run: () => {
                test.app.setupItem.useSystem();
                test.check(test.app.setupItem.folderExists && test.app.setupItem.folderInFlake, "flake folder not accepted as output folder");
            },
            timeout: 600000,
            until: () => test.app.systemModel.target_ready
        },
        {
            name: "scratch setup: import check",
            run: () => test.app.setupModel.check_import(),
            timeout: 600000,
            until: () => {
                const status = test.app.setupModel.import_status;
                return status !== Kinds.importChecking && status !== Kinds.importIdle;
            },
            then: () => test.check(test.app.setupModel.import_status === Kinds.importImported, "scratch import says " + test.app.setupModel.import_status + " " + test.app.setupModel.import_error)
        },
        {
            name: "scratch setup: finish",
            run: () => {
                test.app.setupItem.finished();
                test.check(!test.app.setupOpen && test.app.section === "overview" && !test.app.setupModel.needs_setup, "scratch setup did not finish");
                test.check(test.app.changeModel.count === test.scratchStaged, "staged edits lost in the scratch tour");
                test.shot("16-scratch-setup");
            },
            delay: 500
        }
    ];
}
