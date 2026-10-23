# QML, models and icons

- Qt 6.11 conventions: `pragma ComponentBehavior: Bound`, qualified ids
  (`page.x`), typed properties and functions, interaction signals
  (`textEdited`, `activated`) for write-back.
- No `Kirigami.FormLayout`: it centres itself and scrolls to focus. Use a
  left-aligned `GridLayout` with `FieldLabel`.
- Never override FINAL properties (`state`, `baseline`, `action`). The component
  then fails to load.
- Models never borrow each other. Cross-model work goes through `hub::Mailbox`.
- Structured data is `serde_json::Value`. QML never parses or stringifies model
  data. QML numbers are floats, so pass integers that must stay integers as `i32`.
- Bump `API_VERSION` (`models/system.rs`) and `expectedApi` (`Main.qml`) when QML
  uses new slots or properties. An older binary then shows the rebuild banner.
- Icons are freedesktop action names that exist in Breeze. No `-symbolic`, and
  no names shared with app, place, mimetype or emblem icons (those render in
  colour). Own icons come from `scripts/make-icons.py`.
- File choosers use `PortalPicker` (xdg-desktop-portal via zbus).
