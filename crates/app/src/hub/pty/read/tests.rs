use super::*;

#[test]
fn split_utf8_waits_for_the_rest() {
    let mut bytes = "aé".as_bytes().to_vec();
    let last = bytes.pop().unwrap();

    assert_eq!(take_decoded(&mut bytes), Some("a".to_owned()));
    assert_eq!(bytes.len(), 1);
    bytes.push(last);
    assert_eq!(take_decoded(&mut bytes), Some("é".to_owned()));
    assert_eq!(take_decoded(&mut bytes), None);
}
