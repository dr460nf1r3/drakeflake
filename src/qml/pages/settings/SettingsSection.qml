import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

GridLayout {
    required property real labelColumnWidth

    Layout.fillWidth: true
    columns: 2
    columnSpacing: Kirigami.Units.largeSpacing
    rowSpacing: Kirigami.Units.smallSpacing
}
