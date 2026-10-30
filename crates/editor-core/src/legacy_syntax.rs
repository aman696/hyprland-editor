//! Shared bounded legacy lexical projection. No expansion or host reads.
use crate::MAX_DOCUMENT_BYTES;
use std::ops::Range;
pub(crate) fn trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_ascii_whitespace())
}
// Match the pinned hyprlang hash scan, including its skipped byte after ##.
// Quotes do not protect # in this grammar. Offsets still refer to input bytes.
pub(crate) fn code(line: &str) -> (String, usize) {
    let bytes = line.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            if bytes.get(i + 1) == Some(&b'#') {
                result.push(b'#');
                i += 2;
                if i < bytes.len() {
                    result.push(bytes[i]);
                    i += 1;
                }
                continue;
            }
            break;
        }
        result.push(bytes[i]);
        i += 1;
    }
    (
        String::from_utf8(result).expect("Hash removal preserves UTF-8"),
        i,
    )
}
pub(crate) fn has_expansion(value: &str) -> bool {
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '$'
            && chars
                .clone()
                .next()
                .is_some_and(|next| next.is_alphanumeric() || next == '_' || next == '{')
        {
            return true;
        }
    }
    false
}
pub(crate) fn unescape(value: &str) -> Result<String, String> {
    if value.len() > 64 * 1024 || has_expansion(value) {
        return Err("Variable expansion remains unresolved".into());
    }
    let bytes = value.as_bytes();
    for (at, pair) in bytes.windows(2).enumerate() {
        if pair == b"{{" {
            let count = bytes[..at]
                .iter()
                .rev()
                .take_while(|b| **b == b'\\')
                .count();
            if count % 2 == 0 {
                return Err("Legacy expressions remain unresolved".into());
            }
        }
    }
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\'
            && bytes
                .get(i + 1)
                .is_some_and(|b| matches!(b, b'\\' | b'{' | b'}'))
        {
            i += 1;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).map_err(|_| "Invalid legacy string".into())
}
pub(crate) fn csv(value: &str) -> Vec<String> {
    let mut fields = vec![];
    let (mut begin, mut escaped) = (0, vec![]);
    let bytes = value.as_bytes();
    let finish = |end: usize, begin: usize, escaped: &[usize]| {
        let mut skip = escaped.iter().copied().peekable();
        let part = bytes[begin..end]
            .iter()
            .enumerate()
            .filter_map(|(at, b)| {
                if skip.peek() == Some(&at) {
                    skip.next();
                    None
                } else {
                    Some(*b)
                }
            })
            .collect::<Vec<_>>();
        trim(std::str::from_utf8(&part).expect("Delimiter removal preserves UTF-8")).to_owned()
    };
    for (at, b) in bytes.iter().enumerate() {
        if *b != b',' {
            continue;
        }
        if at > begin && bytes[at - 1] == b'\\' {
            escaped.push(at - begin - 1);
            if at - begin == 1 || bytes[at - 2] != b'\\' {
                continue;
            }
        }
        fields.push(finish(at, begin, &escaped));
        begin = at + 1;
        escaped.clear();
    }
    if begin < bytes.len() {
        fields.push(finish(bytes.len(), begin, &escaped));
    }
    fields
}
pub(crate) fn literal(value: &str) -> Result<String, String> {
    if trim(value) != value || value.chars().any(|c| c.is_control()) || has_expansion(value) {
        return Err("This value needs legacy expansion, escaping or multiline syntax. Its draft can be saved, but no incomplete source copy was produced.".into());
    }
    // Encoding runs this way reproduces the pinned lexer's ## scan exactly.
    let mut result = String::new();
    let mut hashes = 0usize;
    for ch in value.chars().chain(std::iter::once('\0')) {
        if ch == '#' {
            hashes += 1;
            continue;
        }
        if hashes > 0 {
            result.push_str(&"#".repeat(hashes + hashes.div_ceil(2)));
            hashes = 0;
        }
        if ch != '\0' {
            if matches!(ch, '\\' | '{' | '}') {
                result.push('\\');
            }
            result.push(ch);
        }
    }
    Ok(result)
}

pub(crate) struct Statement<'a> {
    pub line: usize,
    pub key: &'a str,
    pub value: &'a str,
    pub statement: Range<usize>,
    pub editable: bool,
}
/// Walk declarations without collecting arbitrarily many source lines.
pub(crate) fn walk(content: &str, mut visit: impl FnMut(Statement<'_>)) -> Result<(), String> {
    if content.len() > MAX_DOCUMENT_BYTES || content.contains('\0') {
        return Err("Legacy source exceeds its size policy or contains NUL".into());
    }
    let (mut offset, mut depth, mut continued, mut flags) = (0usize, 0usize, false, false);
    for (index, line) in content.split_inclusive('\n').enumerate() {
        let physical = line.trim_end_matches(['\r', '\n']);
        let previous = continued;
        continued = physical.ends_with('\\');
        let raw = trim(physical);
        if raw.starts_with('#') {
            if trim(raw.trim_start_matches('#')).starts_with("hyprlang") {
                flags = true;
            }
            offset += line.len();
            continue;
        }
        let (decoded, end) = code(physical);
        let value = trim(&decoded);
        if let Some((key, rhs)) = value.split_once('=') {
            let rawcode = &physical[..end];
            let statement = offset + rawcode.len()
                - rawcode
                    .trim_start_matches(|c: char| c.is_ascii_whitespace())
                    .len()
                ..offset
                    + rawcode
                        .trim_end_matches(|c: char| c.is_ascii_whitespace())
                        .len();
            visit(Statement {
                line: index + 1,
                key: trim(key),
                value: trim(rhs),
                statement,
                editable: depth == 0 && !flags && !previous && !continued,
            });
        } else if value == "}" {
            depth = depth.saturating_sub(1);
        } else if value.ends_with('{') {
            depth = depth
                .checked_add(1)
                .ok_or("Legacy category depth overflow")?;
            if depth > 16 {
                return Err("Legacy category nesting exceeds 16".into());
            }
        }
        offset += line.len();
    }
    Ok(())
}
