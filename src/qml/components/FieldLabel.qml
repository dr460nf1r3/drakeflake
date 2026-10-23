import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// Keep `visible` in sync with the field next to it: hidden items leave the grid.
Controls.Label {
    Layout.alignment: Qt.AlignLeft | Qt.AlignTop
    Layout.topMargin: Kirigami.Units.smallSpacing
    opacity: 0.75
}
