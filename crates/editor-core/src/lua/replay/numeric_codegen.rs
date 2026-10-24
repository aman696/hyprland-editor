//! Linked compiler observations and the supported Lua 5.4 constant descriptor.
//! This never executes Lua. Descriptor jump lists are distinct from model values.
use super::*;
use sha2::Digest;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZeroSubtraction {
    RewritesConstantIntegerZero,
    PreservesSubtraction,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericCodegen {
    pub probe_sha256: String,
    pub schema_sha256: String,
    pub zero_subtraction: Option<ZeroSubtraction>,
}
unsafe extern "C" {
    fn hye_lua54_subtract_probe(
        output: *mut u8,
        capacity: usize,
        written: *mut usize,
    ) -> libc::c_int;
}
pub(super) fn observe() -> Option<NumericCodegen> {
    let mut bytes = [0u8; 4096];
    let mut written = 0;
    // SAFETY: live bounded buffer. The adapter compiles/dumps a fixed literal
    // probe, never invokes it, retains no pointers and closes its bounded state.
    let status = unsafe { hye_lua54_subtract_probe(bytes.as_mut_ptr(), bytes.len(), &mut written) };
    if status != 0 || written == 0 || written > bytes.len() {
        return None;
    }
    let fingerprint = format!("{:x}", sha2::Sha256::digest(&bytes[..written]));
    let source = include_str!("../../../../../data/lua-numeric-codegen.json");
    let matrix: serde_json::Value =
        serde_json::from_str(source).expect("Bundled compiler code-generation profiles");
    let zero_subtraction = lookup(&matrix, &fingerprint);
    Some(NumericCodegen {
        probe_sha256: fingerprint,
        schema_sha256: source_hash(source),
        zero_subtraction,
    })
}
fn lookup(matrix: &serde_json::Value, fingerprint: &str) -> Option<ZeroSubtraction> {
    let mut modes = matrix["profiles"]
        .as_array()?
        .iter()
        .filter(|row| row["probe_sha256"] == fingerprint)
        .map(|row| serde_json::from_value::<ZeroSubtraction>(row["mode"].clone()).ok());
    let first = modes.next()??;
    modes
        .all(|next| next.as_ref() == Some(&first))
        .then_some(first)
}

#[derive(Clone, Copy)]
enum Kind {
    Nil,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String,
    Register,
}
#[derive(Clone, Copy)]
struct Descriptor {
    kind: Kind,
    true_jump: bool,
    false_jump: bool,
}
impl Descriptor {
    fn plain(kind: Kind) -> Self {
        Self {
            kind,
            true_jump: false,
            false_jump: false,
        }
    }
    fn numeric(self) -> Option<Kind> {
        (!self.true_jump
            && !self.false_jump
            && matches!(self.kind, Kind::Integer(_) | Kind::Float(_)))
        .then_some(self.kind)
    }
    fn true_constant(self) -> bool {
        matches!(
            self.kind,
            Kind::Boolean(true) | Kind::Integer(_) | Kind::Float(_) | Kind::String
        )
    }
    fn false_constant(self) -> bool {
        matches!(self.kind, Kind::Nil | Kind::Boolean(false))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    Budget,
    Unsupported,
}
/// Matches Lua's isKint after expression discharge, not whether an executed
/// expression happens to produce integer zero. All traversed work is reported.
pub(super) fn integer_zero(
    node: Node<'_>,
    source: &str,
) -> (std::result::Result<bool, Refusal>, usize) {
    let mut visits = 0;
    let result = descriptor(node, source, 0, &mut visits)
        .map(|d| matches!(d.kind, Kind::Integer(0)) && !d.true_jump && !d.false_jump);
    (result, visits)
}
fn descriptor(
    node: Node<'_>,
    source: &str,
    depth: usize,
    visits: &mut usize,
) -> std::result::Result<Descriptor, Refusal> {
    *visits += 1;
    if depth > 128 || *visits > 4096 {
        return Err(Refusal::Budget);
    }
    let child = |name| node.child_by_field_name(name).ok_or(Refusal::Unsupported);
    let plain = Descriptor::plain;
    let value = match node.kind() {
        "nil" => plain(Kind::Nil),
        "true" => plain(Kind::Boolean(true)),
        "false" => plain(Kind::Boolean(false)),
        "string" => plain(Kind::String),
        "number" => match crate::lua::evaluate(node, source, &BTreeMap::new(), 0) {
            Some(crate::lua::Expr::Int(v)) => plain(Kind::Integer(v)),
            Some(crate::lua::Expr::Float(v)) => plain(Kind::Float(v)),
            _ => return Err(Refusal::Unsupported),
        },
        "parenthesized_expression" => descriptor(
            *named(node).first().ok_or(Refusal::Unsupported)?,
            source,
            depth + 1,
            visits,
        )?,
        "identifier"
        | "function_call"
        | "function_definition"
        | "vararg_expression"
        | "table_constructor"
        | "dot_index_expression"
        | "bracket_index_expression" => plain(Kind::Register),
        "unary_expression" => {
            let operand = descriptor(child("operand")?, source, depth + 1, visits)?;
            match text(child("operator")?, source) {
                "not" => Descriptor {
                    kind: match operand.kind {
                        Kind::Nil | Kind::Boolean(false) => Kind::Boolean(true),
                        Kind::Boolean(true) | Kind::Integer(_) | Kind::Float(_) | Kind::String => {
                            Kind::Boolean(false)
                        }
                        Kind::Register => Kind::Register,
                    },
                    true_jump: operand.false_jump,
                    false_jump: operand.true_jump,
                },
                "-" => plain(match operand.numeric() {
                    Some(Kind::Integer(v)) => Kind::Integer(v.wrapping_neg()),
                    Some(Kind::Float(v)) if v != 0.0 && !v.is_nan() => Kind::Float(-v),
                    _ => Kind::Register,
                }),
                "#" => plain(Kind::Register),
                _ => return Err(Refusal::Unsupported),
            }
        }
        "binary_expression" => {
            let left = descriptor(child("left")?, source, depth + 1, visits)?;
            let right = descriptor(child("right")?, source, depth + 1, visits)?;
            let op = text(child("operator")?, source);
            if op == "and" {
                Descriptor {
                    kind: right.kind,
                    true_jump: right.true_jump,
                    false_jump: right.false_jump || left.false_jump || !left.true_constant(),
                }
            } else if op == "or" {
                Descriptor {
                    kind: right.kind,
                    true_jump: right.true_jump || left.true_jump || !left.false_constant(),
                    false_jump: right.false_jump,
                }
            } else {
                plain(fold(op, left.numeric(), right.numeric()))
            }
        }
        _ => return Err(Refusal::Unsupported),
    };
    Ok(value)
}
fn fold(op: &str, left: Option<Kind>, right: Option<Kind>) -> Kind {
    if let (Some(Kind::Integer(a)), Some(Kind::Integer(b))) = (left, right) {
        let integer = match op {
            "+" => Some(a.wrapping_add(b)),
            "-" => Some(a.wrapping_sub(b)),
            "*" => Some(a.wrapping_mul(b)),
            "//" | "%" if b != 0 => {
                let mut q = a.wrapping_div(b);
                if a.wrapping_rem(b) != 0 && (a < 0) != (b < 0) {
                    q = q.wrapping_sub(1);
                }
                Some(if op == "//" {
                    q
                } else {
                    a.wrapping_sub(q.wrapping_mul(b))
                })
            }
            _ => None,
        };
        if let Some(v) = integer {
            return Kind::Integer(v);
        }
    }
    let number = |kind| match kind {
        Some(Kind::Integer(v)) => Some(v as f64),
        Some(Kind::Float(v)) => Some(v),
        _ => None,
    };
    let (Some(a), Some(b)) = (number(left), number(right)) else {
        return Kind::Register;
    };
    let v = match op {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" if b != 0.0 => a / b,
        "//" if b != 0.0 => (a / b).floor(),
        "%" if b != 0.0 => a - (a / b).floor() * b,
        "^" => a.powf(b),
        _ => return Kind::Register,
    };
    if v == 0.0 || v.is_nan() {
        Kind::Register
    } else {
        Kind::Float(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn outer<'a>(node: Node<'a>, source: &str) -> Node<'a> {
        fn find<'a>(node: Node<'a>, source: &str) -> Option<Node<'a>> {
            if node.kind() == "binary_expression"
                && node
                    .child_by_field_name("left")
                    .is_some_and(|n| text(n, source) == "nz")
            {
                return Some(node);
            }
            named(node)
                .into_iter()
                .find_map(|child| find(child, source))
        }
        find(node, source).expect("fixture subtraction")
    }
    #[test]
    fn compiler_constants_follow_native_logical_jump_and_numeric_reference_cases() {
        let matrix: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../data/lua-numeric-fixtures.json"
        ))
        .unwrap();
        let profile = matrix["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["release"] == "5.4.9" && p["variant"] == "original")
            .unwrap();
        for (i, expression) in matrix["expressions"].as_array().unwrap().iter().enumerate() {
            let source = format!("return nz-({})", expression.as_str().unwrap());
            let tree = syntax_tree(&source).unwrap();
            let node = outer(tree.root_node(), &source)
                .child_by_field_name("right")
                .unwrap();
            let (constant, visits) = integer_zero(node, &source);
            let expected = profile["right_kinds"][i] == "integer"
                && profile["observations"][i] == "0000000000000000";
            assert_eq!(constant, Ok(expected), "{source}");
            assert!(visits > 0 && visits <= 4096);
        }
    }
    #[test]
    fn compiler_probe_lookup_refuses_unknown_or_conflicting_observations() {
        let source = include_str!("../../../../../data/lua-numeric-codegen.json");
        let matrix: serde_json::Value = serde_json::from_str(source).unwrap();
        assert_eq!(lookup(&matrix, &"0".repeat(64)), None);
        let fingerprint = matrix["profiles"][0]["probe_sha256"].as_str().unwrap();
        assert_eq!(
            lookup(&matrix, fingerprint),
            Some(ZeroSubtraction::RewritesConstantIntegerZero)
        );
        let conflict = serde_json::json!({"profiles":[{"probe_sha256":"same","mode":"rewrites_constant_integer_zero"},{"probe_sha256":"same","mode":"preserves_subtraction"}]});
        assert_eq!(lookup(&conflict, "same"), None);
        let p = compiler_profile().numeric_codegen.unwrap();
        assert_eq!(p.schema_sha256, source_hash(source));
        assert_eq!(p.zero_subtraction, lookup(&matrix, &p.probe_sha256));
    }
    #[test]
    fn compiler_constant_inspection_refuses_depth_and_unsupported_nodes() {
        let source = format!("return nz-({}0{})", "(".repeat(130), ")".repeat(130));
        let tree = syntax_tree(&source).unwrap();
        let node = outer(tree.root_node(), &source)
            .child_by_field_name("right")
            .unwrap();
        assert_eq!(integer_zero(node, &source).0, Err(Refusal::Budget));
        assert_eq!(
            integer_zero(tree.root_node(), &source).0,
            Err(Refusal::Unsupported)
        );
    }
}
