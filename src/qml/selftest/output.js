.pragma library
.import "../main/kinds.js" as Kinds


function steps(test) {
    return [
        {
            name: "bad target",
            run: () => {
                test.stagedBefore = test.app.changeModel.count;
                test.app.systemModel.retarget("/nonexistent-flake", "", "");
            },
            timeout: 120000,
            until: () => test.app.systemModel.target_error.length > 0
        },
        {
            name: "screenshot bad target",
            run: () => {
                test.app.showSection("options");
                test.log("target error: " + test.app.systemModel.target_error);
            },
            delay: 800
        },
        {
            name: "screenshot bad target 2",
            run: () => test.shot("14-bad-target")
        },
        {
            name: "custom output folder",
            run: () => test.app.systemModel.retarget(test.flake, test.host, test.flake + "/modules/"),
            timeout: 600000,
            until: () => test.app.systemModel.target_ready && test.app.changeModel.ready && test.app.systemModel.warnings.some(warning => warning.id === "not-imported")
        },
        {
            name: "save to custom output",
            run: () => {
                test.check(test.app.systemModel.managed_path === test.flake + "/modules/default.nix", "managed path " + test.app.systemModel.managed_path);
                test.check(test.app.setupModel.import_line() === "imports = [ ./modules ];", "import line " + test.app.setupModel.import_line());
                test.check(test.app.systemModel.managed_setting === test.flake + "/modules/", "setting not kept");
                test.check(test.app.changeModel.managed_count === 0, "new output should start empty");
                test.log("staged before switching away: " + test.stagedBefore + ", after coming back: " + test.app.changeModel.count);
                test.check(test.app.changeModel.count === test.stagedBefore, "staged edits were not restored");
                test.app.changeModel.clear();
                test.check(test.app.changeModel.stage_option("services.openssh.enable", "boolean", "true") === "", "stage failed");
                const preview = test.app.changeModel.preview(test.app.changeModel.plan_json());
                test.check(preview.indexOf("/modules/default.nix") >= 0 && preview.indexOf("    ./services/openssh.nix") >= 0, "parts are not beside default.nix:\n" + preview.substring(0, 800));
                test.app.startApply("save");
            },
            timeout: 120000,
            until: () => !test.app.systemModel.running && test.app.systemModel.last_result !== ""
        },
        {
            name: "custom output saved",
            run: () => {
                test.check(test.app.systemModel.last_result === Kinds.outcomeOk, "save failed:\n" + test.app.consoleModel.log_text());
                test.app.showSection("settings");
            },
            timeout: 10000,
            until: () => test.app.changeModel.managed_count === 1,
            delay: 800
        },
        {
            name: "screenshot custom output",
            run: () => test.shot("15-custom-output")
        },
        {
            name: "missing import reported",
            run: () => test.app.setupModel.check_import(),
            timeout: 120000,
            until: () => {
                const status = test.app.setupModel.import_status;
                return status !== Kinds.importChecking && status !== Kinds.importIdle;
            },
            then: () => {
                test.check(test.app.setupModel.import_status === Kinds.importMissing, "unimported folder says " + test.app.setupModel.import_status);
                test.check(test.app.setupModel.import_error === "", "missing import has an error: " + test.app.setupModel.import_error);
            }
        },
        {
            name: "retarget back",
            run: () => test.app.systemModel.retarget(test.flake, test.host, ""),
            timeout: 600000,
            until: () => test.app.systemModel.target_ready && test.app.optionModel.size > 0 && test.app.changeModel.ready && test.app.changeModel.managed_count === (test.applyMode === "" ? 0 : test.savedCount)
        },
        {
            name: "remove a managed package and restore it",
            skip: () => test.applyMode === "",
            run: () => {
                const c = test.app.changeModel
                const staged = c.count
                const managed = c.managed_count
                test.check(c.package_state("hello") === "managed", "hello is " + c.package_state("hello"))
                test.check(c.stage_package("hello", false) === "", "managed removal refused")
                test.check(c.package_state("hello") === "remove", "hello not removed")
                c.unstage("package:hello")
                test.check(c.package_state("hello") === "managed", "hello not managed again")
                test.check(c.managed_count === managed, "managed count changed")
                test.check(c.count === staged, "staged count changed")
            }
        }
    ];
}
