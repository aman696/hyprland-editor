use serde::{Deserialize, Serialize};
pub const PROTOCOL: &str = "hye-lua55-compiler-1";
pub const MAX_WIRE: usize = 4 * 1024 * 1024;
pub const MAX_SOURCE: usize = 1024 * 1024;
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: String,
    pub nonce: [u8; 32],
    pub expected_release: String,
    pub operation: Operation,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Profile {},
    Compile { source: Vec<u8>, load_name: Vec<u8> },
    Convert { values: Vec<Primitive> },
    Probe {},
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Primitive {
    Nil,
    Bool(bool),
    Int(i64),
    FloatBits(u64),
    Bytes(Vec<u8>),
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub release: String,
    pub library_identity: String,
    pub family: u16,
    pub numeric_locale: String,
    pub private_seed: String,
    pub global_declaration_compiles: bool,
    pub global_identifier_compiles: bool,
    pub imported_code_executed: bool,
    pub complete_language_verified: bool,
    pub selected_compositor_verified: bool,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conversion {
    pub flags: u32,
    pub integer: i64,
    pub number_bits: u64,
    pub coerced_integer: i64,
    pub text: Vec<u8>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    Profile {},
    Compiled {
        valid: bool,
        diagnostic: Vec<u8>,
    },
    Converted {
        values: Vec<Conversion>,
    },
    Probe {
        file_denied: bool,
        network_denied: bool,
        process_denied: bool,
        inherited_descriptor_closed: bool,
    },
    Error {
        reason: String,
    },
}
impl Outcome {
    pub fn error(reason: impl Into<String>) -> Self {
        Self::Error {
            reason: reason.into(),
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub protocol: String,
    pub request_sha256: String,
    pub nonce: Option<[u8; 32]>,
    pub profile: Option<Profile>,
    pub outcome: Outcome,
}
