import QtQuick
import org.kde.kirigami as Kirigami

Kirigami.PlaceholderMessage {
    id: placeholder

    required property var functions

    icon.name: placeholder.functions.state === "missing" || placeholder.functions.state === "error" ? "data-error" : "search"
    text: {
        switch (placeholder.functions.state) {
        case "missing":
            return qsTr("The function index is not installed");
        case "error":
            return qsTr("Could not read the function index");
        case "ready":
            return qsTr("No matching functions");
        default:
            return qsTr("Reading the function index…");
        }
    }
    explanation: {
        switch (placeholder.functions.state) {
        case "missing":
            return qsTr("The package builds it. cargo build does not: in a development build, run scripts/function-index.sh once. Expected at: %1").arg(placeholder.functions.error);
        case "error":
            return placeholder.functions.error;
        case "ready":
            return qsTr("Try another name, a word from the description or a type such as “string -> list”. The filters above may hide what you look for.");
        default:
            return "";
        }
    }
}
