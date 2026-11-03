//! Ordinary finite JSON numbers stay compatible; other observations carry bits.
//! This preserves native inputs, never relaxes the finite editor-draft policy.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bits {
    bits: String,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Wire {
    Number(f64),
    Bits(Bits),
}
pub(super) fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    if value.is_finite() {
        value.serialize(serializer)
    } else {
        Bits {
            bits: format!("{:016x}", value.to_bits()),
        }
        .serialize(serializer)
    }
}
pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    match Wire::deserialize(deserializer)? {
        Wire::Number(value) if value.is_finite() => Ok(value),
        Wire::Bits(value)
            if value.bits.len() == 16 && value.bits.bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            let bits = u64::from_str_radix(&value.bits, 16).map_err(serde::de::Error::custom)?;
            Ok(f64::from_bits(bits))
        }
        _ => Err(serde::de::Error::custom("Invalid observed floating bits")),
    }
}
