pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

Controls.AbstractButton {
    id: button

    padding: 0
    icon.width: Kirigami.Units.iconSizes.sizeForLabels
    icon.height: Kirigami.Units.iconSizes.sizeForLabels
    Accessible.name: button.text
    Controls.ToolTip.visible: button.hovered
    Controls.ToolTip.text: button.text
    background: null

    contentItem: Kirigami.Icon {
        implicitWidth: button.icon.width
        implicitHeight: button.icon.height
        source: button.icon.name || button.icon.source
        opacity: !button.enabled ? 0.4 : button.hovered || button.activeFocus ? 1.0 : 0.85
    }
}
