import QtQuick
import QtTest

import "../src/qml/pages/actions/recommendations.js" as Recommendations

Item {
    id: root

    width: 200
    height: 200

    TestCase {
        function healthy(): var {
            return {
                "documentation.nixos.enable": "true",
                "programs.nh.enable": "true",
                "nix.settings.auto-optimise-store": "true",
                "nix.optimise.automatic": "true",
                "nix.gc.automatic": "true",
                "programs.nh.clean.enable": "true"
            };
        }

        function runWith(values: var, staged: var, darwin: var, generations: var, inputs: var, undocumented: var): var {
            let options = {
                details: name => {
                    const entry = values[name];
                    if (entry && typeof entry === "object")
                        return {
                            valueDisplay: entry.display,
                            valueInput: entry.input,
                            type: entry.type
                        };
                    return {
                        valueDisplay: entry
                    };
                },
                undocumented: () => undocumented || []
            };
            let changes = {
                option_state: name => (staged && staged[name]) || ""
            };
            return Recommendations.list(options, changes, darwin, generations, inputs);
        }

        function keysOf(found: var): string {
            return found.map(entry => entry.key).join(",");
        }

        function test_emptyWhenHealthy(): void {
            let found = runWith(healthy(), {}, false, [], [], []);
            compare(found.length, 0);
        }

        function test_fixDocs(): void {
            let found = runWith(healthy(), {}, false, [], [], ["a.b", "c.d"]);
            compare(keysOf(found), "fix-docs");
            compare(found[0].search, "a.b");
        }

        function test_manual(): void {
            let values = healthy();
            values["documentation.nixos.enable"] = "false";
            let found = runWith(values, {}, false, [], [], []);
            compare(keysOf(found), "manual");
            compare(found[0].option, "documentation.nixos.enable");
            compare(found[0].input, "true");
        }

        function test_manualSuppressedWhenStaged(): void {
            let values = healthy();
            values["documentation.nixos.enable"] = "false";
            let found = runWith(values, {
                "documentation.nixos.enable": "staged"
            }, false, [], [], []);
            compare(found.length, 0);
        }

        function test_nh(): void {
            let values = healthy();
            values["programs.nh.enable"] = "false";
            let found = runWith(values, {}, false, [], [], []);
            compare(keysOf(found), "nh");
            compare(found[0].option, "programs.nh.enable");
        }

        function test_optimiseNixos(): void {
            let values = healthy();
            values["nix.settings.auto-optimise-store"] = "false";
            let found = runWith(values, {}, false, [], [], []);
            compare(keysOf(found), "optimise");
            compare(found[0].option, "nix.settings.auto-optimise-store");
        }

        function test_optimiseDarwin(): void {
            let values = healthy();
            values["nix.optimise.automatic"] = "false";
            let found = runWith(values, {}, true, [], [], []);
            compare(keysOf(found), "optimise");
            compare(found[0].option, "nix.optimise.automatic");
        }

        function test_darwinSuggestsNoNixosOptimise(): void {
            let values = healthy();
            values["nix.settings.auto-optimise-store"] = "false";
            let found = runWith(values, {}, true, [], [], []);
            compare(found.length, 0);
        }

        function test_cleanNeedsNh(): void {
            let values = healthy();
            values["nix.gc.automatic"] = "false";
            values["programs.nh.clean.enable"] = "false";
            let found = runWith(values, {}, false, [], [], []);
            compare(keysOf(found), "clean");
            compare(found[0].option, "programs.nh.clean.enable");
        }

        function test_cleanStagesUserOnDarwin(): void {
            let values = healthy();
            values["nix.gc.automatic"] = "false";
            values["programs.nh.clean.enable"] = "false";
            values["system.primaryUser"] = {
                display: "\"nijen\"",
                input: "nijen"
            };
            values["programs.nh.clean.user"] = {
                display: "null",
                input: "null",
                type: "null or string"
            };
            let found = runWith(values, {}, true, [], [], []);
            compare(keysOf(found), "clean");
            compare(found[0].also.length, 1);
            compare(found[0].also[0].option, "programs.nh.clean.user");
            compare(found[0].also[0].input, "nijen");
        }

        function test_cleanKeepsConfiguredUser(): void {
            let values = healthy();
            values["nix.gc.automatic"] = "false";
            values["programs.nh.clean.enable"] = "false";
            values["system.primaryUser"] = {
                display: "\"nijen\"",
                input: "nijen"
            };
            values["programs.nh.clean.user"] = {
                display: "\"boss\"",
                input: "boss",
                type: "null or string"
            };
            let found = runWith(values, {}, true, [], [], []);
            compare(keysOf(found), "clean");
            compare(found[0].also, undefined);
        }

        function test_cleanNeedsNoUserOffDarwin(): void {
            let values = healthy();
            values["nix.gc.automatic"] = "false";
            values["programs.nh.clean.enable"] = "false";
            values["system.primaryUser"] = {
                display: "\"nijen\"",
                input: "nijen"
            };
            let found = runWith(values, {}, false, [], [], []);
            compare(keysOf(found), "clean");
            compare(found[0].also, undefined);
        }

        function test_cleanWithoutKnownUser(): void {
            let values = healthy();
            values["nix.gc.automatic"] = "false";
            values["programs.nh.clean.enable"] = "false";
            let found = runWith(values, {}, true, [], [], []);
            compare(keysOf(found), "clean");
            compare(found[0].also, undefined);
        }

        function test_gcWithoutNh(): void {
            let values = healthy();
            delete values["programs.nh.enable"];
            values["nix.gc.automatic"] = "false";
            let found = runWith(values, {}, false, [], [], []);
            compare(keysOf(found), "gc");
            compare(found[0].option, "nix.gc.automatic");
        }

        function test_generations(): void {
            let generations = [];
            for (let i = 0; i < 21; ++i)
                generations.push({
                    id: i
                });
            let found = runWith(healthy(), {}, false, generations, [], []);
            compare(keysOf(found), "generations");
            compare(found[0].section, "cleanup");
            compare(found[0].title, qsTranslate("ActionsPage", "%1 system generations are kept").arg(21));
        }

        function test_inputsOld(): void {
            let inputs = [
                {
                    name: "nixpkgs",
                    last_modified: 1577836800
                }
            ];
            let found = runWith(healthy(), {}, false, [], inputs, []);
            compare(keysOf(found), "inputs");
            compare(found[0].section, "inputs");
        }

        function test_inputsFresh(): void {
            let inputs = [
                {
                    name: "nixpkgs",
                    last_modified: Date.now() / 1000
                }
            ];
            let found = runWith(healthy(), {}, false, [], inputs, []);
            compare(found.length, 0);
        }

        function test_combinedOrder(): void {
            let values = healthy();
            values["documentation.nixos.enable"] = "false";
            values["programs.nh.enable"] = "false";
            values["nix.settings.auto-optimise-store"] = "false";
            let generations = [];
            for (let i = 0; i < 21; ++i)
                generations.push({
                    id: i
                });
            let inputs = [
                {
                    name: "nixpkgs",
                    last_modified: 1577836800
                }
            ];
            let found = runWith(values, {}, false, generations, inputs, ["a.b"]);
            compare(keysOf(found), "fix-docs,manual,nh,optimise,generations,inputs");
        }

        function test_generationsBoundary(): void {
            let generations = [];
            for (let i = 0; i < 20; ++i)
                generations.push({
                    id: i
                });
            let found = runWith(healthy(), {}, false, generations, [], []);
            compare(found.length, 0);
        }

        function test_inputsWithoutTimestamp(): void {
            let found = runWith(healthy(), {}, false, [], [
                {
                    name: "nixpkgs"
                }
            ], []);
            compare(found.length, 0);
        }

        function test_inputsWithoutNixpkgs(): void {
            let inputs = [
                {
                    name: "other",
                    last_modified: 1577836800
                }
            ];
            let found = runWith(healthy(), {}, false, [], inputs, []);
            compare(found.length, 0);
        }

        name: "RecommendationsTests"
        when: windowShown
    }
}
