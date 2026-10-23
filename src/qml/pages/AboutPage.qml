pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"

Kirigami.ScrollablePage {
    id: page

    required property var app

    title: qsTr("About")

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            Logo {
                Layout.preferredWidth: Kirigami.Units.iconSizes.huge
                Layout.preferredHeight: Kirigami.Units.iconSizes.huge
                Layout.alignment: Qt.AlignTop
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                Kirigami.Heading {
                    level: 1
                    text: qsTr("DrakeFlake")
                }

                Controls.Label {
                    textFormat: Text.PlainText
                    font.italic: true
                    text: qsTr("NixOS, tamed.")
                }

                Controls.Label {
                    opacity: 0.75
                    textFormat: Text.PlainText
                    text: qsTr("Version %1").arg(page.app.settingsModel.version)
                }

                Controls.Label {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    textFormat: Text.PlainText
                    text: qsTr("Search and change the NixOS options, Home Manager settings and packages of your system flake, and keep its inputs up to date. Changes are checked against the option types and written to generated files your configuration imports. The rest of your configuration stays untouched. Works with flakes that define nixosConfigurations or darwinConfigurations.")
                }
            }
        }

        SectionHeading {
            text: qsTr("Links")
        }

        Flow {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            Repeater {
                model: [
                    {
                        text: qsTr("Documentation"),
                        icon: "help-contents",
                        url: "https://dr460nf1r3.github.io/drakeflake/"
                    },
                    {
                        text: qsTr("Source code"),
                        icon: "code-context",
                        url: "https://github.com/dr460nf1r3/drakeflake"
                    },
                    {
                        text: qsTr("Issues"),
                        icon: "dialog-messages",
                        url: "https://github.com/dr460nf1r3/drakeflake/issues"
                    }
                ]

                delegate: Controls.Button {
                    id: link

                    required property var modelData

                    text: link.modelData.text
                    icon.name: link.modelData.icon
                    icon.width: Kirigami.Units.iconSizes.sizeForLabels
                    icon.height: Kirigami.Units.iconSizes.sizeForLabels
                    Controls.ToolTip.visible: link.hovered
                    Controls.ToolTip.text: link.modelData.url

                    onClicked: page.app.openExternal(link.modelData.url)
                }
            }
        }
    }
}
