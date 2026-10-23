use super::*;

#[test]
fn mailbox_without_invoker_just_queues() {
    let mailbox: Mailbox<u32> = Mailbox::default();
    mailbox.post(1);
    mailbox.post(2);
    assert_eq!(mailbox.take(), vec![1, 2]);
    assert_eq!(mailbox.take(), Vec::<u32>::new());
}

#[test]
fn posts_after_detach_are_dropped() {
    let mailbox: Mailbox<u32> = Mailbox::default();
    mailbox.post(1);
    mailbox.detach();
    mailbox.post(2);
    assert_eq!(mailbox.take(), vec![1]);
}
