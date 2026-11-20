//! Bounded expansion of definitions encountered during native source traversal.
use crate::{legacy_syntax::trim, legacy_variables::identifier};
use std::{collections::BTreeMap, sync::Arc};

const MAX_NAMES: usize = 1_024;
const MAX_NAME: usize = 256;
const MAX_VALUE: usize = 65_536;
const MAX_RETAINED: usize = 262_144;
const MAX_ORIGINS: usize = 24;
const MAX_BYTES_PROCESSED: usize = 8 * 1_024 * 1_024;
const MAX_STEPS: usize = 262_144;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    pub name: Arc<str>,
    pub file: Option<usize>,
    pub line: usize,
}

pub(super) struct Resolution {
    pub text: String,
    pub origins: Vec<Origin>,
}

pub(super) struct Variables {
    values: BTreeMap<Arc<str>, Option<Resolution>>,
    retained: usize,
    remaining_bytes: usize,
    remaining_steps: usize,
    reliable: bool,
}

impl Default for Variables {
    fn default() -> Self {
        Self {
            values: BTreeMap::new(),
            retained: 0,
            remaining_bytes: MAX_BYTES_PROCESSED,
            remaining_steps: MAX_STEPS,
            reliable: true,
        }
    }
}

impl Variables {
    pub fn invalidate(&mut self) {
        self.reliable = false;
        self.values.clear();
        self.retained = 0;
    }

    pub fn define(&mut self, name: &Arc<str>, raw: Option<&str>, file: Option<usize>, line: usize) {
        if !self.reliable {
            return;
        }
        if !identifier(name) || (self.values.len() == MAX_NAMES && !self.values.contains_key(name))
        {
            self.invalidate();
            return;
        }
        let expanded = raw.and_then(|raw| self.expand(raw));
        if let Some(previous) = self.values.remove(name).flatten() {
            self.retained -= previous.text.len();
        }
        let definition = expanded.and_then(|mut value| {
            if self.retained + value.text.len() > MAX_RETAINED {
                return None;
            }
            append_origin(
                &mut value.origins,
                &Origin {
                    name: name.clone(),
                    file,
                    line,
                },
            )?;
            self.retained += value.text.len();
            Some(value)
        });
        // An unresolved longer name still shadows a known shorter prefix.
        self.values.insert(name.clone(), definition);
    }

    pub fn expand(&mut self, raw: &str) -> Option<Resolution> {
        if !self.reliable || raw.len() > MAX_VALUE {
            return None;
        }
        self.remaining_bytes = self.remaining_bytes.checked_sub(raw.len())?;
        self.remaining_steps = self.remaining_steps.checked_sub(1)?;
        let mut resolved = Resolution {
            text: String::new(),
            origins: Vec::new(),
        };
        let mut rest = raw;
        while let Some((literal, reference)) = rest.split_once('$') {
            push(&mut resolved.text, literal, &mut self.remaining_bytes)?;
            // Consecutive name tokens can form a different name if the
            // second replacement runs first. Keep that case unresolved.
            if reference
                .trim_start_matches(|ch: char| ch.is_alphanumeric() || ch == '_')
                .starts_with('$')
            {
                return None;
            }
            // The pinned parser sorts by name length. Resolved definitions
            // contain no dollar signs, so inserting one cannot form a new match.
            let (name, value) = longest_prefix(&self.values, reference, &mut self.remaining_steps)?;
            push(&mut resolved.text, &value.text, &mut self.remaining_bytes)?;
            for origin in &value.origins {
                append_origin(&mut resolved.origins, origin)?;
            }
            rest = &reference[name.len()..];
        }
        push(&mut resolved.text, rest, &mut self.remaining_bytes)?;
        if resolved.text.contains("{{") {
            resolved.text = expressions(
                &resolved.text,
                &self.values,
                &mut resolved.origins,
                &mut self.remaining_bytes,
                &mut self.remaining_steps,
            )?;
        }
        Some(resolved)
    }
}

fn longest_prefix<'a>(
    values: &'a BTreeMap<Arc<str>, Option<Resolution>>,
    text: &str,
    remaining_steps: &mut usize,
) -> Option<(&'a str, &'a Resolution)> {
    let mut end = 0;
    for (offset, ch) in text.char_indices() {
        let next = offset + ch.len_utf8();
        if next > MAX_NAME || !(ch.is_alphanumeric() || ch == '_') {
            break;
        }
        end = next;
    }
    while end > 0 {
        *remaining_steps = remaining_steps.checked_sub(1)?;
        if let Some((key, value)) = values.get_key_value(&text[..end]) {
            return value.as_ref().map(|value| (key.as_ref(), value));
        }
        end = text[..end].char_indices().next_back()?.0;
    }
    None
}

fn push(output: &mut String, text: &str, remaining_bytes: &mut usize) -> Option<()> {
    if output.len() + text.len() > MAX_VALUE {
        return None;
    }
    *remaining_bytes = remaining_bytes.checked_sub(text.len())?;
    output.push_str(text);
    Some(())
}

fn append_origin(origins: &mut Vec<Origin>, origin: &Origin) -> Option<()> {
    if !origins.contains(origin) {
        if origins.len() == MAX_ORIGINS {
            return None;
        }
        origins.push(origin.clone());
    }
    Some(())
}

fn expressions(
    value: &str,
    variables: &BTreeMap<Arc<str>, Option<Resolution>>,
    origins: &mut Vec<Origin>,
    remaining_bytes: &mut usize,
    remaining_steps: &mut usize,
) -> Option<String> {
    let mut output = String::new();
    let mut rest = value;
    while let Some((prefix, rhs)) = rest.split_once("{{") {
        *remaining_steps = remaining_steps.checked_sub(1)?;
        // Escape pairing and brace boundaries can change after substitution.
        // Leave those spellings unresolved until their exact parsing is modeled.
        if prefix.ends_with(['{', '}', '\\']) {
            return None;
        }
        let (body, tail) = rhs.split_once("}}")?;
        if body.contains(['{', '}']) || tail.starts_with('}') {
            return None;
        }
        let mut fields = body.split(' ').filter(|field| !field.is_empty());
        let (left, operator, right) = (fields.next()?, fields.next()?, fields.next()?);
        if fields.next().is_some() {
            return None;
        }
        let left = number(left, variables, origins)?;
        let right = number(right, variables, origins)?;
        let value = match operator {
            "+" => left + right,
            "-" => left - right,
            "*" => left * right,
            "/" => left / right,
            _ => return None,
        };
        // For this bounded range, the pinned float formatting uses a plain
        // shortest decimal. Leave scientific and non-finite results unresolved.
        if value != 0.0 && !(0.001..=10_000.0).contains(&value.abs()) {
            return None;
        }
        push(&mut output, prefix, remaining_bytes)?;
        let shortest = value.to_string();
        let places = shortest
            .split_once('.')
            .map_or(0, |(_, fraction)| fraction.len());
        // Precision formatting rounds decimal ties to even. The shortest
        // Rust display spelling by itself can choose the other tied digit.
        push(&mut output, &format!("{value:.places$}"), remaining_bytes)?;
        rest = tail;
    }
    push(&mut output, rest, remaining_bytes)?;
    Some(output)
}

fn number(
    token: &str,
    variables: &BTreeMap<Arc<str>, Option<Resolution>>,
    origins: &mut Vec<Origin>,
) -> Option<f32> {
    let text = if let Some(value) = variables.get(token) {
        let value = value.as_ref()?;
        for origin in &value.origins {
            append_origin(origins, origin)?;
        }
        trim(&value.text)
    } else {
        token
    };
    let value: f32 = text.parse().ok()?;
    if value == 0.0 {
        let significand = text.split(['e', 'E']).next()?;
        return (!significand.bytes().any(|b| matches!(b, b'1'..=b'9'))).then_some(value);
    }
    (value.is_finite() && value.abs() >= 2.0 * f32::MIN_POSITIVE).then_some(value)
}
