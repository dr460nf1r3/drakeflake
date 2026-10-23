.pragma library
.import "../main/helpers.js" as Helpers

function stagedCount(test) {
    return test.app.changeModel.count
}

function checkCountRestored(test, before, what) {
    test.check(test.app.changeModel.count === before, what + " changed the staged count to " + test.app.changeModel.count)
}

function universal(test) {
    return [
        {
            name: "edge: selftest contract",
            run: () => {
                const system = test.app.systemModel
                test.check(system.selftest("dir") !== "", "selftest dir is empty")
                test.check(system.selftest("apply") !== "", "selftest apply mode is empty")
                test.check(system.selftest("suite") === (test.isDarwin ? "darwin" : "plain") || (!test.isDarwin && system.selftest("suite") === "hm"), "selftest suite is " + system.selftest("suite"))
                test.check(system.selftest("bogus-key") === "", "unknown selftest key is not empty")
            }
        },
        {
            name: "edge: api version and capabilities",
            run: () => {
                test.check(test.app.systemModel.api_version() === test.app.expectedApi, "api " + test.app.systemModel.api_version() + " against QML " + test.app.expectedApi)
                const can = test.app.systemModel.capabilities
                test.check(can.platform === "nixos" || can.platform === "darwin", "platform is " + can.platform)
                test.check(typeof can.native === "boolean", "native is not a bool")
                test.check(typeof can.generations === "boolean", "generations is not a bool")
                test.check(Array.isArray(can.apply) && can.apply.indexOf("save") >= 0, "save not offered: " + JSON.stringify(can.apply))
                test.check(Array.isArray(test.app.systemModel.home_users), "home users are not a list")
                test.check(Array.isArray(test.app.systemModel.warnings), "warnings are not a list")
            }
        },
        {
            name: "edge: unknown section falls back",
            run: () => {
                const back = test.app.section
                test.app.showSection("no-such-section")
                test.check(test.app.section === "overview", "unknown section shows " + test.app.section)
                test.app.showSection(back)
                test.check(test.app.section === back, "did not return to " + back)
            }
        },
        {
            name: "edge: offscreen stays solid",
            skip: () => Qt.platform.pluginName !== "offscreen",
            run: () => {
                test.check(!test.app.vibrancy, "vibrancy without the blur")
                test.check(!test.app.unifiedTitlebar, "unified titlebar without the blur")
                test.check(test.app.color.a === 1, "see-through window without the blur")
            }
        },
        {
            name: "edge: validate matrix",
            run: () => {
                const c = test.app.changeModel
                const portType = test.details("services.openssh.ports").type
                for (const good of ["true", "false", "yes", "0", " TRUE "])
                    test.check(c.validate("boolean", good) === "", "bool rejected " + good)
                for (const bad of ["maybe", "", "truthy"])
                    test.check(c.validate("boolean", bad) !== "", "bool accepted " + bad)
                test.check(c.validate("string", "") === "", "empty string rejected")
                test.check(c.validate("string", "hello world") === "", "string rejected")
                if (portType) {
                    test.check(c.validate(portType, "22") === "", "port 22 rejected")
                    test.check(c.validate(portType, " 22 ") === "", "port with blanks rejected")
                    for (const bad of ["ssh", "99999", "22 nope"])
                        test.check(c.validate(portType, bad) !== "", "port accepted " + bad)
                } else {
                    test.log("services.openssh.ports not defined here; skipping port asserts")
                }
            }
        },
        {
            name: "edge: expression validation is local",
            run: () => {
                const c = test.app.changeModel
                test.check(c.validate_expression("[ 1 2 ]") === "", "list expression rejected")
                test.check(c.validate_expression("{ a = 1; }") === "", "attrset expression rejected")
                test.check(c.validate_expression("[ 1") !== "", "broken expression accepted")
                test.check(c.validate_expression("1); evil = (2") !== "", "escaping expression accepted")
            }
        },
        {
            name: "edge: priority choices",
            run: () => {
                const c = test.app.changeModel
                const portType = test.details("services.openssh.ports").type
                const scalar = c.priority_choices("boolean", "true", "typed")
                test.check(scalar.implicit === "override" && scalar.choices[0] === "override", "scalar priorities: " + JSON.stringify(scalar))
                test.check(scalar.choices.indexOf("force") >= 0 && scalar.choices.length === 4, "scalar choices: " + JSON.stringify(scalar.choices))
                if (portType) {
                    const list = c.priority_choices(portType, "22", "typed")
                    test.check(list.implicit === "merge" && list.choices[0] === "merge", "list priorities: " + JSON.stringify(list))
                    test.check(list.choices.indexOf("before") >= 0 && list.choices.length === 4, "list choices: " + JSON.stringify(list.choices))
                } else {
                    test.log("services.openssh.ports not defined here; skipping list asserts")
                }
                const unknownMode = c.priority_choices("boolean", "true", "bogus-mode")
                test.check(unknownMode.implicit === "override", "unknown mode is not typed: " + JSON.stringify(unknownMode))
            }
        },
        {
            name: "edge: unknown staged lookups",
            run: () => {
                const c = test.app.changeModel
                test.check(c.check_status("does.not.exist") === "", "unknown check has a status")
                test.check(c.check_message("does.not.exist") === "", "unknown check has a message")
                test.check(c.option_state("does.not.exist") === "", "unknown option has a state")
                test.check(c.staged_input("does.not.exist") === "", "unknown option has staged input")
                test.check(Object.keys(c.staged_settings("does.not.exist")).length === 0, "unknown option has staged settings")
                test.check(c.package_state("does-not-exist") === "", "unknown package has a state")
            }
        },
        {
            name: "edge: unstage garbage",
            run: () => {
                const before = stagedCount(test)
                const c = test.app.changeModel
                c.unstage("")
                c.unstage("bogus")
                c.unstage("option:")
                c.unstage("package:")
                c.unstage("option:does.not.exist")
                c.unstage("package:does-not-exist")
                checkCountRestored(test, before, "unstaging garbage")
            }
        },
        {
            name: "edge: reset of unmanaged options",
            run: () => {
                const before = stagedCount(test)
                const c = test.app.changeModel
                c.stage_reset("services.printing.enable")
                c.stage_reset("")
                checkCountRestored(test, before, "resetting unmanaged options")
                test.check(c.option_state("services.printing.enable") === "", "reset staged an unmanaged option")
            }
        },
        {
            name: "edge: refused runs start nothing",
            run: () => {
                const system = test.app.systemModel
                system.generation_action(-1, "bogus-action")
                test.check(!system.running, "a refused generation action runs")
                const result = system.last_result
                system.apply(test.app.changeModel.plan_json(), "bogus-mode")
                system.review_apply(test.app.changeModel.plan_json(), "bogus-mode")
                test.check(system.last_result === result, "a refused apply changed the result")
                test.check(system.rebuild("bogus-mode") !== "", "a bogus rebuild mode accepted")
                test.check(system.update_lock("bogus-scalar") !== "", "a bogus input list accepted")
                test.check(!system.can_restore, "a backup exists before anything ran")
                system.restore_backup()
                system.cancel()
                test.check(!system.running, "nothing runs after the refused runs")
                test.check(system.last_result === result, "a refused run changed the result")
            }
        },
        {
            name: "edge: console idle",
            run: () => {
                const console = test.app.consoleModel
                test.check(console.prompt === "" && !console.prompt_password, "a prompt with nothing running")
                console.answer_yes()
                test.check(console.prompt === "", "answering without a prompt asked one")
                console.answer_password("edge-probe")
                test.check(console.prompt === "", "a password answer without a prompt asked one")
                console.clear_log()
                test.check(console.line_count === 0 && console.log_text() === "", "console not cleared")
            }
        },
        {
            name: "edge: unknown home user ignored",
            run: () => {
                const system = test.app.systemModel
                const user = system.home_user
                system.select_home_user("definitely-not-a-user")
                test.check(system.home_user === user, "home user changed to " + system.home_user)
            }
        },
        {
            name: "edge: settings layout roundtrip",
            run: () => {
                const settings = test.app.settingsModel
                test.check(settings.layout_mode === "area-module", "default layout is " + settings.layout_mode)
                test.check(settings.layout_rules === "", "default layout has rules")
                test.check(settings.set_layout("bogus", "") !== "", "unknown layout accepted")
                test.check(settings.layout_mode === "area-module", "unknown layout changed the mode")
                test.check(settings.set_layout("area", "no equals here") !== "", "bad rules accepted")
                test.check(settings.layout_mode === "area-module", "bad rules changed the mode")
                test.check(settings.set_layout("single", "") === "", "single layout refused")
                test.check(settings.layout_mode === "single", "single layout not stored")
                test.check(settings.set_layout("area-module", "") === "", "default layout not restored")
                test.check(settings.layout_mode === "area-module", "default layout not stored again")
            }
        },
        {
            name: "edge: setup folder helpers",
            run: () => {
                const setup = test.app.setupModel
                const flake = test.app.systemModel.flake
                test.check(setup.flake_root("relative") === "", "relative path has a flake root")
                test.check(setup.flake_root("/nonexistent-dir-xyz") === "", "missing path has a flake root")
                test.check(setup.flake_root(flake + "/configuration.nix") === setup.flake_root(flake), "flake root moves within the flake")
                test.check(setup.normalize_folder("  /a/b// ") === "/a/b", "folder not normalised")
                test.check(setup.normalize_folder("") === "", "blank folder not blank")
                test.check(setup.effective_folder("", flake) === flake, "blank folder is not the flake")
                test.check(setup.folder_in_flake(flake, flake) === true, "flake not in itself")
                test.check(setup.folder_in_flake("/tmp", flake) === false, "/tmp counted in the flake")
                test.check(setup.folder_in_flake("relative", flake) === false, "relative folder counted in the flake")
                test.check(setup.folder_exists(flake) === true, "flake folder missing")
                test.check(setup.folder_exists("") === false, "blank folder exists")
                test.check(setup.folder_exists(flake + "/no-such-dir") === false, "missing folder exists")
                test.check(setup.create_folder("relative") !== "", "relative folder created")
                const probe = test.dir + "/edge-probe/sub"
                test.check(setup.create_folder(probe) === "", "probe folder not created")
                test.check(setup.folder_exists(probe) === true, "probe folder missing")
            }
        },
        {
            name: "edge: doc text strips images",
            run: () => {
                test.check(Helpers.docText("") === "", "blank doc text is not blank")
                test.check(Helpers.docText(null) === "", "null doc text is not blank")
                const image = Helpers.docText("see ![alt](http://evil/x.png) here")
                test.check(image === "see alt here", "image kept markup: " + image)
                test.check(Helpers.docText("see ![alt][ref] here") === "see alt here", "referenced image kept markup")
                test.check(Helpers.docText("{option}`services.foo`") === "`services.foo`", "MyST role kept markup")
                test.check(Helpers.docText("::: {.note}") === "**note:**", "admonition kept markup")
                const html = Helpers.docText("a <img src=\"http://evil/x.png\"> `<b>` <https://example.org>")
                test.check(html === "a &lt;img src=\"http://evil/x.png\"> `<b>` <https://example.org>", "raw html not escaped: " + html)
            }
        },
        {
            name: "edge: functions index edges",
            run: () => {
                const model = test.app.functionModel
                test.check(model.kinds().indexOf("function") >= 0, "no function kind: " + JSON.stringify(model.kinds()))
                test.check(model.top("", 5).total === 0 && model.top("", 5).rows.length === 0, "blank query matched")
                const negative = model.top("mapAttrs", -1)
                test.check(negative.rows.length === 0, "negative count returned rows")
                if (!test.isDarwin)
                    test.check(model.top("mapAttrs", 5).total > 0, "mapAttrs not found")
                const before = model.total
                model.set_takes("bogus-kind")
                model.set_returns("bogus-kind")
                test.check(model.takes === "" && model.returns === "", "bogus kinds filtered")
                test.check(model.total === before, "bogus kinds changed the total")
            }
        },
        {
            name: "edge: package index ready",
            timeout: 600000,
            until: () => test.app.packageModel.size > 0,
        },
        {
            name: "edge: package index edges",
            run: () => {
                const model = test.app.packageModel
                const unknown = model.details("definitely-not-a-package")
                test.check(unknown.known === false && unknown.state === "unknown", "unknown package has details: " + JSON.stringify(unknown))
                test.check(model.is_known("definitely-not-a-package") === false, "unknown package known")
                if (!test.isDarwin)
                    test.check(model.is_known("hello") === true, "hello not known")
                const commands = model.overview_commands
                test.check(commands.command === "" && commands.attrs.length === 0, "overview commands without command mode: " + JSON.stringify(commands))
                test.check(model.command_mode === false, "command mode on")
                const before = model.total
                model.set_only_installed(true)
                test.check(model.total <= before, "installed filter grew the list")

                test.check(model.total <= model.installed_count, "installed filter shows more than installed")
                model.set_only_installed(false)
                test.check(model.total === before, "installed filter did not restore the list")
            }
        },
        {
            name: "edge: option index edges",
            run: () => {
                const model = test.app.optionModel
                test.check(model.top("networking.hostName", 5).total >= 1, "hostName not in the index")
                test.check(model.top("networking.hostName", 5).rows.length >= 1, "hostName top has no rows")
                test.check(model.top("firewall", 0).rows.length === 0, "zero count returned rows")
                test.check(model.top("zzz-no-such-option", 3).total === 0, "nonsense query matched")
                test.check(model.namespaces().length > 0, "no namespaces")
                if (!test.isDarwin)
                    test.check(model.namespaces().some(namespace => namespace.name === "services" && namespace.count > 0), "no services namespace")
                test.check(Array.isArray(model.undocumented()), "undocumented is not a list")
                test.check(model.values_error === "", "values error: " + model.values_error)
                test.check(model.error === "", "index error: " + model.error)
            }
        },
        {
            name: "edge: tree builds in the background",
            run: () => test.app.treeModel.open_tree(),
            timeout: 60000,
            until: () => !test.app.treeModel.tree_building && test.app.treeModel.rowCount() > 0
        },
        {
            name: "edge: tree collapse roundtrip",
            run: () => {
                const tree = test.app.treeModel
                const rows = tree.rowCount()
                tree.open_tree()
                test.check(!tree.tree_building, "opening the built tree again started a new build")
                test.check(rows > 0 && tree.rowCount() === rows, "opening the built tree again changed it: " + rows + " → " + tree.rowCount())
                tree.expand_all()
                test.check(tree.all_open, "expand all did not open the tree")
                tree.collapse_all()
                test.check(!tree.all_open, "collapse all left the tree open")
                tree.expand_all()
                test.check(tree.all_open, "expand all did not restore the tree")
            }
        }
    ]
}

function extras(test) {
    return [
        {
            name: "edge: every section opens",
            run: () => {
                for (const section of ["overview", "options", "home", "packages", "functions", "generations", "actions", "cleanup", "inputs", "impermanence", "tree", "changes", "configured", "history", "log", "settings", "docs", "about"]) {
                    test.app.showSection(section)
                    test.check(test.app.section === section, "section shows " + test.app.section)
                }
                test.app.showSection("options")
            },
            delay: 500
        },
        {
            name: "edge: staged settings shape",
            run: () => {
                const c = test.app.changeModel
                const staged = c.staged_settings("networking.hostName")
                test.check(staged.mode === "typed" && staged.priority === "" && staged.effectivePriority === "override" && staged.host === "", "staged settings: " + JSON.stringify(staged))
                test.check(c.host_scope_allowed("services.openssh.enable") === true, "host scope refused for openssh.enable")
                test.check(c.staged_input("networking.hostName").length > 0, "hostName has no staged input")
            }
        },
        {
            name: "edge: removing unmanaged packages is a no-op",
            run: () => {
                const before = stagedCount(test)
                const c = test.app.changeModel
                test.check(c.stage_package("git", false) === "", "package removal refused")
                test.check(c.package_state("git") === "", "removing an unmanaged package staged")
                checkCountRestored(test, before, "removing an unmanaged package")
            }
        },
        {
            name: "edge: nothing managed before apply",
            run: () => {
                test.check(test.app.changeModel.managed_count === 0, "managed before apply: " + test.app.changeModel.managed_count)
                test.check(test.app.changeModel.managed_list().length === 0, "managed list before apply is not empty")
            }
        }
    ]
}

function steps(test) {
    return [].concat(universal(test), extras(test))
}

function darwin(test) {
    return universal(test)
}
