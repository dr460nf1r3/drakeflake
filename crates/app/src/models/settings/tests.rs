use super::SettingsModel;

#[test]
fn invalid_settings_are_refused_without_saving() {
    let mut model = SettingsModel::default();
    let layout = (model.layout_mode(), model.layout_rules());
    let level = model.log_level();

    assert_eq!(
        model.set_layout("weird".to_owned(), String::new()),
        "Unknown layout \"weird\""
    );
    let rules = model.set_layout(
        "area".to_owned(),
        "services = x.nix\nno rule here".to_owned(),
    );

    assert!(rules.starts_with("line 2: "), "{rules}");
    assert_eq!((model.layout_mode(), model.layout_rules()), layout);

    assert_eq!(
        model.set_tray_interval("check_hours".to_owned(), -1),
        "-1 is not allowed for check_hours"
    );
    model.set_log_level("loud".to_owned());
    assert_eq!(model.log_level(), level);

    assert_eq!(model.relay.system.take().len(), 0);
}
