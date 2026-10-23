import QtQuick
import org.kde.kirigami as Kirigami
import org.kde.kirigami.templates as KT

KT.InlineMessage {
    id: root

    padding: Kirigami.Units.largeSpacing

    background: Rectangle {
        radius: Kirigami.Units.cornerRadius * 2
        color: {
            switch (root.type) {
            case Kirigami.MessageType.Positive:
                return Qt.alpha(Kirigami.Theme.positiveTextColor, 0.15);
            case Kirigami.MessageType.Warning:
                return Qt.alpha(Kirigami.Theme.neutralTextColor, 0.15);
            case Kirigami.MessageType.Error:
                return Qt.alpha(Kirigami.Theme.negativeTextColor, 0.15);
            default:
                return Kirigami.ColorUtils.linearInterpolation(Kirigami.Theme.backgroundColor, Kirigami.Theme.textColor, 0.06);
            }
        }
    }
}
