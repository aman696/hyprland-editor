//! Pinned primitive parser semantics, separate from registration and edit authority.
//! Product consumers must run this native conversion inside the restricted worker.
use super::replay::{compiler_profile, Observed};
use crate::Value;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, ptr::NonNull};
pub mod complex;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedInteger {
    pub name: Vec<u8>,
    pub value: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StringChoices {
    pub choices: Vec<Vec<u8>>,
}
/// Existing opaque validator names keep their wire spelling. Known closed
/// choice sets are explicit data, never user-provided code or callbacks.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringValidator {
    Unsupported(String),
    Choices(StringChoices),
}
impl From<&str> for StringValidator {
    fn from(name: &str) -> Self {
        Self::Unsupported(name.into())
    }
}
impl From<String> for StringValidator {
    fn from(name: String) -> Self {
        Self::Unsupported(name)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub codec: Codec,
    pub input: Observed,
}

/// One bounded parser-worker batch, never a transaction or source import.
pub fn decode_batch(cases: &[Case]) -> Result<Vec<Outcome>, String> {
    if cases.is_empty() || cases.len() > 256 {
        return Err("Primitive decoding requires 1–256 cases".into());
    }
    struct Count(usize);
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            if self.0 > 2 * crate::MAX_DOCUMENT_BYTES {
                return Err(std::io::Error::other("Primitive output budget exceeded"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut parser = Parser::new()?;
    let mut count = Count(0);
    let mut outcomes = Vec::new();
    for case in cases {
        let outcome = parser.decode(&case.codec, &case.input)?;
        serde_json::to_writer(&mut count, &outcome)
            .map_err(|_| "Primitive decoding output exceeds 2 MiB".to_string())?;
        outcomes.push(outcome);
    }
    Ok(outcomes)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Codec {
    Bool,
    Color,
    Gradient,
    Vec2 {
        validator: Option<complex::Vec2Bounds>,
    },
    CssGap {
        minimum: Option<i64>,
        maximum: Option<i64>,
    },
    FontWeight,
    Int {
        minimum: Option<i64>,
        maximum: Option<i64>,
        names: Option<Vec<NamedInteger>>,
    },
    Float {
        #[serde(with = "finite_bound")]
        minimum: Option<f32>,
        #[serde(with = "finite_bound")]
        maximum: Option<f32>,
    },
    String {
        /// An unimplemented native validator prevents acceptance being inferred.
        validator: Option<StringValidator>,
    },
}
mod finite_bound {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub(super) fn serialize<S: Serializer>(
        value: &Option<f32>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if value.is_some_and(|v| !v.is_finite()) {
            return Err(serde::ser::Error::custom("Nonfinite primitive bound"));
        }
        value.serialize(serializer)
    }
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<f32>, D::Error> {
        let value = Option::<f32>::deserialize(deserializer)?;
        if value.is_some_and(|v| !v.is_finite()) {
            return Err(serde::de::Error::custom("Nonfinite primitive bound"));
        }
        Ok(value)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rejection {
    OutOfRange,
    BadValue,
    BadType,
}
impl Rejection {
    pub fn native_code(self) -> u8 {
        match self {
            Self::OutOfRange => 1,
            Self::BadValue => 2,
            Self::BadType => 3,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum NativeValue {
    Bool(bool),
    Int(i64),
    /// Native Config::FLOAT storage, including nonfinite values, without JSON NaN.
    FloatBits(u32),
    /// C-string bytes after the native parser's NUL truncation.
    Bytes(Vec<u8>),
    Color(u32),
    Gradient {
        colors: Vec<u32>,
        angle_bits: u32,
    },
    Vec2Bits([u32; 2]),
    CssGap([i64; 4]),
    FontWeight(i64),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Representation {
    Value { value: Value },
    Unavailable { reason: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    Accepted {
        native: NativeValue,
        /// A representable draft value is not an ownership or editing capability.
        representation: Representation,
    },
    Rejected {
        error: Rejection,
    },
    Unresolved {
        reason: String,
    },
}
impl NativeValue {
    fn representation(&self) -> Representation {
        let value = match self {
            Self::Bool(v) => Value::Bool(*v),
            Self::Int(v) => Value::Int(*v),
            Self::FloatBits(bits) => {
                let v = f32::from_bits(*bits);
                if !v.is_finite() {
                    return Representation::Unavailable {
                        reason: "The native parser accepts this nonfinite number; editor drafts require finite values".into(),
                    };
                }
                Value::Float(f64::from(v))
            }
            Self::Bytes(bytes) => {
                if bytes.len() > 16384 {
                    return Representation::Unavailable {
                        reason: "The native string exceeds the editor draft's 16 KiB text limit"
                            .into(),
                    };
                }
                let Ok(text) = std::str::from_utf8(bytes) else {
                    return Representation::Unavailable {
                        reason: "The native string is not UTF-8; no replacement text is inferred"
                            .into(),
                    };
                };
                Value::String(text.into())
            }
            Self::Color(v) => Value::Color(*v),
            Self::FontWeight(v) if *v >= 0 => Value::FontWeight(*v),
            Self::CssGap(v) => Value::CssGap(*v),
            Self::Vec2Bits(bits) if bits.iter().all(|b| f32::from_bits(*b).is_finite()) => {
                Value::Vec2(bits.map(|b| f64::from(f32::from_bits(b))))
            }
            Self::Gradient { colors, angle_bits }
                if !colors.is_empty()
                    && colors.len() <= 32
                    && f32::from_bits(*angle_bits).is_finite() =>
            {
                Value::Gradient {
                    colors: colors.clone(),
                    angle: f64::from(f32::from_bits(*angle_bits)) * 180.0 / std::f64::consts::PI,
                }
            }
            _ => {
                return Representation::Unavailable {
                    reason:
                        "The native complex value exceeds the editable finite-value representation"
                            .into(),
                }
            }
        };
        Representation::Value { value }
    }
}
impl Codec {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::CssGap { minimum, maximum }
                if minimum.zip(*maximum).is_some_and(|(a, b)| a > b) =>
            {
                return Err("Reversed gap codec bounds".into())
            }
            Self::Vec2 {
                validator: Some(bounds),
            } => bounds.validate()?,
            Self::Int {
                minimum,
                maximum,
                names,
            } => {
                if minimum.zip(*maximum).is_some_and(|(a, b)| a > b) {
                    return Err("Reversed integer codec bounds".into());
                }
                if let Some(names) = names {
                    let mut keys = BTreeSet::new();
                    let mut bytes = 0usize;
                    if names.len() > 512 {
                        return Err("Integer name map exceeds 512 entries".into());
                    }
                    for entry in names {
                        bytes = bytes.saturating_add(entry.name.len());
                        if entry.name.len() > 4096
                            || bytes > 65536
                            || entry.name.contains(&0)
                            || !keys.insert(&entry.name)
                        {
                            return Err("Invalid, duplicated or oversized integer name map".into());
                        }
                    }
                }
            }
            Self::Float { minimum, maximum } => {
                if minimum.iter().chain(maximum).any(|v| !v.is_finite())
                    || minimum.zip(*maximum).is_some_and(|(a, b)| a > b)
                {
                    return Err("Invalid floating codec bounds".into());
                }
            }
            Self::String {
                validator: Some(StringValidator::Unsupported(name)),
            } if name.len() > 4096 => return Err("Validator identity exceeds 4 KiB".into()),
            Self::String {
                validator: Some(StringValidator::Choices(set)),
            } => {
                let mut keys = BTreeSet::new();
                let mut bytes = 0usize;
                if set.choices.is_empty() || set.choices.len() > 256 {
                    return Err("String choice count exceeds its bounds".into());
                }
                for choice in &set.choices {
                    bytes = bytes.saturating_add(choice.len());
                    if choice.len() > 4096 || bytes > 65536 || !keys.insert(choice) {
                        return Err("Duplicated or oversized string choice set".into());
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
#[derive(Default)]
#[repr(C)]
struct Converted {
    flags: u32,
    integer: i64,
    number: f64,
    length: usize,
    coerced_integer: i64,
}
unsafe extern "C" {
    fn hye_lua54_numeric_locale_c() -> libc::c_int;
    fn hye_lua54_coercions_new() -> *mut libc::c_void;
    fn hye_lua54_coercions_close(pointer: *mut libc::c_void);
    fn hye_lua54_convert(
        pointer: *mut libc::c_void,
        kind: libc::c_int,
        integer: i64,
        number: f64,
        bytes: *const u8,
        length: usize,
        result: *mut Converted,
        text: *mut u8,
        capacity: usize,
    ) -> libc::c_int;
}
/// Reused private C-API state with an 8 MiB allocator. No Lua libraries, user
/// bytecode, callbacks, metatables, hooks or objects enter this state.
pub struct Parser {
    pointer: NonNull<libc::c_void>,
    text: Vec<u8>,
}
impl Drop for Parser {
    fn drop(&mut self) {
        // SAFETY: this uniquely owned pointer came from the matching constructor.
        unsafe {
            hye_lua54_coercions_close(self.pointer.as_ptr());
        }
    }
}
impl Parser {
    pub fn new() -> Result<Self, String> {
        if !compiler_profile().primitive_conversions_profiled {
            return Err(
                "Primitive conversion requires a recorded matching Lua library profile".into(),
            );
        }
        // SAFETY: queries only the current process's numeric locale, without
        // changing it. Production runs before GUI initialization in a worker.
        if unsafe { hye_lua54_numeric_locale_c() } != 1 {
            return Err("Primitive conversion requires the profiled C numeric locale".into());
        }
        // SAFETY: no input pointers; constructor creates a new independent state.
        let pointer = NonNull::new(unsafe { hye_lua54_coercions_new() })
            .ok_or("Primitive conversion allocation budget exceeded")?;
        Ok(Self {
            pointer,
            text: vec![0; 65536],
        })
    }
    fn convert(&mut self, input: &Observed) -> Result<Converted, String> {
        let (kind, integer, number, bytes): (_, _, _, &[u8]) = match input {
            Observed::Bool(v) => (1, i64::from(*v), 0.0, &[]),
            Observed::Int(v) => (2, *v, 0.0, &[]),
            Observed::Float(v) => (3, 0, *v, &[]),
            Observed::Bytes(v) => (4, 0, 0.0, v),
            _ => return Ok(Converted::default()),
        };
        if bytes.len() > 65536 {
            return Err("Primitive conversion text exceeds 64 KiB".into());
        }
        let mut result = Converted::default();
        // SAFETY: all pointers are valid for their lengths and remain alive.
        // The private adapter copies at most text.len() bytes, retains no input,
        // protects allocating C-API operations and clears its stack on return.
        let status = unsafe {
            hye_lua54_convert(
                self.pointer.as_ptr(),
                kind,
                integer,
                number,
                bytes.as_ptr(),
                bytes.len(),
                &mut result,
                self.text.as_mut_ptr(),
                self.text.len(),
            )
        };
        if status != 0 || result.length > self.text.len() {
            return Err("Primitive conversion allocation/output budget exceeded".into());
        }
        Ok(result)
    }
    pub fn decode(&mut self, codec: &Codec, input: &Observed) -> Result<Outcome, String> {
        codec.validate()?;
        if let Some(outcome) = complex::decode(self, codec, input)? {
            return Ok(outcome);
        }
        let v = self.convert(input)?;
        let rejected = |error| Ok(Outcome::Rejected { error });
        let native = match codec {
            Codec::Bool => {
                if v.flags & 2 != 0 {
                    if v.number != 0.0 && v.number != 1.0 {
                        return rejected(Rejection::BadType);
                    }
                    NativeValue::Bool(v.number != 0.0)
                } else if v.flags & 8 != 0 {
                    NativeValue::Bool(v.integer != 0)
                } else {
                    return rejected(Rejection::BadType);
                }
            }
            Codec::Int {
                minimum,
                maximum,
                names,
            } => {
                if v.flags & (8 | 1) != 0 {
                    if minimum.is_some_and(|n| v.integer < n)
                        || maximum.is_some_and(|n| v.integer > n)
                    {
                        return rejected(Rejection::OutOfRange);
                    }
                    NativeValue::Int(v.integer)
                } else if v.flags & 4 != 0 {
                    let Some(names) = names else {
                        return rejected(Rejection::BadType);
                    };
                    let Some(entry) = names.iter().find(|e| e.name == self.text[..v.length]) else {
                        return rejected(Rejection::BadValue);
                    };
                    // The original named branch intentionally skips numeric bounds.
                    NativeValue::Int(entry.value)
                } else {
                    return rejected(Rejection::BadType);
                }
            }
            Codec::Float { minimum, maximum } => {
                let data = if v.flags & (8 | 1) != 0 {
                    // C++ compares INTEGER/boolean to FLOAT bounds in binary32.
                    let data = v.integer as f32;
                    if minimum.is_some_and(|n| data < n) || maximum.is_some_and(|n| data > n) {
                        return rejected(Rejection::OutOfRange);
                    }
                    data
                } else if v.flags & 2 != 0 {
                    // Lua numbers/numeric strings compare in binary64 first.
                    if minimum.is_some_and(|n| v.number < f64::from(n))
                        || maximum.is_some_and(|n| v.number > f64::from(n))
                    {
                        return rejected(Rejection::OutOfRange);
                    }
                    v.number as f32
                } else {
                    return rejected(Rejection::BadType);
                };
                NativeValue::FloatBits(data.to_bits())
            }
            Codec::String { validator } => {
                if v.flags & 4 == 0 {
                    return rejected(Rejection::BadType);
                }
                if let Some(name) = validator {
                    match name {
                        StringValidator::Unsupported(name) => {
                            return Ok(Outcome::Unresolved {
                                reason: format!(
                                    "Native string validator is not implemented: {name}"
                                ),
                            })
                        }
                        StringValidator::Choices(set)
                            if !set.choices.iter().any(|c| c == &self.text[..v.length]) =>
                        {
                            return rejected(Rejection::BadValue)
                        }
                        StringValidator::Choices(_) => {}
                    }
                }
                NativeValue::Bytes(self.text[..v.length].into())
            }
            _ => return Err("Complex codec dispatch is missing".into()),
        };
        let representation = native.representation();
        Ok(Outcome::Accepted {
            native,
            representation,
        })
    }
}
