.pragma library
.import "../main/kinds.js" as Kinds


function steps(test) {
    return [
        {
            name: "settings: host name evaluated",
            run: () => test.app.optionModel.request_value("networking.hostName"),
            timeout: 300000,
            until: () => test.valueSettled("networking.hostName"),
            then: () => test.check(typeof test.details("networking.hostName").valueInput === "string" && test.details("networking.hostName").valueInput.length > 0, "networking.hostName did not evaluate")
        },
        {
            name: "settings: stage nested, ordered, expression and host-scoped",
            run: () => {
                const c = test.app.changeModel;
                const host = test.details("networking.hostName").valueInput;
                const portType = test.details("services.openssh.ports").type;
                const keysType = test.details("services.openssh.authorizedKeysFiles").type;
                test.check(c.stage_setting("services.openssh.ports", portType, "2222", {
                    priority: "before"
                }) === "", "list with Add first not staged");
                test.check(c.stage_setting("services.openssh.ports", portType, "2222", {
                    priority: "default"
                }) !== "", "scalar priority accepted for a list");
                test.check(c.stage_setting("services.openssh.authorizedKeysFiles", keysType, "1); evil = (2", {
                    mode: "expression"
                }) !== "", "expression broke out of its parentheses");
                test.check(c.stage_setting("services.openssh.authorizedKeysFiles", keysType, "[ \"/etc/ssh/keys/${config.networking.hostName}\" ]", {
                    mode: "expression"
                }) === "", "expression not staged");
                test.check(!c.host_scope_allowed("networking.hostName"), "host scope offered for networking.hostName");
                test.check(c.stage_setting("networking.hostName", "string", "selftest-host", {
                    host: host
                }) !== "", "networking.hostName limited to its own host");
                test.check(c.stage_setting("services.openssh.enable", "boolean", "false", {
                    priority: "force",
                    host: host
                }) === "", "host-scoped setting not staged");
                test.check(c.staged_settings("services.openssh.enable").host === host, "host scope not kept");
                test.app.showOption("services.openssh.authorizedKeysFiles");
            },
            timeout: 300000,
            until: () => test.app.changeModel.check_status("services.openssh.authorizedKeysFiles") !== Kinds.checkPending,
            then: () => {
                const status = test.app.changeModel.check_status("services.openssh.authorizedKeysFiles");
                test.log("expression check: " + status + " " + test.app.changeModel.check_message("services.openssh.authorizedKeysFiles"));
                test.check(status === Kinds.checkOk, "the expression does not type-check");
            },
            delay: 800
        },
        {
            name: "screenshot expression",
            run: () => {
                test.check(test.findChild(test.app.pageStack.currentItem, "expressionField") !== null, "expression editor missing");
                test.shot("09d-details-expression");
            },
            delay: 800
        },
        {
            name: "settings: show priority",
            run: () => test.app.showOption("services.openssh.ports"),
            delay: 800
        },
        {
            name: "screenshot priority",
            run: () => {
                const box = test.findChild(test.app.pageStack.currentItem, "priorityBox");
                test.check(box !== null && box.displayText === qsTr("Add first"), "priority box shows " + (box ? box.displayText : "nothing"));
                test.shot("09e-details-priority");
            },
            delay: 800
        },
        {
            name: "settings: show host scope",
            run: () => test.app.showOption("services.openssh.enable"),
            delay: 800
        },
        {
            name: "screenshot host scope",
            run: () => {
                const box = test.findChild(test.app.pageStack.currentItem, "scopeBox");
                test.check(box !== null && box.currentIndex === 1, "host scope not shown");
                test.shot("09f-details-host");
            },
            delay: 800
        },
        {
            name: "settings: changes page",
            run: () => test.app.showSection("changes"),
            delay: 800
        },
        {
            name: "settings: preview",
            run: () => {
                test.shot("09g-changes-settings");
                const host = test.details("networking.hostName").valueInput;
                const text = test.app.changeModel.preview(test.app.changeModel.plan_json());
                test.log("settings preview:\n" + text);
                for (const expected of ["services.openssh = {", "ports = lib.mkBefore [ 2222 ];", "authorizedKeysFiles = ([ \"/etc/ssh/keys/${config.networking.hostName}\" ]);", "(lib.mkIf (config.networking.hostName == \"" + host + "\") {", "services.openssh.enable = lib.mkForce false;"])
                    test.check(text.indexOf(expected) >= 0, "preview misses: " + expected);
            },
            delay: 800
        },
        {
            name: "settings: preview page",
            run: () => test.app.showPreview(),
            delay: 800
        },
        {
            name: "screenshot settings preview",
            run: () => test.shot("09h-preview-settings"),
            delay: 500
        },
        {
            name: "settings: unstage the extras",
            run: () => {
                const c = test.app.changeModel;
                c.unstage("option:services.openssh.ports");
                c.unstage("option:services.openssh.authorizedKeysFiles");
                test.check(c.option_state("services.openssh.ports") === "", "ports still staged");
                test.app.showSection("options");
            },
            delay: 500
        },
        {
            name: "theme back to automatic",
            run: () => test.app.settingsModel.set_highlight_theme("")
        }
    ];
}
