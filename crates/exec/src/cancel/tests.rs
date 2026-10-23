use super::*;

#[test]
fn cancelling_reaches_every_clone() {
    let token = CancelToken::new();
    let clone = token.clone();

    assert!(!clone.is_cancelled());
    token.cancel();
    assert!(clone.is_cancelled());
}

#[test]
fn a_parent_cancels_its_children_but_not_the_other_way() {
    let parent = CancelToken::new();
    let child = parent.child();
    let grandchild = child.child();

    grandchild.cancel();
    assert!(!child.is_cancelled() && !parent.is_cancelled());

    parent.cancel();
    assert!(child.is_cancelled());
}
