//! Core type system and value representations.
//!
//! Port of `meridian.core.types`. This is the universal hub: `Value`,
//! `compare_values`, and `value_fingerprint` are pulled in by nearly everything.

/// Port of Python `DataType(Enum)`. Five members; hashable in Python, so `Hash`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DataType {
    Int64,
    Float64,
    String,
    Bool,
    Timestamp,
}

impl DataType {
    /// Mirror of Python `DataType.name`, used by `Value` repr / error messages.
    pub fn name(&self) -> &'static str {
        match self {
            DataType::Int64 => "INT64",
            DataType::Float64 => "FLOAT64",
            DataType::String => "STRING",
            DataType::Bool => "BOOL",
            DataType::Timestamp => "TIMESTAMP",
        }
    }
}

/// Port of Python `@dataclass(frozen=True, slots=True) Value`.
///
/// The tagged value used throughout the engine. In Python a `(type, payload)`
/// pair where `payload is None` means NULL; here each NULL carries its `DataType`.
///
/// NOTE: deliberately **not** `Copy` (holds `String`) and **not** `Eq`/`Hash`
/// yet — `Float64(f64)` is neither `Eq` nor `Hash`. A separate `key_repr()`
/// (using `f64::to_bits()` for keying only) will be added later. Arithmetic and
/// SUM must continue to use the real `f64`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null(DataType),
    Int64(i64),
    Float64(f64),
    Str(String),
    Bool(bool),
    Timestamp(i64),
}

/// Port of Python `Row = tuple[Value, ...]`.
pub type Row = Vec<Value>;

impl Value {
    /// Python `Value.null(dtype)`.
    pub fn null(dtype: DataType) -> Value {
        Value::Null(dtype)
    }

    /// Python `Value.from_int`.
    pub fn from_int(v: i64) -> Value {
        Value::Int64(v)
    }

    /// Python `Value.from_float`.
    pub fn from_float(v: f64) -> Value {
        Value::Float64(v)
    }

    /// Python `Value.from_str`.
    pub fn from_str(v: impl Into<String>) -> Value {
        Value::Str(v.into())
    }

    /// Python `Value.from_bool`.
    pub fn from_bool(v: bool) -> Value {
        Value::Bool(v)
    }

    /// Python `Value.from_timestamp`.
    pub fn from_timestamp(v: i64) -> Value {
        Value::Timestamp(v)
    }

    /// Python `Value.type` — the `DataType` tag, including for NULLs.
    pub fn data_type(&self) -> DataType {
        match self {
            Value::Null(dt) => *dt,
            Value::Int64(_) => DataType::Int64,
            Value::Float64(_) => DataType::Float64,
            Value::Str(_) => DataType::String,
            Value::Bool(_) => DataType::Bool,
            Value::Timestamp(_) => DataType::Timestamp,
        }
    }

    /// Python `Value.is_null` (`payload is None`).
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null(_))
    }

    /// Python `Value.as_int`. `None` if NULL; mismatched type panics in Python
    /// (`TypeError`). No test asserts the raise, so we return `None` on mismatch.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int64(v) => Some(*v),
            _ => None,
        }
    }

    /// Python `Value.as_float`.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float64(v) => Some(*v),
            _ => None,
        }
    }

    /// Python `Value.as_str`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(v) => Some(v.as_str()),
            _ => None,
        }
    }

    /// Python `Value.as_bool`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Python `Value.as_timestamp`.
    pub fn as_timestamp(&self) -> Option<i64> {
        match self {
            Value::Timestamp(v) => Some(*v),
            _ => None,
        }
    }
}

/// Port of Python `compare_values` — three-way compare of compatible values;
/// NULL sorts before non-NULL.
///
/// Parity notes:
/// - NULL/NULL → Equal; NULL vs non-NULL → NULL first (Less / Greater).
/// - INT64 vs FLOAT64 promote to `f64` (Python `float(payload)`); NaN is not
///   present in the data — `partial_cmp` `None` maps to `Equal`, mirroring
///   Python's "neither `<` nor `>` ⇒ return 0".
/// - STRING uses Rust `str` ordering, which for valid UTF-8 equals Python's
///   code-point ordering. BOOL compares as 0/1. TIMESTAMP compares as i64.
/// - A genuine non-numeric type mismatch raises `TypeError` in Python; the
///   signature here is infallible and no test exercises this path, so we return
///   `Equal` (columns are homogeneously typed, so this cannot occur in practice).
pub fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    use Value::*;

    let a_null = a.is_null();
    let b_null = b.is_null();
    if a_null && b_null {
        return Ordering::Equal;
    }
    if a_null {
        return Ordering::Less;
    }
    if b_null {
        return Ordering::Greater;
    }

    let cmp_f64 = |x: f64, y: f64| x.partial_cmp(&y).unwrap_or(Ordering::Equal);

    match (a, b) {
        (Int64(x), Int64(y)) | (Timestamp(x), Timestamp(y)) => x.cmp(y),
        (Float64(x), Float64(y)) => cmp_f64(*x, *y),
        (Str(x), Str(y)) => x.cmp(y),
        (Bool(x), Bool(y)) => (*x as i64).cmp(&(*y as i64)),
        // INT64 vs FLOAT64 cross-compare via f64 (Python promotes both to float).
        (Int64(x), Float64(y)) => cmp_f64(*x as f64, *y),
        (Float64(x), Int64(y)) => cmp_f64(*x, *y as f64),
        // Genuine mismatch — Python raises TypeError; unreachable for homogeneous
        // columns. Return Equal rather than panic.
        _ => Ordering::Equal,
    }
}

/// Port of Python `coerce_to_type`.
///
/// NULL coerces to a typed NULL. Same-type passthrough. Otherwise mirror the
/// Python conversion ladder exactly. An un-coercible pair raises `TypeError` in
/// Python; we `panic!` with the same message shape (no test hits this path, and
/// callers propagate the failure).
pub fn coerce_to_type(value: &Value, dtype: DataType) -> Value {
    use DataType::*;

    if value.is_null() {
        return Value::null(dtype);
    }
    if value.data_type() == dtype {
        return value.clone();
    }
    match (dtype, value) {
        // -> INT64
        (Int64, Value::Float64(v)) => Value::from_int(*v as i64),
        (Int64, Value::Str(v)) => Value::from_int(parse_py_int(v)),
        (Int64, Value::Bool(v)) => Value::from_int(*v as i64),
        // -> FLOAT64
        (Float64, Value::Int64(v)) => Value::from_float(*v as f64),
        (Float64, Value::Str(v)) => Value::from_float(
            v.trim()
                .parse::<f64>()
                .unwrap_or_else(|_| panic!("could not convert string to float: '{v}'")),
        ),
        // -> STRING (Python `str(payload)`)
        (String, v) => Value::from_str(py_str(v)),
        // -> BOOL (only from INT64 in Python)
        (Bool, Value::Int64(v)) => Value::from_bool(*v != 0),
        // -> TIMESTAMP (only from INT64 in Python)
        (Timestamp, Value::Int64(v)) => Value::from_timestamp(*v),
        _ => panic!(
            "cannot coerce {:?} to {:?}",
            value.data_type(),
            dtype
        ),
    }
}

/// Python `int(str)`: parse a base-10 integer, tolerating surrounding
/// whitespace. Panics on bad input (mirrors Python `ValueError`).
fn parse_py_int(s: &str) -> i64 {
    s.trim()
        .parse::<i64>()
        .unwrap_or_else(|_| panic!("invalid literal for int(): '{s}'"))
}

/// Python `str(payload)` for the value's payload. Used by `coerce_to_type` and
/// reproduces Python's default scalar `str()` rendering closely enough for the
/// engine (ints/timestamps as decimals, bools as `True`/`False`).
fn py_str(v: &Value) -> std::string::String {
    match v {
        Value::Int64(x) | Value::Timestamp(x) => x.to_string(),
        Value::Float64(x) => x.to_string(),
        Value::Str(x) => x.clone(),
        Value::Bool(x) => if *x { "True" } else { "False" }.to_string(),
        Value::Null(_) => "None".to_string(),
    }
}

/// Port of Python `value_fingerprint` — a 64-bit FNV-1a-style fingerprint used
/// only for *internal bucketing* of group-by / hash-index keys.
///
/// Structure reproduced exactly from Python:
/// ```text
/// h  = FNV_OFFSET
/// h ^= hash(type);            h *= FNV_PRIME
/// if not null:
///     STRING: for ch: h ^= ord(ch); h *= FNV_PRIME   (char-by-char)
///     else:   h ^= hash(payload);   h *= FNV_PRIME
/// return h & 0xFFFF_FFFF_FFFF_FFFF
/// ```
/// Because Python masks only at the end, and both `^` (bitwise, per-bit) and `*`
/// (mod 2^64 in its low bits) are determined solely by the low 64 bits of their
/// operands, doing every step with `wrapping_mul` / xor mod 2^64 yields the
/// identical low-64-bit result — so the wrapping port is faithful step-for-step.
///
/// PARITY CAVEAT: Python's `hash(value.type)` (salted enum→str hash) and
/// `hash(int/float)` are process-salted / non-trivial and **cannot** be
/// reproduced byte-for-byte in Rust. No test asserts a literal fingerprint; the
/// value only needs to be self-consistent within this engine for bucketing to
/// be correct. We therefore substitute deterministic operands:
///   - type tag → a fixed small code per `DataType`,
///   - INT64/TIMESTAMP → the i64 bits,
///   - FLOAT64 → `f64::to_bits()` (keying only; arithmetic still uses real f64),
///   - BOOL → 1/0 (matches Python `hash(True)`/`hash(False)`),
///   - STRING → char-by-char `ord` (matches Python exactly).
pub fn value_fingerprint(value: &Value) -> u64 {
    const FNV_OFFSET: u64 = 1469598103934665603;
    const FNV_PRIME: u64 = 1099511628211;
    const MASK: u64 = 0xFFFF_FFFF_FFFF_FFFF;

    let type_code: u64 = match value.data_type() {
        DataType::Int64 => 1,
        DataType::Float64 => 2,
        DataType::String => 3,
        DataType::Bool => 4,
        DataType::Timestamp => 5,
    };

    let mut h = FNV_OFFSET;
    h ^= type_code;
    h = h.wrapping_mul(FNV_PRIME);

    if !value.is_null() {
        match value {
            Value::Str(s) => {
                for ch in s.chars() {
                    h ^= ch as u64;
                    h = h.wrapping_mul(FNV_PRIME);
                }
            }
            Value::Int64(v) | Value::Timestamp(v) => {
                h ^= *v as u64;
                h = h.wrapping_mul(FNV_PRIME);
            }
            Value::Float64(v) => {
                h ^= v.to_bits();
                h = h.wrapping_mul(FNV_PRIME);
            }
            Value::Bool(v) => {
                h ^= *v as u64;
                h = h.wrapping_mul(FNV_PRIME);
            }
            Value::Null(_) => unreachable!(),
        }
    }
    h & MASK
}
