//! Utility hash functions — intentionally varied for optimization practice.
//!
//! Port of `meridian.util.hash`. W1 warning: reproduces `fnv1a64`,
//! `murmurish_mix`, `hash_combine`, and `partition_hash` EXACTLY using wrapping
//! arithmetic.
//!
//! Parity reasoning for the unmasked Python ops (`hash_combine`, `hash_row_key`):
//! Python ints are unbounded, so `((a << 5) + a) ^ b` can exceed 64 bits. But
//! every consumer masks the result down (`& 0xFFFF` in hash/composite indexes,
//! `& 0xFFFFFF` in group-by). Under `<<`, `+`, and `^`, each output bit depends
//! only on input bits at or below its position (carries propagate upward only),
//! so the low 64 bits computed with `wrapping_shl`/`wrapping_add`/`^` equal the
//! low 64 bits of the unbounded Python value — hence the masked buckets match.

use crate::core::errors::{MeridianError, Result};
use crate::core::types::{value_fingerprint, Value};

/// Python `hash_value` — alias for `value_fingerprint`.
pub fn hash_value(value: &Value) -> u64 {
    value_fingerprint(value)
}

/// Python `hash_combine` — `((a << 5) + a) ^ b`, low-64-bit faithful.
pub fn hash_combine(a: u64, b: u64) -> u64 {
    (a.wrapping_shl(5).wrapping_add(a)) ^ b
}

/// Python `hash_row_key` — fold `hash_combine` over the row's value fingerprints.
pub fn hash_row_key(values: &[Value]) -> u64 {
    let mut h: u64 = 0;
    for v in values {
        h = hash_combine(h, hash_value(v));
    }
    h
}

/// Python `fnv1a64` over raw bytes. Masked each step in Python; `wrapping_mul`
/// reproduces it exactly.
pub fn fnv1a64(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 1469598103934665603;
    const FNV_PRIME: u64 = 1099511628211;
    let mut h = FNV_OFFSET;
    for &byte in data {
        h ^= byte as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Python `fnv1a64_str` — `fnv1a64(text.encode("utf-8"))`.
pub fn fnv1a64_str(text: &str) -> u64 {
    fnv1a64(text.as_bytes())
}

/// Python `murmurish_mix` — fmix64-style finalizer; masks reproduced via u64
/// wrapping.
pub fn murmurish_mix(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xFF51AFD7ED558CCD);
    k ^= k >> 33;
    k = k.wrapping_mul(0xC4CEB9FE1A85EC53);
    k ^= k >> 33;
    k
}

/// Python `partition_hash` — `hash_value(value) % buckets`. `buckets <= 0`
/// raises `ValueError`; here `buckets` is unsigned, so we reject 0.
pub fn partition_hash(value: &Value, buckets: u64) -> Result<u64> {
    if buckets == 0 {
        return Err(MeridianError::Other("buckets must be positive".to_string()));
    }
    Ok(hash_value(value) % buckets)
}
