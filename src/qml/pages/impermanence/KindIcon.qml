import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.Icon {
    id: icon

    required property string kind

    source: icon.kind === "dir" ? "document-open-folder" : "document-open"
    Layout.preferredWidth: Kirigami.Units.iconSizes.small
    Layout.preferredHeight: Kirigami.Units.iconSizes.small
}
