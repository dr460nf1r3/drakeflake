import QtQuick
import QtTest

import "../src/qml/pages/overview/results.js" as Search

Item {
    id: root

    width: 200
    height: 200

    TestCase {
        function stubApp(): var {
            let empty = {
                rows: [],
                total: 0
            };
            return {
                systemModel: {
                    capabilities: {
                        generations: true,
                        images: true,
                        rebuild: ["switch", "boot", "test", "build"]
                    },
                    kind: "nixos",
                    home_user: "alice"
                },
                optionModel: {
                    top: (query, n) => empty
                },
                configuredModel: {
                    top: (query, n) => empty
                },
                homeModel: {
                    size: 0,
                    top: (query, n) => empty
                },
                hasHomeManager: false,
                packageModel: {
                    top: (query, n) => empty,
                    overview_commands: {
                        command: "",
                        attrs: []
                    }
                },
                functionModel: {
                    top: (query, n) => empty
                },
                searchModel: {
                    flake_hits: {
                        query: "",
                        total: 0,
                        hits: []
                    }
                },
                inputsModel: {
                    inputs: []
                },
                generationsModel: {
                    generations: [],
                    history: []
                }
            };
        }

        function groupByKey(groups: var, key: string): var {
            return groups.find(group => group.key === key);
        }

        function test_optionGroupConfigured(): void {
            let model = {
                top: (query, n) => ({
                            rows: [
                                {
                                    name: "services.foo",
                                    summary: "summary",
                                    value: "true",
                                    defined_in: "flake.nix"
                                }
                            ],
                            total: 7
                        })
            };
            let grouped = Search.optionGroup("configured", "T", qsTr("All %1 in X"), model, "q", "configured", name => name);
            compare(grouped.items.length, 1);
            compare(grouped.items[0].title, "services.foo");
            compare(grouped.items[0].subtitle, qsTr("Set in %1").arg("flake.nix"));
            compare(grouped.items[0].trailing, "true");
            compare(grouped.items[0].mono, true);
            compare(grouped.items[0].open.kind, "configured");
            compare(grouped.items[0].open.name, "services.foo");
            compare(grouped.more, qsTr("All %1 in X").arg("7"));
            compare(grouped.moreOpen.query, "q");
        }

        function test_optionGroupPlainSubtitle(): void {
            let model = {
                top: (query, n) => ({
                            rows: [
                                {
                                    name: "services.foo",
                                    summary: "summary",
                                    value: "true"
                                }
                            ],
                            total: 1
                        })
            };
            let grouped = Search.optionGroup("options", "T", qsTr("All %1 in X"), model, "q", "option", name => name);
            compare(grouped.items[0].subtitle, "summary");
        }

        function test_packageGroupMapsRows(): void {
            let app = stubApp();
            app.packageModel.top = (query, n) => ({
                        rows: [
                            {
                                attr: "firefox",
                                description: "browser",
                                installed: true
                            }
                        ],
                        total: 1
                    });
            let grouped = Search.packageGroup(app, "fox");
            compare(grouped.items.length, 1);
            compare(grouped.items[0].title, "firefox");
            compare(grouped.items[0].trailing, qsTr("installed"));
            compare(grouped.items[0].open.kind, "package");
            compare(grouped.items[0].open.attr, "firefox");
            compare(grouped.moreOpen.section, "packages");
        }

        function test_commandGroupMatches(): void {
            let app = stubApp();
            app.packageModel.overview_commands = {
                command: "git",
                attrs: ["git"]
            };
            let grouped = Search.commandGroup(app, "git");
            verify(grouped !== null);
            compare(grouped.items.length, 1);
            compare(grouped.items[0].open.attr, "git");
            compare(Search.commandGroup(app, "vim"), null);
        }

        function test_sourceGroupShowsAttributesAndValues(): void {
            let app = stubApp();
            app.searchModel.flake_hits = {
                query: "ssh",
                total: 2,
                hits: [
                    {
                        file: "/f/hosts/a.nix",
                        display: "hosts/a.nix",
                        line: 4,
                        kind: "attribute",
                        path: "services.openssh.enable",
                        snippet: "true"
                    },
                    {
                        file: "/f/hosts/a.nix",
                        display: "hosts/a.nix",
                        line: 9,
                        kind: "value",
                        path: "environment.systemPackages",
                        snippet: "pkgs.sshfs"
                    }
                ]
            };
            let grouped = Search.sourceGroup(app, "ssh");
            compare(grouped.items[0].title, "services.openssh.enable");
            compare(grouped.items[0].subtitle, "hosts/a.nix:4 · true");
            compare(grouped.items[1].title, "pkgs.sshfs");
            compare(grouped.items[1].subtitle, "environment.systemPackages · hosts/a.nix:9");
            compare(grouped.items[1].open.kind, "source");
            compare(grouped.items[1].open.line, 9);
        }

        function test_sourceGroupIgnoresOlderQueries(): void {
            let app = stubApp();
            app.searchModel.flake_hits = {
                query: "ss",
                total: 1,
                hits: [
                    {
                        file: "/f/a.nix",
                        display: "a.nix",
                        line: 1,
                        kind: "value",
                        path: "",
                        snippet: "ss"
                    }
                ]
            };
            compare(Search.sourceGroup(app, "ssh"), null);
            compare(Search.sourceGroup(app, "ss").items.length, 1);
        }

        function test_commandGroupEmptyAttrs(): void {
            let app = stubApp();
            app.packageModel.overview_commands = {
                command: "git",
                attrs: []
            };
            compare(Search.commandGroup(app, "git"), null);
        }

        function test_functionGroupBuiltin(): void {
            let app = stubApp();
            app.functionModel.top = (query, n) => ({
                        rows: [
                            {
                                path: "builtins.map",
                                signature: "",
                                summary: "summary",
                                builtin: true
                            }
                        ],
                        total: 1
                    });
            let grouped = Search.functionGroup(app, "map");
            compare(grouped.items[0].title, "builtins.map");
            compare(grouped.items[0].trailing, qsTr("builtin"));
            compare(grouped.items[0].open.kind, "function");
            compare(grouped.items[0].open.path, "builtins.map");
        }

        function test_inputGroupMarksUpdate(): void {
            let app = stubApp();
            app.inputsModel.inputs = [
                {
                    name: "nixpkgs",
                    source: "github:NixOS/nixpkgs",
                    check: {
                        state: "available"
                    }
                }
            ];
            let grouped = Search.inputGroup(app, "");
            compare(grouped.items.length, 1);
            compare(grouped.items[0].title, "nixpkgs");
            compare(grouped.items[0].trailing, qsTr("update available"));
            compare(grouped.items[0].open.section, "inputs");
        }

        function test_generationGroupRunning(): void {
            let app = stubApp();
            app.generationsModel.generations = [
                {
                    id: 2,
                    date: 1577836800,
                    version: "",
                    kernel: "",
                    running: true
                }
            ];
            let grouped = Search.generationGroup(app, "");
            compare(grouped.items.length, 1);
            compare(grouped.items[0].title, qsTr("Generation %1").arg(2));
            compare(grouped.items[0].trailing, qsTr("running"));
        }

        function test_historyGroupEarlier(): void {
            let app = stubApp();
            app.generationsModel.history = [
                {
                    saved: 0,
                    changes: ["a", "b", "c", "d"]
                }
            ];
            let grouped = Search.historyGroup(app, "");
            compare(grouped.items.length, 1);
            compare(grouped.items[0].title, qsTr("Earlier settings"));
            compare(grouped.items[0].subtitle, "a, b, c");
        }

        function test_groupsHideUnoffered(): void {
            let app = stubApp();
            app.systemModel.capabilities = {
                generations: false,
                images: false,
                rebuild: []
            };
            let groups = Search.groups(app, "inputs");
            compare(groupByKey(groups, "pages").items[0].open.section, "inputs");
            compare(groupByKey(Search.groups(app, "generations"), "pages").items.length, 0);
            compare(groupByKey(Search.groups(app, "rebuild"), "actions").items.length, 0);
            compare(groupByKey(Search.groups(app, "image"), "actions").items.length, 0);
        }

        function test_groupsOfferCapabilities(): void {
            let app = stubApp();
            let actions = groupByKey(Search.groups(app, "rebuild"), "actions");
            verify(actions.items.some(item => item.open.kind === "rebuild" && item.open.mode === "switch"));
            let imaged = Search.groups(app, "image");
            verify(groupByKey(imaged, "actions").items.some(item => item.open.images));
        }

        function test_groupsDarwinTitle(): void {
            let app = stubApp();
            app.systemModel.kind = "darwin";
            let groups = Search.groups(app, "");
            compare(groupByKey(groups, "options").title, qsTr("nix-darwin options"));
        }

        function test_groupsHomeSection(): void {
            let app = stubApp();
            app.hasHomeManager = true;
            app.homeModel.size = 1;
            let groups = Search.groups(app, "");
            compare(groupByKey(groups, "home").title, qsTr("Home Manager · %1").arg("alice"));
            app.homeModel.size = 0;
            compare(groupByKey(Search.groups(app, ""), "home"), undefined);
        }

        function test_optionGroupEmpty(): void {
            let model = {
                top: (query, n) => ({
                            rows: [],
                            total: 0
                        })
            };
            let grouped = Search.optionGroup("options", "T", qsTr("All %1 in X"), model, "q", "option", name => name);
            compare(grouped.items.length, 0);
            compare(grouped.total, 0);
            compare(grouped.more, "");
        }

        function test_packageGroupNotInstalled(): void {
            let app = stubApp();
            app.packageModel.top = (query, n) => ({
                        rows: [
                            {
                                attr: "vim",
                                description: "editor",
                                installed: false
                            }
                        ],
                        total: 1
                    });
            let grouped = Search.packageGroup(app, "vim");
            compare(grouped.items[0].trailing, "");
        }

        function test_commandGroupTrimsQuery(): void {
            let app = stubApp();
            app.packageModel.overview_commands = {
                command: "git",
                attrs: ["git"]
            };
            let grouped = Search.commandGroup(app, "  git  ");
            verify(grouped !== null);
            compare(grouped.items[0].open.attr, "git");
        }

        function test_historyGroupReplaced(): void {
            let app = stubApp();
            app.generationsModel.history = [
                {
                    saved: 1577836800,
                    changes: []
                }
            ];
            let grouped = Search.historyGroup(app, "");
            compare(grouped.items.length, 1);
            verify(grouped.items[0].title.startsWith(qsTr("Replaced")));
            compare(grouped.items[0].subtitle, "");
        }

        function test_generationGroupIdle(): void {
            let app = stubApp();
            app.generationsModel.generations = [
                {
                    id: 3,
                    date: 1577836800,
                    version: "",
                    kernel: "",
                    running: false
                }
            ];
            let grouped = Search.generationGroup(app, "");
            compare(grouped.items[0].trailing, "");
        }

        function test_groupsEmptyQuery(): void {
            let app = stubApp();
            compare(Search.rows(Search.groups(app, "zzz-no-match")).length, 0);
        }

        name: "OverviewSearchGroupTests"
        when: windowShown
    }
}
