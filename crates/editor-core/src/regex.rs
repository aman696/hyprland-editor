//! Bounded RE2 sample checks. User-facing untrusted work must run in the restricted worker.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub pattern: String,
    pub text: String,
    pub negated: bool,
}
unsafe extern "C" {
    fn hye_regex_match(
        pattern: *const libc::c_char,
        pattern_len: usize,
        text: *const libc::c_char,
        text_len: usize,
        error: *mut libc::c_char,
        error_len: usize,
    ) -> libc::c_int;
}
pub fn check(sample: &Sample) -> Result<bool, String> {
    if sample.pattern.len() > 16384 || sample.text.len() > 65536 || sample.pattern.contains('\0') {
        return Err(
            "Regex sample exceeds the pattern/text policy or contains a NUL pattern".into(),
        );
    }
    let mut error = [0 as libc::c_char; 512];
    let result = unsafe {
        hye_regex_match(
            sample.pattern.as_ptr().cast(),
            sample.pattern.len(),
            sample.text.as_ptr().cast(),
            sample.text.len(),
            error.as_mut_ptr(),
            error.len(),
        )
    };
    match result {
        0 => Ok(sample.negated),
        1 => Ok(!sample.negated),
        _ => {
            let bytes = error
                .iter()
                .take_while(|n| **n != 0)
                .map(|n| *n as u8)
                .collect::<Vec<_>>();
            Err(String::from_utf8_lossy(&bytes).into_owned())
        }
    }
}
pub fn literal(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if "\\.+*?()|[]{}^$".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
/// Exact rule text must not accidentally trigger Hyprland's negative: marker.
pub fn rule_literal(text: &str, negated: bool) -> String {
    let mut pattern = literal(text);
    if pattern.starts_with("negative:") {
        pattern.replace_range(0..1, "\\x6e");
    }
    if negated {
        format!("negative:{pattern}")
    } else {
        pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_match_negation_unicode_literal_escaping_and_bad_patterns_are_explicit() {
        for (text, pattern, expected) in [
            ("firefox", "fire", false),
            ("firefox", "fire.*", true),
            ("a+b[0]", &literal("a+b[0]"), true),
            ("δοκιμή", ".*", true),
        ] {
            assert_eq!(
                check(&Sample {
                    pattern: pattern.into(),
                    text: text.into(),
                    negated: false
                })
                .unwrap(),
                expected
            );
        }
        assert!(check(&Sample {
            pattern: "x".into(),
            text: "y".into(),
            negated: true
        })
        .unwrap());
        for text in ["negative:foo", "foo+bar[0]", "δοκιμή"] {
            for negated in [false, true] {
                let predicate = crate::rule::Predicate {
                    property: "class".into(),
                    value: crate::Value::String(rule_literal(text, negated)),
                };
                assert_eq!(
                    check(
                        &predicate
                            .regex_sample(crate::rule::Kind::Window, text)
                            .unwrap()
                    )
                    .unwrap(),
                    !negated
                );
            }
        }
        for p in ["[", "(?=x)", "(x)\\1"] {
            assert!(
                check(&Sample {
                    pattern: p.into(),
                    text: "x".into(),
                    negated: false
                })
                .is_err(),
                "{p}"
            );
        }
    }
}
