//! Source-pinned complex primary-value models. No setter/runtime authority.
use super::{Codec, NativeValue, Outcome, Parser, Rejection};
use crate::lua::replay::Observed;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vec2Bounds {
    pub minimum: [f32; 2],
    pub maximum: [f32; 2],
}
impl Vec2Bounds {
    pub(super) fn validate(&self) -> Result<(), String> {
        if (0..2).any(|i| {
            !self.minimum[i].is_finite()
                || !self.maximum[i].is_finite()
                || self.minimum[i] > self.maximum[i]
        }) {
            return Err("Invalid vector validator bounds".into());
        }
        Ok(())
    }
}
pub const FONT_WEIGHTS: [(&[u8], i64); 12] = [
    (b"thin".as_slice(), 100),
    (b"ultralight", 200),
    (b"light", 300),
    (b"semilight", 350),
    (b"book", 380),
    (b"normal", 400),
    (b"medium", 500),
    (b"semibold", 600),
    (b"bold", 700),
    (b"ultrabold", 800),
    (b"heavy", 900),
    (b"ultraheavy", 1000),
];
unsafe extern "C" {
    fn hye_complex_context_supported() -> libc::c_int;
    fn hye_complex_text_profile(output: *mut u8, capacity: usize) -> usize;
    fn hye_complex_text(
        kind: libc::c_int,
        input: *const u8,
        length: usize,
        output: *mut u32,
    ) -> libc::c_int;
}
pub fn text_profile() -> Option<String> {
    let mut buffer = [0u8; 160];
    // SAFETY: bounded writable buffer; no imported pointers or code.
    let n = unsafe { hye_complex_text_profile(buffer.as_mut_ptr(), buffer.len()) };
    if n == 0 || n >= buffer.len() {
        return None;
    }
    String::from_utf8(buffer[..n].to_vec()).ok()
}
fn unresolved(reason: &str) -> Outcome {
    Outcome::Unresolved {
        reason: reason.into(),
    }
}
fn rejected(error: Rejection) -> Outcome {
    Outcome::Rejected { error }
}
fn accepted(native: NativeValue) -> Outcome {
    let representation = native.representation();
    Outcome::Accepted {
        native,
        representation,
    }
}
fn text(kind: i32, bytes: &[u8]) -> Result<[u32; 2], Outcome> {
    let mut out = [0u32; 2];
    // SAFETY: valid immutable input and two writable u32 slots; the adapter
    // copies no more than those slots and retains no input pointer.
    let status = unsafe { hye_complex_text(kind, bytes.as_ptr(), bytes.len(), out.as_mut_ptr()) };
    match status {
        0 => Ok(out),
        1 => Err(rejected(Rejection::BadValue)),
        2 => Err(unresolved(
            "The original text conversion has undefined or unprofiled input behavior",
        )),
        _ => Err(unresolved(
            "Complex text conversion reached its bounded input/allocation limit",
        )),
    }
}
fn table(input: &Observed) -> Result<&[(Observed, Observed)], Outcome> {
    let Observed::Table(rows) = input else {
        return Err(rejected(Rejection::BadType));
    };
    if rows.len() > 4096 {
        return Err(unresolved("Complex table exceeds 4096 observed entries"));
    }
    Ok(rows)
}
fn field<'a>(rows: &'a [(Observed, Observed)], name: &[u8]) -> Result<&'a Observed, Outcome> {
    static NIL: Observed = Observed::Nil;
    let mut found = None;
    for (key, value) in rows {
        if matches!(key,Observed::Bytes(bytes) if bytes==name) {
            if found.is_some() {
                return Err(unresolved(
                    "Duplicate keys do not identify one native table field",
                ));
            }
            found = Some(value);
        }
    }
    Ok(found.unwrap_or(&NIL))
}
/// Only a dense positive-integer domain establishes one Lua table border from
/// a snapshot. Sparse tables need original VM layout/history, never a guessed
/// raw length from a reconstructed table or a count of rows.
fn array(rows: &[(Observed, Observed)]) -> Result<Vec<&Observed>, Outcome> {
    let mut values = std::collections::BTreeMap::new();
    for (key, value) in rows {
        let index = match key {
            Observed::Int(v) if *v > 0 => Some(*v),
            Observed::Float(v)
                if v.is_finite() && *v >= 1.0 && *v < 9223372036854775808.0 && v.fract() == 0.0 =>
            {
                Some(*v as i64)
            }
            _ => None,
        };
        if let Some(index) = index {
            if matches!(value, Observed::Nil) || values.insert(index, value).is_some() {
                return Err(unresolved("Invalid or duplicate native array observations"));
            }
        }
    }
    if values.keys().copied().ne(1..=values.len() as i64) {
        return Err(unresolved(
            "Sparse native table length requires its VM layout/history",
        ));
    }
    Ok(values.into_values().collect())
}
fn cast_integer(number: f64) -> Result<i64, Outcome> {
    if !number.is_finite() || !(-9223372036854775808.0..9223372036854775808.0).contains(&number) {
        return Err(unresolved(
            "The original floating-to-integer cast has no defined result for this value",
        ));
    }
    Ok(number as i64)
}
fn inner(parser: &mut Parser, codec: &Codec, input: &Observed) -> Result<NativeValue, Outcome> {
    match codec {
        Codec::Color => {
            let v = parser.convert(input).map_err(|e| unresolved(&e))?;
            if v.flags & 4 == 0 {
                return Err(rejected(Rejection::BadType));
            }
            Ok(NativeValue::Color(text(1, &parser.text[..v.length])?[0]))
        }
        Codec::FontWeight => {
            let v = parser.convert(input).map_err(|e| unresolved(&e))?;
            let value = if v.flags & 2 != 0 {
                cast_integer(v.number)?
            } else if v.flags & 4 != 0 {
                let bytes = &parser.text[..v.length];
                if !bytes.is_ascii() {
                    return Err(unresolved(
                        "Non-ASCII case folding has unprofiled native ctype behavior",
                    ));
                }
                let name = bytes.to_ascii_lowercase();
                FONT_WEIGHTS
                    .into_iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, v)| v)
                    .ok_or_else(|| rejected(Rejection::BadValue))?
            } else {
                return Err(rejected(Rejection::BadType));
            };
            if value < 0 {
                return Err(rejected(Rejection::OutOfRange));
            }
            Ok(NativeValue::FontWeight(value))
        }
        Codec::CssGap { minimum, maximum } => {
            let v = parser.convert(input).map_err(|e| unresolved(&e))?;
            let range = |n: i64| {
                if minimum.is_some_and(|m| n < m) || maximum.is_some_and(|m| n > m) {
                    Err(rejected(Rejection::OutOfRange))
                } else {
                    Ok(n)
                }
            };
            if v.flags & 2 != 0 {
                return Ok(NativeValue::CssGap([range(cast_integer(v.number)?)?; 4]));
            }
            let rows = table(input)?;
            let mut values = [0; 4];
            for (index, name) in [b"top".as_slice(), b"right", b"bottom", b"left"]
                .into_iter()
                .enumerate()
            {
                let value = field(rows, name)?;
                if matches!(value, Observed::Nil) {
                    continue;
                }
                let v = parser.convert(value).map_err(|e| unresolved(&e))?;
                if v.flags & 2 == 0 {
                    return Err(rejected(Rejection::BadType));
                }
                values[index] = range(v.coerced_integer)?;
            }
            Ok(NativeValue::CssGap(values))
        }
        Codec::Vec2 { validator } => {
            let v = parser.convert(input).map_err(|e| unresolved(&e))?;
            let bits = if v.flags & 4 != 0 {
                text(2, &parser.text[..v.length])?
            } else {
                let values = array(table(input)?)?;
                if values.len() != 2 {
                    return Err(rejected(Rejection::BadValue));
                }
                let mut bits = [0; 2];
                for (index, value) in values.into_iter().enumerate() {
                    let v = parser.convert(value).map_err(|e| unresolved(&e))?;
                    if v.flags & 2 == 0 {
                        return Err(rejected(Rejection::BadType));
                    }
                    bits[index] = (v.number as f32).to_bits();
                }
                bits
            };
            if validator.as_ref().is_some_and(|bounds| {
                (0..2).any(|i| {
                    f32::from_bits(bits[i]) < bounds.minimum[i]
                        || f32::from_bits(bits[i]) > bounds.maximum[i]
                })
            }) {
                return Err(rejected(Rejection::BadValue));
            }
            Ok(NativeValue::Vec2Bits(bits))
        }
        Codec::Gradient => {
            let v = parser.convert(input).map_err(|e| unresolved(&e))?;
            if v.flags & 4 != 0 {
                return Ok(NativeValue::Gradient {
                    colors: vec![text(1, &parser.text[..v.length])?[0]],
                    angle_bits: 0,
                });
            }
            let rows = table(input)?;
            let colors = field(rows, b"colors")?;
            if !matches!(colors, Observed::Table(_)) {
                return Err(rejected(Rejection::BadValue));
            }
            let color_values = array(table(colors)?)?;
            if color_values.is_empty() {
                return Err(rejected(Rejection::BadValue));
            }
            let mut colors = vec![];
            for value in color_values {
                let v = parser.convert(value).map_err(|e| unresolved(&e))?;
                if v.flags & 4 == 0 {
                    return Err(rejected(Rejection::BadType));
                }
                colors.push(text(1, &parser.text[..v.length])?[0]);
            }
            let angle = field(rows, b"angle")?;
            let angle_bits = if matches!(angle, Observed::Nil) {
                0
            } else {
                let v = parser.convert(angle).map_err(|e| unresolved(&e))?;
                if v.flags & 2 == 0 {
                    return Err(rejected(Rejection::BadType));
                }
                ((v.number * std::f64::consts::PI / 180.0) as f32).to_bits()
            };
            Ok(NativeValue::Gradient { colors, angle_bits })
        }
        _ => Err(unresolved("Missing complex codec")),
    }
}
pub(super) fn decode(
    parser: &mut Parser,
    codec: &Codec,
    input: &Observed,
) -> Result<Option<Outcome>, String> {
    if !matches!(
        codec,
        Codec::Color
            | Codec::Gradient
            | Codec::Vec2 { .. }
            | Codec::CssGap { .. }
            | Codec::FontWeight
    ) {
        return Ok(None);
    }
    // SAFETY: read-only process locale/rounding queries with no input pointers.
    if text_profile().is_none() || unsafe { hye_complex_context_supported() } != 1 {
        return Ok(Some(unresolved(
            "Complex text decoding requires the recorded libstdc++ classic-locale profile",
        )));
    }
    Ok(Some(match inner(parser, codec, input) {
        Ok(native) => accepted(native),
        Err(outcome) => outcome,
    }))
}
