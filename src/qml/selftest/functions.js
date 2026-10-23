.pragma library
.import "../pages/overview/results.js" as Results


function functions(test) {
    return test.app.functionModel;
}

function paths(rows) {
    return rows.map(row => row.path);
}

function steps(test) {
    return [
        {
            name: "functions page",
            run: () => test.app.showSection("functions"),
            until: () => functions(test).state !== "idle" && functions(test).state !== "loading",
            then: () => {
                const model = functions(test);
                test.check(model.state === "ready", "function index not loaded: " + model.state + " " + model.error);
                test.check(model.size > 500, "function index has only " + model.size + " functions");
                test.check(model.namespaces.some(namespace => namespace.name === "lib.strings" && namespace.card), "no lib.strings card");
                test.log("functions: " + model.size + " in the index");
            },
            delay: 1500
        },
        {
            name: "screenshot functions overview",
            run: () => test.shot("01d-functions-overview")
        },
        {
            name: "functions search",
            run: () => functions(test).search("concatStrings"),
            delay: 1000,
            then: () => {
                const model = functions(test);
                const top = paths(model.top("concatStrings", 5).rows);
                test.check(top.includes("lib.strings.concatStrings"), "concatStrings not found: " + top.join(", "));
                test.check(model.total > 0 && model.browsing, "the list shows no matches");

                const info = model.details("lib.strings.concatStrings");
                test.check(info.known && info.signature.length > 0, "concatStrings has no signature");
                test.check(info.blocks.some(block => block.kind === "code"), "concatStrings has no example");
                test.check(info.aliases.includes("lib.concatStrings"), "lib.concatStrings is not an alias: " + info.aliases.join(", "));
                test.check(model.details("lib.concatStrings").doc_from === "lib.strings.concatStrings", "the alias does not share the documentation");
                test.shot("01e-functions-search");
            }
        },
        {
            name: "function details",
            run: () => test.app.showFunction("lib.strings.concatStrings"),
            delay: 1500,
            then: () => {
                const top = test.app.pageStack.items[test.app.pageStack.depth - 1];
                test.check(top && top.isFunctionDetails && top.path === "lib.strings.concatStrings", "the details page is not open");
                test.shot("01f-function-details");
            }
        },
        {
            name: "builtin details",
            run: () => test.app.showFunction("builtins.map"),
            delay: 1000,
            then: () => {
                const info = functions(test).details("builtins.map");
                test.check(info.known && info.builtin && info.blocks.length > 0, "builtins.map is not documented");
                test.check(info.args.length === 2, "builtins.map has " + info.args.length + " arguments");
                test.shot("01g-builtin-details");
            }
        },
        {
            name: "functions type filter",
            run: () => {
                test.app.showSection("functions");
                functions(test).search("");
                functions(test).set_takes("function");
                functions(test).set_returns("attrset");
            },
            delay: 1000,
            then: () => {
                const model = functions(test);
                test.check(model.total > 0, "no function takes a function and returns an attribute set");
                test.check(model.details("lib.attrsets.mapAttrs").takes === "function", "mapAttrs does not take a function");
                test.shot("01h-functions-type-filter");
            }
        },
        {
            name: "functions without matches",
            run: () => functions(test).search("no-function-is-called-this"),
            delay: 1000,
            then: () => {
                test.check(functions(test).total === 0, "a nonsense query matched");
                test.check(!functions(test).details("lib.no.such.function").known, "an unknown path has details");
                test.shot("01i-functions-none");
            }
        },
        {
            name: "functions in the overview search",
            run: () => {
                functions(test).reset();
                test.app.functionQuery = "";
                const groups = Results.groups(test.app, "mapAttrs");
                const found = groups.find(group => group.key === "functions");
                test.check(found && found.items.length > 0, "the overview search finds no functions for mapAttrs");
            }
        }
    ];
}
