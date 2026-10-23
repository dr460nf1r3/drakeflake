import QtQuick

QtObject {
    id: settingText

    function priorityLabel(name: string): string {
        switch (name) {
        case "override":
            return qsTr("Override");
        case "default":
            return qsTr("Default");
        case "normal":
            return qsTr("Normal");
        case "force":
            return qsTr("Force");
        case "merge":
            return qsTr("Add (merge)");
        case "before":
            return qsTr("Add first");
        case "after":
            return qsTr("Add last");
        case "replace":
            return qsTr("Replace");
        default:
            return name;
        }
    }

    function priorityHint(name: string): string {
        switch (name) {
        case "override":
            return qsTr("Wins over your configuration (lib.mkOverride 90)");
        case "default":
            return qsTr("Your configuration wins (lib.mkDefault)");
        case "normal":
            return qsTr("Same priority as your configuration. Different values conflict");
        case "force":
            return qsTr("Wins over everything (lib.mkForce)");
        case "merge":
            return qsTr("Added to the items of your configuration");
        case "before":
            return qsTr("Added before the other items (lib.mkBefore)");
        case "after":
            return qsTr("Added after the other items (lib.mkAfter)");
        case "replace":
            return qsTr("Replaces what your configuration defines (lib.mkForce)");
        default:
            return "";
        }
    }

    function summary(priority: string, mode: string, host: string): string {
        const parts = [];

        if (priority.length > 0)
            parts.push(settingText.priorityLabel(priority));
        if (mode === "expression")
            parts.push(qsTr("Nix expression"));
        if (host.length > 0)
            parts.push(qsTr("only on %1").arg(host));
        return parts.join(" · ");
    }
}
