import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"
import "settings"

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var system: page.app.systemModel
    readonly property var settingsModel: page.app.settingsModel
    readonly property var setupModel: page.app.setupModel
    readonly property real labelWidth: Math.max(systemSection.labelWidth, layoutSection.labelWidth, traySection.labelWidth, highlightSection.labelWidth, appearanceSection.labelWidth, loggingSection.labelWidth, applySection.labelWidth)

    title: qsTr("Settings")

    ColumnLayout {
        spacing: Kirigami.Units.gridUnit

        SystemSection {
            id: systemSection

            labelColumnWidth: page.labelWidth
            system: page.system
            setupModel: page.setupModel

            onBrowseFlake: flakePicker.open()
            onChooseOutputFile: outputFilePicker.open()
            onChooseOutputFolder: outputFolderPicker.open()
            onSetupRequested: page.app.setupOpen = true
        }

        LayoutSection {
            id: layoutSection

            labelColumnWidth: page.labelWidth
            settingsModel: page.settingsModel
        }

        ApplySection {
            id: applySection

            labelColumnWidth: page.labelWidth
            settingsModel: page.settingsModel
        }

        TraySection {
            id: traySection

            labelColumnWidth: page.labelWidth
            settingsModel: page.settingsModel
            macos: page.system.capabilities.platform === "darwin"
        }

        HighlightSection {
            id: highlightSection

            labelColumnWidth: page.labelWidth
            settingsModel: page.settingsModel
            desktopHighlightTheme: page.app.desktopHighlightTheme
            highlightTheme: page.app.highlightTheme
        }

        AppearanceSection {
            id: appearanceSection

            labelColumnWidth: page.labelWidth
            settingsModel: page.settingsModel
            macos: page.system.capabilities.platform === "darwin"
        }

        LoggingSection {
            id: loggingSection

            labelColumnWidth: page.labelWidth
            settingsModel: page.settingsModel
        }
    }

    PortalPicker {
        id: outputFilePicker

        setupModel: page.setupModel
        purpose: "settings-output-file"
        folder: false
        title: qsTr("Choose the generated .nix file")
        suggestedName: "drakeflake.nix"
        startFolder: systemSection.startFolder

        onPicked: path => systemSection.setOutput(path)
    }

    PortalPicker {
        id: outputFolderPicker

        setupModel: page.setupModel
        purpose: "settings-output-folder"
        title: qsTr("Choose the folder for drakeflake.nix")
        startFolder: systemSection.startFolder

        onPicked: path => systemSection.setOutput(path + "/")
    }

    PortalPicker {
        id: flakePicker

        setupModel: page.setupModel
        purpose: "settings-flake"
        title: qsTr("Choose your system flake")
        startFolder: systemSection.startFolder

        onPicked: path => systemSection.setFlake(path)
    }
}
