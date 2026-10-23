.pragma library
.import "../main/kinds.js" as Kinds


function top(test) {
    return test.app.pageStack.items[test.app.pageStack.depth - 1];
}

function review(test) {
    return [
        {
            name: "changes page",
            run: () => test.app.showSection("changes"),
            delay: 800
        },
        {
            name: "preview",
            run: () => {
                test.shot("09-changes");
                const text = test.app.changeModel.preview(test.app.changeModel.plan_json());
                test.log("preview:\n" + text);
                test.check(text.indexOf('networking.hostName = lib.mkOverride 90 "selftest-host";') >= 0, "preview misses hostName");
                test.check(text.indexOf("pkgs.hello") >= 0, "preview misses hello");
                test.check(test.app.changeModel.preview("not json").indexOf("Invalid plan") >= 0, "bad plan not reported");
            }
        },
        {
            name: "preview page",
            run: () => test.app.showPreview(),
            delay: 800
        },
        {
            name: "screenshot preview",
            run: () => test.shot("09b-preview")
        },
        {
            name: "review opens preview",
            run: () => {
                test.app.showSection("changes");
                const actions = test.app.sectionPages.changes.actions;
                const review = actions.find(action => action.objectName === "reviewApplyAction");
                test.check(!!review, "no Review and apply action");
                review.trigger();
            },
            until: () => top(test).isPreview === true,
            then: () => {
                test.check(top(test).pendingMode === "switch", "preview waits for " + top(test).pendingMode);
                test.shot("09d-review");
                test.app.pageStack.pop();
            },
            skip: () => {
                const page = test.app.sectionPages.changes;
                if (!page)
                    return true;
                const review = page.actions.find(action => action.objectName === "reviewApplyAction");
                return !review || !review.enabled;
            },
            delay: 800
        },
        {
            name: "configured view",
            run: () => {
                const before = test.app.optionModel.total;
                test.app.showSection("configured");
                test.app.configuredModel.search("");
                test.check(test.app.optionModel.total === before, "configured search changed the Options view");
            },
            timeout: 900000,
            until: () => test.app.configuredModel.values_ready && test.app.configuredModel.total > 0,
            delay: 800
        },
        {
            name: "screenshot configured",
            run: () => {
                test.log("configured matches: " + test.app.configuredModel.total);
                test.app.configuredModel.search("");
            },
            delay: 600
        },
        {
            name: "screenshot configured 2",
            run: () => test.shot("09c-configured")
        }
    ];
}

function apply(test) {
    return [
        {
            name: "apply",
            skip: () => test.applyMode === "",
            run: () => {
                test.log("applying with mode " + test.applyMode);
                test.app.startApply(test.applyMode);
            },
            timeout: 1800000,
            until: () => !test.app.systemModel.running && test.app.systemModel.last_result !== ""
        },
        {
            name: "split layout written",
            skip: () => test.applyMode === "",
            run: () => {
                test.log("files after save:\n" + test.app.changeModel.preview(test.app.changeModel.plan_json()).split("\n").filter(l => l.startsWith("### ")).join("\n"));
                test.check(test.app.changeModel.preview(test.app.changeModel.plan_json()).indexOf("### drakeflake/services/openssh.nix") >= 0, "default layout does not split by area and module");
                if (test.app.systemModel.home_users.length > 0)
                    test.check(test.app.changeModel.preview(test.app.changeModel.plan_json()).indexOf("### drakeflake/home-manager/alice/programs/git.nix") >= 0, "Home Manager settings do not follow the layout");
            }
        },
        {
            name: "apply result",
            skip: () => test.applyMode === "",
            then: () => {
                test.shot("12-log");
                test.log("log:\n" + test.app.consoleModel.log_text());
                test.check(test.app.systemModel.last_result === Kinds.outcomeOk, "apply failed");
                test.checkCommitted();
            },
            timeout: 10000,
            until: () => test.app.changeModel.count === 0
        },
        {
            name: "managed after apply",
            skip: () => test.applyMode === "",
            run: () => {
                test.check(test.app.changeModel.managed_count === test.savedCount, "managed count " + test.app.changeModel.managed_count);
                test.check(test.app.changeModel.option_state("networking.hostName") === "managed", "hostName not managed");
                test.app.showSection("changes");
            },
            delay: 1000
        },
        {
            name: "values updated after apply",
            skip: () => test.applyMode === "",
            timeout: 600000,
            until: () => test.details("networking.hostName").userFiles.some(file => file.indexOf("drakeflake") >= 0),
            then: () => {
                const info = test.details("networking.hostName");
                test.log("hostName after apply = " + info.valueDisplay + " from " + JSON.stringify(info.userFiles));
                test.check(info.valueDisplay.indexOf("selftest-host") >= 0, "hostName after apply is " + info.valueDisplay);
            }
        },
        {
            name: "screenshot managed",
            skip: () => test.applyMode === "",
            run: () => test.shot("13-managed")
        },
        {
            name: "history after save",
            skip: () => test.applyMode === "",
            run: () => {
                test.app.showSection("history");
                test.app.generationsModel.load_history();
            },
            until: () => !test.app.generationsModel.history_loading,
            then: () => {
                const entries = test.app.generationsModel.history;
                test.log("history entries: " + entries.length);
            },
            delay: 800
        },
        {
            name: "screenshot history",
            skip: () => test.applyMode === "",
            run: () => test.shot("13b-history")
        }
    ];
}
