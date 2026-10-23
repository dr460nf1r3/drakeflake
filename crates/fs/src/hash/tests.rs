use super::{FNV_OFFSET, fnv1a, fnv1a_from};

#[test]
fn the_hash_is_standard_fnv1a() {
    // Keys are stored in caches: a different hash would rebuild every one.
    assert_eq!(fnv1a(b""), FNV_OFFSET);
    assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    assert_eq!(fnv1a_from(fnv1a(b"foo"), b"bar"), fnv1a(b"foobar"));
}
