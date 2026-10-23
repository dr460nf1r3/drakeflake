pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../main/helpers.js" as Helpers

GridLayout {
    id: tiles

    required property var app
    readonly property bool generations: tiles.app.systemModel.target_ready && tiles.app.systemModel.capabilities.generations
    readonly property var running: tiles.generations ? tiles.app.generationsModel.generations.find(generation => generation.running) || null : null
    readonly property int updates: tiles.app.inputsModel.inputs.filter(input => input.check && input.check.state === "available").length
    readonly property int staged: tiles.app.changeModel.count
    readonly property bool hasTarget: tiles.app.systemModel.target_error === ""
    readonly property var options: tiles.app.optionModel
    readonly property var packages: tiles.app.packageModel
    readonly property bool optionsLoading: tiles.hasTarget && tiles.options.size === 0 && tiles.options.error === ""
    readonly property string homeState: tiles.app.systemModel.home_state
    readonly property bool homeBusy: tiles.hasTarget && (tiles.homeState === "" || (tiles.homeState === "available" && tiles.app.homeModel.size === 0))
    readonly property string homeNote: {
        if (tiles.homeState === "none")
            return qsTr("Not set up on this host");
        if (tiles.homeState.startsWith("error"))
            return qsTr("Could not check");
        if (tiles.homeState === "")
            return tiles.hasTarget ? qsTr("Checking…") : "";
        return tiles.homeBusy ? qsTr("Loading %1…").arg(tiles.app.systemModel.home_user) : tiles.app.systemModel.home_user;
    }
    readonly property bool packagesLoading: tiles.hasTarget && tiles.packages.error === "" && tiles.packages.total === 0
    readonly property var functions: tiles.app.functionModel
    readonly property string functionsNote: {
        switch (tiles.functions.state) {
        case "ready":
            return qsTr("%1 Nix functions").arg(Helpers.count(tiles.functions.size));
        case "missing":
        case "error":
            return qsTr("Index not available");
        default:
            return qsTr("Reading the index…");
        }
    }
    readonly property var entries: [
        {
            section: "options",
            title: qsTr("Options"),
            note: tiles.options.size === 0 ? (tiles.optionsLoading ? qsTr("Loading options…") : "") : !tiles.options.values_ready && tiles.options.values_error === "" ? qsTr("%1 options, reading values…").arg(Helpers.count(tiles.options.size)) : qsTr("%1 options").arg(Helpers.count(tiles.options.size)),
            accent: false,
            busy: tiles.optionsLoading || (tiles.options.size > 0 && !tiles.options.values_ready && tiles.options.values_error === "")
        },
        {
            section: "home",
            title: qsTr("Home Manager"),
            note: tiles.homeNote,
            accent: false,
            busy: tiles.homeBusy
        },
        {
            section: "packages",
            title: qsTr("Packages"),
            note: tiles.packages.installed_count > 0 ? qsTr("%1 installed").arg(Helpers.count(tiles.packages.installed_count)) : tiles.packagesLoading ? qsTr("Loading packages…") : "",
            accent: false,
            busy: tiles.packagesLoading
        },
        {
            section: "functions",
            title: qsTr("Functions"),
            note: tiles.functionsNote,
            accent: false,
            busy: tiles.functions.state === "loading"
        },
        {
            section: "generations",
            title: qsTr("Generations"),
            note: tiles.running ? qsTr("%1 running").arg(tiles.running.id) : "",
            accent: false
        },
        {
            section: "inputs",
            title: qsTr("Inputs"),
            note: tiles.updates > 0 ? (tiles.updates === 1 ? qsTr("1 update available") : qsTr("%1 updates available").arg(tiles.updates)) : tiles.app.inputsModel.inputs.length === 1 ? qsTr("1 input") : qsTr("%1 inputs").arg(tiles.app.inputsModel.inputs.length),
            accent: tiles.updates > 0
        },
        {
            section: "changes",
            title: qsTr("Changes"),
            note: tiles.staged > 0 ? qsTr("%1 staged").arg(tiles.staged) : qsTr("Nothing staged"),
            accent: tiles.staged > 0
        },
        {
            section: "actions",
            title: qsTr("Actions"),
            note: tiles.app.systemModel.capabilities.images ? qsTr("Images, VM, recommendations") : qsTr("Recommendations"),
            accent: false
        },
        {
            section: "cleanup",
            title: qsTr("Cleanup"),
            note: qsTr("Free disk space"),
            accent: false
        }
    ].filter(entry => tiles.generations || entry.section !== "generations")

    columns: Math.max(1, Math.floor(tiles.width / (Kirigami.Units.gridUnit * 11)))
    columnSpacing: Kirigami.Units.largeSpacing
    rowSpacing: Kirigami.Units.largeSpacing

    Repeater {
        model: tiles.entries

        delegate: Kirigami.AbstractCard {
            id: tile

            required property var modelData

            Layout.fillWidth: true
            Layout.preferredWidth: Kirigami.Units.gridUnit * 10
            showClickFeedback: true
            Accessible.name: tile.modelData.title

            contentItem: ColumnLayout {
                spacing: Kirigami.Units.smallSpacing

                Kirigami.Heading {
                    Layout.fillWidth: true
                    level: 4
                    text: tile.modelData.title
                    elide: Text.ElideRight
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing

                    Controls.BusyIndicator {
                        Layout.preferredWidth: Kirigami.Units.iconSizes.small
                        Layout.preferredHeight: Kirigami.Units.iconSizes.small
                        visible: tile.modelData.busy === true
                        running: visible
                    }

                    Controls.Label {
                        Layout.fillWidth: true
                        text: tile.modelData.note
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        color: tile.modelData.accent ? Kirigami.Theme.linkColor : Kirigami.Theme.disabledTextColor
                        font.weight: tile.modelData.accent ? Font.DemiBold : Font.Normal
                    }
                }
            }

            onClicked: tiles.app.showSection(tile.modelData.section)

            Binding {
                when: tiles.app.vibrancy
                target: tile.background
                property: "defaultColor"
                value: Qt.alpha(Kirigami.Theme.textColor, 0.07)
            }

            // Kirigami's depth layer under the card would show as a dark slab.
            Binding {
                when: tiles.app.vibrancy && tile.background.children.length > 0
                target: tile.background.children[0] ?? null
                property: "color"
                value: "transparent"
            }
        }
    }
}
