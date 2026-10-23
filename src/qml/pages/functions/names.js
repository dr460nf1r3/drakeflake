.pragma library


function kindLabel(kind) {
    switch (kind) {
    case "string":
        return qsTr("String");
    case "number":
        return qsTr("Number");
    case "bool":
        return qsTr("Boolean");
    case "path":
        return qsTr("Path");
    case "list":
        return qsTr("List");
    case "attrset":
        return qsTr("Attribute set");
    case "function":
        return qsTr("Function");
    case "derivation":
        return qsTr("Derivation");
    case "null":
        return qsTr("Null");
    case "any":
        return qsTr("Any type (a, b, …)");
    default:
        return qsTr("Anything");
    }
}

function kindOptions(kinds) {
    return [
        {
            value: "",
            text: kindLabel("")
        }
    ].concat(kinds.map(kind => ({
                value: kind,
                text: kindLabel(kind)
            })));
}

function namespaceNote(name) {
    switch (name) {
    case "builtins":
        return qsTr("Built into Nix");
    case "lib":
        return qsTr("The nixpkgs library");
    case "lib.attrsets":
        return qsTr("Attribute sets");
    case "lib.strings":
        return qsTr("Text");
    case "lib.lists":
        return qsTr("Lists");
    case "lib.trivial":
        return qsTr("Small helpers");
    case "lib.options":
        return qsTr("Declaring module options");
    case "lib.modules":
        return qsTr("The module system");
    case "lib.types":
        return qsTr("Option types");
    case "lib.fileset":
        return qsTr("Selecting source files");
    case "lib.generators":
        return qsTr("Writing config file formats");
    case "pkgs":
        return qsTr("Builders and fetchers");
    case "pkgs.writers":
        return qsTr("Scripts in many languages");
    case "pkgs.formats":
        return qsTr("Settings files for modules");
    case "pkgs.testers":
        return qsTr("Package tests");
    case "pkgs.dockerTools":
        return qsTr("Container images");
    default:
        return "";
    }
}

function linkedFunction(link) {
    const library = link.match(/^#function-library-(.+)$/);
    if (library)
        return library[1];

    const builtin = link.match(/^#builtins-(.+)$/);
    return builtin ? "builtins." + builtin[1] : "";
}
