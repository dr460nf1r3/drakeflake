//! FNV-1a, spelled out: cache keys must not change between builds, unlike
//! those of the standard library's hashers.

#[cfg(test)]
mod tests;

pub const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

const FNV_PRIME: u64 = 0x0100_0000_01b3;

#[must_use = "the hash is the only result"]
pub fn fnv1a(bytes: &[u8]) -> u64 {
    fnv1a_from(FNV_OFFSET, bytes)
}

#[must_use = "the hash is the only result"]
pub fn fnv1a_from(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}
