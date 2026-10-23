pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

Kirigami.SearchField {
    id: field

    property bool focusOnCreate: true
    property string menuObjectName: ""

    // A page that writes the field from outside must not overwrite text still waiting for its search.
    readonly property bool pending: debounce.running

    signal queryEdited(string query)
    signal queryChanged(string query)

    focusSequences: []
    autoAccept: false

    // textEdited (not textChanged): restoring a saved query when the page is created must not search again.
    onTextEdited: {
        field.queryEdited(field.text);
        debounce.restart();
    }
    // The clear button empties the text without textEdited and then emits accepted.
    onAccepted: {
        debounce.stop();
        field.queryEdited(field.text);
        field.queryChanged(field.text);
    }
    Component.onCompleted: {
        if (field.focusOnCreate)
            field.forceActiveFocus();
    }

    FieldMenu {
        objectName: field.menuObjectName
        target: field
    }

    Timer {
        id: debounce

        interval: 200

        onTriggered: field.queryChanged(field.text)
    }
}
