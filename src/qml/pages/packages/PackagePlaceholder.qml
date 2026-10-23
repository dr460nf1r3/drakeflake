import QtQuick
import org.kde.kirigami as Kirigami

import "../../main/kinds.js" as Kinds

Kirigami.PlaceholderMessage {
    id: placeholder

    required property var packages
    required property string query
    readonly property bool failed: placeholder.packages.error.length > 0 && placeholder.packages.size === 0

    icon.name: placeholder.failed ? "data-error" : "search"
    text: {
        if (placeholder.failed)
            return qsTr("Could not index packages");
        if (placeholder.packages.set_loading)
            return qsTr("Indexing %1…").arg(placeholder.packages.package_set);
        if (placeholder.packages.size === 0)
            return qsTr("Indexing packages…");
        if (placeholder.packages.command_mode) {
            if (placeholder.packages.command_state === Kinds.commandSearching)
                return qsTr("Looking up the command…");
            if (placeholder.packages.command_state === Kinds.commandDownloading)
                return qsTr("Downloading the command index…");
            return placeholder.query.length > 0 ? qsTr("No package has a program named %1").arg(placeholder.query) : qsTr("Type a program name");
        }
        return qsTr("No matching packages");
    }
    explanation: {
        if (placeholder.failed)
            return placeholder.packages.error;
        if (placeholder.packages.set_loading)
            return qsTr("Package sets are indexed the first time you open them, then kept until your flake changes.");
        if (placeholder.packages.size === 0)
            return qsTr("Packages come from the nixpkgs your system uses. This takes a few seconds the first time.");
        if (placeholder.packages.command_mode)
            return qsTr("Programs are matched by their exact name in bin/.");
        return qsTr("Try another name or a word from the description.");
    }
}
