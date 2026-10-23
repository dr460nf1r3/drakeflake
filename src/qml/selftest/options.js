.pragma library
.import "../main/kinds.js" as Kinds

function basics(test) {
    return [
        {
            name: "target resolved",
            timeout: 600000,
            until: () => test.app.systemModel.target_ready || test.app.systemModel.target_error.length > 0,
            then: () => {
                if (test.app.systemModel.target_error.length > 0) {
                    test.shot("00-target-error");
                    test.fail("target error: " + test.app.systemModel.target_error);
                }
                test.flake = test.app.systemModel.flake;
                test.host = test.app.systemModel.host;
                test.log("target " + test.flake + "#" + test.host);
            }
        },
        {
            name: "option index loaded",
            timeout: 600000,
            until: () => test.app.optionModel.size > 0,
            then: () => {
                test.log("options: " + test.app.optionModel.size);
                test.shot("01-options");
            }
        },
        {
            name: "search openssh",
            run: () => {
                test.app.showSection("options");
                test.app.optionQuery = "openssh";
                test.app.optionModel.search("openssh");
                test.check(test.app.optionModel.total > 0, "no results for openssh");
            },
            delay: 800
        },
        {
            name: "screenshot search",
            run: () => test.shot("02-search")
        },
        {
            name: "details bool",
            run: () => test.app.showOption("services.openssh.enable"),
            timeout: 300000,
            until: () => test.valueSettled("services.openssh.enable")
        },
        {
            name: "screenshot bool",
            run: () => {
                const info = test.details("services.openssh.enable");
                test.check(info.kind === Kinds.editorBool, "openssh.enable kind " + info.kind);
                test.log("openssh.enable = " + info.valueDisplay + " (" + info.valueState + ")");
                test.shot("03-details-bool");
            },
            delay: 500
        },
        {
            name: "details list",
            run: () => test.app.showOption("services.openssh.ports"),
            timeout: 300000,
            until: () => test.valueSettled("services.openssh.ports")
        },
        {
            name: "screenshot list",
            run: () => {
                test.check(test.details("services.openssh.ports").kind === Kinds.editorList, "ports is not a list editor");
                test.shot("04-details-list");
            },
            delay: 500
        },
        {
            name: "details enum",
            run: () => test.app.showOption("services.openssh.settings.PermitRootLogin"),
            delay: 1500
        },
        {
            name: "screenshot enum",
            run: () => {
                const info = test.details("services.openssh.settings.PermitRootLogin");
                test.log("PermitRootLogin kind=" + info.kind + " choices=" + JSON.stringify(info.choices));
                test.shot("05-details-enum");
            }
        },
        {
            name: "stage option changes",
            run: () => {
                const c = test.app.changeModel;
                test.check(c.stage_option("networking.hostName", "string", "selftest-host") === "", "string stage failed");
                test.check(c.stage_option("services.openssh.enable", "boolean", "false") === "", "bool stage failed");
                const portType = test.details("services.openssh.ports").type;
                test.check(c.stage_option("services.openssh.ports", portType, "22 nope") !== "", "invalid list accepted");
                test.check(c.stage_option("services.openssh.ports", portType, "70000") !== "", "out-of-range port accepted");
                test.check(c.validate("boolean", "maybe") !== "", "bad bool validated");
                test.check(c.stage_option("", "boolean", "true") !== "", "empty name accepted");
                test.check(c.count === 2, "expected 2 staged, got " + c.count);
                test.app.showOption("networking.hostName");
            },
            delay: 1500
        },
        {
            name: "type check rejects bad hostname",
            run: () => {
                test.check(test.app.changeModel.stage_option("networking.hostName", test.details("networking.hostName").type, "bad_host!") === "", "syntactically fine value was rejected before the type check");
            },
            timeout: 300000,
            until: () => test.app.changeModel.check_status("networking.hostName") === "invalid",
            then: () => {
                test.log("hostname check: " + test.app.changeModel.check_message("networking.hostName"));
                test.check(test.app.changeModel.checks_failed > 0, "failed check does not block applying");
            },
            delay: 800
        },
        {
            name: "screenshot type error",
            run: () => test.shot("06b-type-error")
        },
        {
            name: "type check accepts good hostname",
            run: () => test.app.changeModel.stage_option("networking.hostName", test.details("networking.hostName").type, "selftest-host"),
            timeout: 300000,
            until: () => test.app.changeModel.check_status("networking.hostName") === Kinds.checkOk && test.app.changeModel.checks_pending === 0,
            then: () => test.check(test.app.changeModel.checks_failed === 0, "checks still failing: " + test.app.changeModel.checks_failed),
            delay: 500
        },
        {
            name: "screenshot staged string",
            run: () => test.shot("06-details-string-staged")
        }
    ];
}

function values(test) {
    return [
        {
            name: "where is this set",
            timeout: 600000,
            until: () => test.app.optionModel.values_ready,
            then: () => {
                const locations = test.details("services.openssh.enable").locations || [];
                test.log("services.openssh.enable set in: " + JSON.stringify(locations));

                const line = test.app.systemModel.selftest("suite") === "hm" ? 14 : 18;
                test.check(locations.some(location => location.local && location.display === "configuration.nix" && location.line === line), "definition not found at configuration.nix:" + line);
            }
        },
        {
            name: "undefined option evaluated on demand",
            run: () => {
                const state = test.details("services.printing.enable").valueState;
                test.check(state === Kinds.valueLoading || state === Kinds.valueUnknown, "the snapshot holds a value of an option no user file defines (" + state + ")");
                test.app.showOption("services.printing.enable");
            },
            timeout: 300000,
            until: () => test.valueSettled("services.printing.enable"),
            then: () => {
                const info = test.details("services.printing.enable");
                test.log("printing.enable on demand = " + info.valueDisplay + " (" + info.valueState + ")");
                test.check(info.valueState === Kinds.valueReady && info.valueDisplay === "false", "value not evaluated on demand");
            }
        }
    ];
}

function tree(test) {
    return [
        {
            name: "tree waits for its page",
            run: () => {
                test.check(test.app.optionModel.size > 0, "no option index loaded yet");
                test.check(test.app.treeModel.rowCount() === 0, "the tree was built before its page opened: " + test.app.treeModel.rowCount() + " rows");
            }
        },
        {
            name: "tree browser",
            run: () => {
                test.app.showSection("tree");
                test.check(test.app.treeModel.tree_building, "the tree was not built in the background");
            },
            timeout: 600000,
            until: () => {
                const list = test.findChild(test.app.pageStack.items[0], "optionTree");
                return test.app.optionModel.size > 0 && list !== null && list.count > 1000;
            },
            then: () => {
                const list = test.findChild(test.app.pageStack.items[0], "optionTree");
                test.log("tree rows: " + list.count);
                test.check(test.app.treeModel.all_open, "tree does not start open");
                test.check(test.app.homeTreeModel.rowCount() === 0, "opening the system tree also built the Home Manager tree");
            },
            delay: 500
        },
        {
            name: "tree collapse and expand",
            run: () => {
                const list = test.findChild(test.app.pageStack.items[0], "optionTree");
                const full = list.count;
                const started = Date.now();
                test.app.treeModel.collapse_all();
                const collapsed = list.count;
                test.app.treeModel.toggle_folder("services");
                const servicesOpen = list.count;
                test.app.treeModel.expand_all();
                test.log("tree: " + full + " rows, collapsed " + collapsed + ", services open " + servicesOpen + ", " + (Date.now() - started) + " ms");
                test.check(collapsed > 0 && collapsed < 200, "collapse all left " + collapsed + " rows");
                test.check(servicesOpen > collapsed, "opening services added no rows");
                test.check(list.count === full && test.app.treeModel.all_open, "expand all did not restore the tree");
            }
        },
        {
            name: "screenshot tree",
            run: () => test.shot("07-tree")
        },
        {
            name: "tree leaf opens details",
            run: () => test.app.showOption("services.openssh.enable"),
            timeout: 300000,
            until: () => test.valueSettled("services.openssh.enable"),
            then: () => test.check(test.details("services.openssh.enable").kind === Kinds.editorBool, "tree leaf details wrong")
        },
        {
            name: "tree closed",
            run: () => test.app.showSection("options"),
            delay: 1500
        }
    ];
}

function stress(test) {
    return [
        {
            name: "stress searches",
            run: () => {
                test.app.showSection("options");
                const queries = ["", "a", "e", "zzzz", "services.", "boot loader", "\"", "\\", "<name>", "ñ", "x".repeat(500)];
                for (let i = 0; i < 40; i++) {
                    test.app.optionModel.search(queries[i % queries.length]);
                    test.app.optionModel.load_more();
                    test.app.optionModel.set_namespace(i % 3 === 0 ? "services" : "");
                    test.app.optionModel.set_only_user(i % 2 === 0);
                    test.app.optionModel.set_show_templated(i % 5 === 0);
                    test.app.packageModel.search(queries[i % queries.length]);
                    test.app.packageModel.load_more();
                }
                test.app.optionModel.set_namespace("");
                test.app.optionModel.set_only_user(false);
                test.app.optionModel.set_show_templated(false);
                test.app.optionModel.search("firewall");
                test.app.optionModel.request_value("does.not.exist");
                test.app.optionModel.request_value("");
                test.check(Object.keys(test.app.optionModel.details("does.not.exist")).length === 0, "unknown option has details");
            },
            delay: 1000
        },
        {
            name: "narrow window",
            run: () => {
                test.app.width = 520;
                test.app.showOption("networking.firewall.allowedTCPPorts");
            },
            delay: 1500
        },
        {
            name: "screenshot narrow",
            run: () => test.shot("11-narrow")
        },
        {
            name: "restore width",
            run: () => {
                test.restoreWindowSize();
                test.app.showSection("options");
            },
            delay: 500
        }
    ];
}
