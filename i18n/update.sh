#!/bin/sh
# Collect the qsTr() strings from the QML files into the translation files.
# Run from the repository root. CI keeps drakeflake_en.ts up to date.
set -eu

lupdate src/qml -locations relative -no-obsolete -ts i18n/*.ts
