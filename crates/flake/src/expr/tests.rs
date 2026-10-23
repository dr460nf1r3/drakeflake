use super::*;

#[test]
fn expressions_have_no_placeholders_left() {
    let trees = [OptionTree::System, OptionTree::HomeManager("alice".into())];

    for tree in &trees {
        for template in [SNAPSHOT, SELECTED, OPTIONS_DOC] {
            let text = resolve(&tree.specialise(template).replace("__NAMES__", "[ ]"));
            assert!(!text.contains("__"), "{tree:?}: {text}");
        }
    }

    let home = OptionTree::HomeManager("alice".into()).specialise(SNAPSHOT);
    assert!(home.contains("cfgRoot = (sys.config.home-manager.users.${\"alice\"})"));
    assert!(resolve(VALUES).contains("valueAt w.l"));
}

#[test]
fn undocumented_marker_matches_the_walker() {
    assert!(OPTIONS_DOC.contains(UNDOCUMENTED));
}
