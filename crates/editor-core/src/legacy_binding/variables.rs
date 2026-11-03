//! Bounded source-defined local values. Ambient variables are deliberately unknown.
use crate::legacy_syntax::unescape;
use std::collections::BTreeMap;
const MAX_NAMES: usize = 1024;
const MAX_VALUE: usize = 65536;
const MAX_RETAINED: usize = 262144;
#[derive(Default)]
pub(super) struct Locals {
    values: BTreeMap<String, Option<String>>,
    retained: usize,
    unsupported_names: bool,
}
fn identifier(name: &str) -> bool {
    !name.is_empty() && name.len() <= 256 && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}
impl Locals {
    pub fn invalidate(&mut self) {
        self.values.clear();
        self.retained = 0;
    }
    pub fn define(&mut self, key: &str, value: &str, editable: bool) {
        let Some(name) = key.strip_prefix('$') else {
            return;
        };
        if !identifier(name) {
            // Native names may contain punctuation and shadow a larger piece
            // of another reference. Do not invent a partial interpretation.
            self.unsupported_names = true;
            self.invalidate();
            return;
        }
        let resolved = editable
            .then(|| self.expand(value).ok())
            .flatten()
            .filter(|(value, _)| unescape(value).is_ok())
            .map(|(value, _)| value);
        if let Some(old) = self.values.remove(name).flatten() {
            self.retained -= old.len();
        }
        if self.values.len() >= MAX_NAMES {
            return;
        }
        let resolved = resolved.filter(|value| self.retained + value.len() <= MAX_RETAINED);
        if let Some(value) = &resolved {
            self.retained += value.len();
        }
        self.values.insert(name.into(), resolved);
    }
    pub fn expand(&self, text: &str) -> Result<(String, bool), String> {
        if text.len() > MAX_VALUE {
            return Err("Local variable input exceeds 64 KiB".into());
        }
        let mut output = String::new();
        let mut at = 0;
        let mut changed = false;
        while at < text.len() {
            let ch = text[at..].chars().next().unwrap();
            at += ch.len_utf8();
            if ch != '$' {
                output.push(ch);
            } else {
                let start = at;
                while let Some(c) = text[at..].chars().next() {
                    if !(c.is_alphanumeric() || c == '_') {
                        break;
                    }
                    at += c.len_utf8();
                }
                if start == at {
                    if text[at..].starts_with('{') {
                        return Err("Braced variable references remain unresolved".into());
                    }
                    output.push('$');
                } else {
                    if self.unsupported_names {
                        return Err(
                            "Non-identifier local definitions need full native resolution".into(),
                        );
                    }
                    let value = self
                        .values
                        .get(&text[start..at])
                        .and_then(Option::as_ref)
                        .ok_or("Variable has no known preceding local value")?;
                    output.push_str(value);
                    changed = true;
                }
            }
            if output.len() > MAX_VALUE {
                return Err("Expanded local value exceeds 64 KiB".into());
            }
        }
        Ok((output, changed))
    }
}
