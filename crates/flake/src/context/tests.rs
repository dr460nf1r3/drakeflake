use drakeflake_exec::testing::ScriptedRunner;

use super::*;

#[test]
fn a_child_is_cancelled_with_its_parent_and_keeps_the_priority() {
    let (_, exec) = ScriptedRunner::answering(Vec::new());
    let parent = Nix::new(exec).with_priority(Priority::urgent());
    let child = parent.child();

    child.cancel();
    assert!(!parent.is_cancelled());
    assert!(child.priority().is_urgent());

    let second = parent.child();
    parent.cancel();
    assert!(second.is_cancelled());
}

#[test]
fn a_replaced_priority_leaves_the_original_alone() {
    let normal = Nix::system();
    let urgent = normal.with_priority(Priority::urgent());

    assert!(urgent.priority().is_urgent());
    assert!(!normal.priority().is_urgent());
}
