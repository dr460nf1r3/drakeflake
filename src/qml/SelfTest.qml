import QtQuick
import org.kde.kirigami as Kirigami
import "selftest/options.js" as Options
import "selftest/missing.js" as Missing
import "selftest/darwin.js" as Darwin
import "selftest/packages.js" as Packages
import "selftest/changes.js" as Changes
import "selftest/home.js" as Home
import "selftest/impermanence.js" as Impermanence
import "selftest/inputs.js" as Inputs
import "selftest/maintenance.js" as Maintenance
import "selftest/settings.js" as Settings
import "selftest/menu.js" as Menu
import "selftest/window.js" as WindowState
import "selftest/choices.js" as Choices
import "selftest/edge.js" as Edge
import "selftest/setup.js" as Setup
import "selftest/output.js" as Output
import "selftest/overview.js" as Overview
import "selftest/functions.js" as Functions
import "selftest/docs.js" as Docs
import "main/kinds.js" as Kinds

Item {
    id: test

    property var app: null
    property int step: 0
    property double stepStarted: Date.now()
    property double waitUntil: 0
    property string flake: ""
    property string host: ""
    property int stagedBefore: 0
    property string scratchFlake: ""
    property int scratchStaged: 0
    readonly property int savedCount: test.app && test.app.systemModel.home_users.length > 0 ? 4 : 3
    readonly property string dir: test.app ? test.app.systemModel.selftest("dir") : ""
    readonly property string applyMode: test.app ? test.app.systemModel.selftest("apply") : ""
    readonly property bool isDarwin: test.app ? test.app.systemModel.selftest("suite") === "darwin" : false
    readonly property bool offline: test.app ? test.app.systemModel.selftest("offline") === "1" : false
    readonly property bool isMissing: test.app ? test.app.systemModel.selftest("suite") === "missing" : false
    readonly property var steps: [].concat(test.isMissing ? Missing.steps(test) : test.isDarwin ? [].concat(Darwin.steps(test), Overview.flakeSource(test), Edge.darwin(test), Menu.steps(test), Docs.steps(test), WindowState.steps(test), Setup.steps(test), Setup.fromScratch(test)) : [].concat(Options.basics(test), Packages.search(test), Overview.steps(test), Overview.flakeSource(test), Functions.steps(test), Changes.review(test), Home.steps(test), Impermanence.steps(test), Packages.sets(test), Inputs.steps(test), Inputs.quit(test), Options.values(test), Options.tree(test), Maintenance.steps(test), Settings.steps(test), Menu.steps(test), Docs.steps(test), WindowState.steps(test), Choices.steps(test), Edge.steps(test), Setup.steps(test), Options.stress(test), Changes.apply(test), Output.steps(test), Setup.fromScratch(test)), [
        {
            name: "done",
            run: () => {
                test.log("PASS");
                Qt.quit();
            }
        }
    ])
    property bool started: false

    function log(text: string): void {
        console.warn("SELFTEST " + text);
    }

    function fail(text: string): void {
        console.error("SELFTEST FAIL at step " + test.step + " (" + test.steps[test.step].name + "): " + text);
        console.error("SELFTEST state: options size=" + test.app.optionModel.size + " total=" + test.app.optionModel.total + " packages size=" + test.app.packageModel.size + " total=" + test.app.packageModel.total + " error=" + test.app.packageModel.error + " activity=" + test.app.systemModel.activity + "\nlog:\n" + test.app.consoleModel.log_text());
        Qt.exit(1);
    }

    function themeColor(role: string): string {
        return Kirigami.Theme[role].toString();
    }

    function check(condition: bool, text: string): bool {
        if (!condition)
            test.fail(text);
        return condition;
    }

    function checkCommitted(): void {
        const log = test.app.consoleModel.log_text();
        test.check(log.indexOf("Created a git repository") >= 0, "no git repository created for the fixture");
        test.check(log.indexOf("Committed the changes") >= 0, "apply did not commit");
    }

    function shot(name: string): void {
        let top = test.app.contentItem;
        while (top.parent)
            top = top.parent;

        const bar = test.app.menuBar;
        bar.hiddenForShot = test.app.systemModel.selftest("theme") !== "" && !name.includes("menu");

        // One image pixel per point, so a Retina display gives the same size as any other.
        top.grabToImage(result => {
            bar.hiddenForShot = false;
            result.saveToFile(test.dir + "/" + name + ".png");
            test.log("screenshot " + name);
        });
    }

    function restoreWindowSize(): void {
        test.app.width = 1600;
        test.app.height = 900;
    }

    function details(name: string): var {
        return test.app.optionModel.details(name);
    }

    // Start from an item: a window exposes no children list.
    function findChild(item: var, name: string): var {
        if (!item)
            return null;

        if (item.objectName === name)
            return item;

        for (const child of item.children || []) {
            const found = test.findChild(child, name);

            if (found)
                return found;
        }

        return null;
    }

    function valueSettled(name: string): bool {
        const state = test.details(name).valueState;
        return state === Kinds.valueReady || state === Kinds.valueError;
    }

    Timer {
        interval: 200
        repeat: true
        running: test.app !== null && test.dir !== ""

        onTriggered: {
            const current = test.steps[test.step];

            if (!current) {
                Qt.quit();
                return;
            }

            if (Date.now() < test.waitUntil)
                return;

            if (!test.started) {
                if (current.skip && current.skip()) {
                    test.step++;
                    return;
                }

                test.log("step " + test.step + ": " + current.name);
                test.started = true;
                test.stepStarted = Date.now();

                if (current.run) {
                    try {
                        current.run();
                    } catch (error) {
                        test.fail("threw: " + error);
                    }
                }
            }

            if (current.until && !current.until()) {
                if (Date.now() - test.stepStarted > (current.timeout || 60000)) {
                    test.shot("timeout-step-" + test.step);
                    test.fail("timed out waiting");
                }
                return;
            }

            if (current.then) {
                // A throw would repeat the step until the script's timeout.
                try {
                    current.then();
                } catch (error) {
                    test.fail("threw: " + error);
                }
            }

            test.started = false;
            test.step++;
            test.waitUntil = Date.now() + (current.delay || 0);
        }
    }
}
