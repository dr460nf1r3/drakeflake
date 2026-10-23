import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../main/kinds.js" as Kinds

RowLayout {
    id: status

    required property string check
    required property string message
    readonly property color tone: status.check === Kinds.checkOk ? Kirigami.Theme.positiveTextColor : status.check === Kinds.checkInvalid || status.check === Kinds.checkFailed ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.disabledTextColor

    spacing: Kirigami.Units.smallSpacing

    Kirigami.Icon {
        implicitWidth: Kirigami.Units.iconSizes.small
        implicitHeight: implicitWidth
        source: status.check === Kinds.checkOk ? "dialog-ok" : status.check === Kinds.checkPending ? "view-refresh" : status.check === Kinds.checkUnknown ? "help-hint" : "dialog-cancel"
        color: status.tone
    }

    Controls.Label {
        Layout.fillWidth: true
        textFormat: Text.PlainText
        text: status.message
        wrapMode: Text.Wrap
        font: Kirigami.Theme.smallFont
        color: status.tone
    }
}
