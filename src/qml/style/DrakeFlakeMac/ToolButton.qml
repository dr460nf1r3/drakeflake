import QtQuick
import QtQuick.Templates as T
import QtQuick.Controls.impl
import org.kde.kirigami as Kirigami

// qmllint disable Quick.controls-native-customize

T.ToolButton {
    id: control

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset, implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset, implicitContentHeight + topPadding + bottomPadding)
    verticalPadding: Kirigami.Units.smallSpacing
    horizontalPadding: Kirigami.Units.smallSpacing * 1.5
    spacing: Kirigami.Units.smallSpacing
    icon.width: Kirigami.Units.iconSizes.sizeForLabels
    icon.height: Kirigami.Units.iconSizes.sizeForLabels

    contentItem: IconLabel {
        spacing: control.spacing
        mirrored: control.mirrored
        display: control.display
        icon: control.icon
        text: control.text
        font: control.font
        color: Kirigami.Theme.textColor
    }
    background: Rectangle {
        implicitWidth: Kirigami.Units.iconSizes.sizeForLabels
        implicitHeight: Kirigami.Units.iconSizes.sizeForLabels
        radius: Kirigami.Units.cornerRadius
        color: Qt.alpha(Kirigami.Theme.textColor, control.down || control.checked ? 0.16 : 0.08)
        visible: control.down || control.checked || control.highlighted || control.visualFocus || (control.enabled && control.hovered)
    }
}
