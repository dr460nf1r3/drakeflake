pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

Rectangle {
    id: card

    property alias model: view.model
    property alias delegate: view.delegate
    property alias listName: view.objectName
    readonly property alias count: view.count
    property alias hint: placeholder.text
    property alias hintExplanation: placeholder.explanation
    property bool hintShown: view.count === 0

    radius: Kirigami.Units.cornerRadius
    color: Qt.alpha(Kirigami.Theme.backgroundColor, 0.6)
    border.width: 1
    border.color: Qt.alpha(Kirigami.Theme.textColor, 0.12)

    Controls.ScrollView {
        anchors.fill: parent
        anchors.margins: card.radius / 2

        ListView {
            id: view

            clip: true
            reuseItems: true

            Kirigami.PlaceholderMessage {
                id: placeholder

                anchors.centerIn: parent
                width: parent.width - Kirigami.Units.gridUnit * 2
                visible: card.hintShown
            }
        }
    }
}
