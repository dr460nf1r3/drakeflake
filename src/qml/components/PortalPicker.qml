pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Dialogs as Dialogs

// Asks the desktop portal through the setup model because Qt Quick lacks a platform file dialog.
Item {
    id: picker

    required property var setupModel
    required property string purpose
    property string title: ""
    property bool folder: true
    property string suggestedName: ""
    property string startFolder: ""
    property var nameFilters: [qsTr("Nix files (*.nix)")]
    property string defaultSuffix: "nix"
    property int lastSerial: 0

    signal picked(string path)

    function open(): void {
        if (picker.folder)
            picker.setupModel.pick_folder(picker.purpose, picker.title, picker.startFolder);
        else
            picker.setupModel.pick_save_file(picker.purpose, picker.title, picker.startFolder, picker.suggestedName);
    }

    function answer(): var {
        return picker.setupModel.picked;
    }

    function localPath(fileUrl: url): string {
        return decodeURIComponent(fileUrl.toString().replace(/^file:\/\//, ""));
    }

    function take(): void {
        const reply = picker.answer();

        if (!reply || reply.serial === picker.lastSerial || reply.purpose !== picker.purpose)
            return;

        picker.lastSerial = reply.serial;

        if (reply.error)
            (picker.folder ? folderFallback : fileFallback).open();
        else if (reply.path)
            picker.picked(reply.path);
    }

    // Answers given before this picker existed are not for it.
    Component.onCompleted: {
        const reply = picker.answer();
        picker.lastSerial = reply ? reply.serial : 0;
    }

    Connections {
        function onState_changed(): void {
            picker.take();
        }

        target: picker.setupModel
    }

    Dialogs.FolderDialog {
        id: folderFallback

        title: picker.title
        currentFolder: "file://" + picker.startFolder

        onAccepted: picker.picked(picker.localPath(folderFallback.selectedFolder))
    }

    Dialogs.FileDialog {
        id: fileFallback

        title: picker.title
        fileMode: Dialogs.FileDialog.SaveFile
        nameFilters: picker.nameFilters
        defaultSuffix: picker.defaultSuffix
        currentFolder: "file://" + picker.startFolder

        onAccepted: picker.picked(picker.localPath(fileFallback.selectedFile))
    }
}
