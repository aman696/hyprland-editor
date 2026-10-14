//! Static workspace filter builders; action/navigation selectors use a different grammar.
use crate::{AUDITED_MAIN, BASELINE};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    pub min: u64,
    pub max: u64,
    pub kind: WindowKind,
    pub pinned: bool,
    pub groups: bool,
    pub visible: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum WindowKind {
    All,
    Tiled,
    Floating,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Predicate {
    Range { min: u64, max: u64 },
    Special(bool),
    Named(bool),
    NamePrefix(String),
    NameSuffix(String),
    Monitor(String),
    Count(Counts),
    Fullscreen(i8),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Selector {
    All,
    Number(u64),
    Named(String),
    Special { name: Option<String> },
    Filters(Vec<Predicate>),
    Unresolved(String),
}
fn plain(s: &str) -> bool {
    !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control)
}
fn bound(version: &str) -> u64 {
    if version == AUDITED_MAIN {
        u64::from(u32::MAX)
    } else {
        i64::MAX as u64
    }
}
impl Predicate {
    pub fn encode(&self, version: &str) -> Result<String, String> {
        let limit = bound(version);
        let range = |min: u64, max: u64| -> Result<String, String> {
            if min == 0 || max < min || max > limit {
                return Err("Filter ranges must be positive, ordered and within this snapshot's integer range".into());
            }
            Ok(format!("{min}-{max}"))
        };
        Ok(match self {
            Self::Range { min, max } => format!("r[{}]", range(*min, *max)?),
            Self::Special(v) => format!("s[{}]", u8::from(*v)),
            Self::Named(v) => format!("n[{}]", u8::from(*v)),
            Self::NamePrefix(s) | Self::NameSuffix(s) | Self::Monitor(s) => {
                if !plain(s) || s.contains(']') {
                    return Err(
                        "Filter text must be nonempty and cannot contain a closing bracket".into(),
                    );
                }
                format!(
                    "{}{}]",
                    match self {
                        Self::NamePrefix(_) => "n[s:",
                        Self::NameSuffix(_) => "n[e:",
                        _ => "m[",
                    },
                    s
                )
            }
            Self::Count(c) => {
                let mut flags = match c.kind {
                    WindowKind::All => String::new(),
                    WindowKind::Tiled => "t".into(),
                    WindowKind::Floating => "f".into(),
                };
                for (on, ch) in [(c.pinned, 'p'), (c.groups, 'g'), (c.visible, 'v')] {
                    if on {
                        flags.push(ch);
                    }
                }
                let count = if c.min == c.max {
                    if c.max > limit {
                        return Err("Count exceeds snapshot integer range".into());
                    }
                    c.min.to_string()
                } else {
                    range(c.min, c.max)?
                };
                format!("w[{flags}{count}]")
            }
            Self::Fullscreen(n) => {
                if !(-1..=2).contains(n) {
                    return Err("Fullscreen filter state must be -1, 0, 1 or 2".into());
                }
                format!("f[{n}]")
            }
        })
    }
}
impl Selector {
    pub fn encode(&self, version: &str) -> Result<String, String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version) {
            return Err("Unknown workspace selector snapshot".into());
        }
        Ok(match self {
            Self::All => String::new(),
            Self::Number(n) => {
                if *n == 0 || *n > bound(version) {
                    return Err("Workspace number is outside the snapshot range".into());
                }
                n.to_string()
            }
            Self::Named(s) => {
                if !plain(s) || version == AUDITED_MAIN && s.parse::<f64>().is_ok() {
                    return Err("Choose a nonempty, nonnumeric workspace name".into());
                }
                format!("name:{s}")
            }
            Self::Special { name: None } => "special".into(),
            Self::Special { name: Some(s) } => {
                if !plain(s) {
                    return Err("Choose a special workspace name".into());
                }
                format!("special:{s}")
            }
            Self::Filters(v) => {
                if v.is_empty() || v.len() > 32 {
                    return Err("Choose 1–32 AND filter clauses".into());
                }
                v.iter()
                    .map(|p| p.encode(version))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(" ")
            }
            Self::Unresolved(s) => {
                if s.len() > 16384 || s.contains('\0') {
                    return Err("Invalid literal selector".into());
                }
                s.clone()
            }
        })
    }
    /// Decode builder-representable syntax. Retain other source spellings without
    /// treating them as validated or as a declaration of a nonexistent workspace.
    pub fn parse(text: &str, version: &str) -> Self {
        let parsed = (|| -> Option<Self> {
            let t = text.trim();
            if t.is_empty() {
                return Some(Self::All);
            }
            if let Some(s) = t.strip_prefix("name:") {
                return Some(Self::Named(s.into()));
            }
            if t == "special" {
                return Some(Self::Special { name: None });
            }
            if let Some(s) = t.strip_prefix("special:") {
                return Some(Self::Special {
                    name: Some(s.into()),
                });
            }
            if let Some(n) = t
                .bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| t.parse::<u64>().ok())
                .flatten()
            {
                return Some(Self::Number(n));
            }
            let mut rest = t;
            let mut clauses = vec![];
            while !rest.is_empty() {
                if clauses.len() >= 32 {
                    return None;
                }
                rest = rest.trim_start();
                let end = rest.find(']')?;
                let token = &rest[..=end];
                if token.len() < 3 || token.as_bytes()[1] != b'[' {
                    return None;
                }
                let value = &token[2..token.len() - 1];
                let boolean = || match value {
                    "0" | "false" => Some(false),
                    "1" | "true" => Some(true),
                    _ => None,
                };
                let range = |s: &str| -> Option<(u64, u64)> {
                    let (a, b) = s.split_once('-')?;
                    Some((a.parse().ok()?, b.parse().ok()?))
                };
                let p = match token.as_bytes()[0] {
                    b'r' => {
                        let (min, max) = range(value)?;
                        Predicate::Range { min, max }
                    }
                    b's' => Predicate::Special(boolean()?),
                    b'n' => {
                        if let Some(s) = value.strip_prefix("s:") {
                            Predicate::NamePrefix(s.into())
                        } else if let Some(s) = value.strip_prefix("e:") {
                            Predicate::NameSuffix(s.into())
                        } else {
                            Predicate::Named(boolean()?)
                        }
                    }
                    b'm' => Predicate::Monitor(value.into()),
                    b'f' => Predicate::Fullscreen(value.parse().ok()?),
                    b'w' => {
                        let mut value = value;
                        let mut c = Counts {
                            min: 0,
                            max: 0,
                            kind: WindowKind::All,
                            pinned: false,
                            groups: false,
                            visible: false,
                        };
                        let mut seen = std::collections::BTreeSet::new();
                        while let Some(ch) = value.chars().next().filter(|c| "tfpgv".contains(*c)) {
                            if !seen.insert(ch) {
                                return None;
                            }
                            value = &value[1..];
                            match ch {
                                't' | 'f' => {
                                    if c.kind != WindowKind::All {
                                        return None;
                                    }
                                    c.kind = if ch == 't' {
                                        WindowKind::Tiled
                                    } else {
                                        WindowKind::Floating
                                    };
                                }
                                'p' => c.pinned = true,
                                'g' => c.groups = true,
                                'v' => c.visible = true,
                                _ => unreachable!(),
                            }
                        }
                        (c.min, c.max) = if value.contains('-') {
                            range(value)?
                        } else {
                            let n = value.parse().ok()?;
                            (n, n)
                        };
                        Predicate::Count(c)
                    }
                    _ => return None,
                };
                clauses.push(p);
                rest = &rest[end + 1..];
            }
            Some(Self::Filters(clauses))
        })();
        match parsed {
            Some(p) if p.encode(version).is_ok() => p,
            _ => Self::Unresolved(text.into()),
        }
    }
    pub fn declaration_address(&self) -> bool {
        matches!(
            self,
            Self::Number(_) | Self::Named(_) | Self::Special { .. }
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graphical_filters_encode_all_source_clause_kinds_and_round_trip() {
        let s = Selector::Filters(vec![
            Predicate::Range { min: 1, max: 9 },
            Predicate::Special(false),
            Predicate::Named(true),
            Predicate::NamePrefix("work".into()),
            Predicate::NameSuffix("dev".into()),
            Predicate::Monitor("DP-1".into()),
            Predicate::Count(Counts {
                min: 1,
                max: 3,
                kind: WindowKind::Floating,
                pinned: true,
                groups: true,
                visible: true,
            }),
            Predicate::Fullscreen(-1),
        ]);
        for version in [BASELINE, AUDITED_MAIN] {
            let text = s.encode(version).unwrap();
            assert_eq!(
                text,
                "r[1-9] s[0] n[1] n[s:work] n[e:dev] m[DP-1] w[fpgv1-3] f[-1]"
            );
            assert_eq!(Selector::parse(&text, version), s);
        }
        assert!(!s.declaration_address());
        assert!(Selector::Named("work".into()).declaration_address());
    }
    #[test]
    fn version_bounds_unknown_navigation_and_injection_remain_explicit() {
        assert!(Selector::Number(u64::from(u32::MAX) + 1)
            .encode(BASELINE)
            .is_ok());
        assert!(Selector::Number(u64::from(u32::MAX) + 1)
            .encode(AUDITED_MAIN)
            .is_err());
        for text in [
            "r[0-5]",
            "w[tf1]",
            "n[s:]",
            "f[3]",
            "r[1-3]evil",
            "w[0-2]",
            "prev",
            "+1",
        ] {
            assert!(
                matches!(Selector::parse(text, BASELINE), Selector::Unresolved(_)),
                "{text}"
            );
        }
        assert!(Predicate::Monitor("DP-1] n[1".into())
            .encode(BASELINE)
            .is_err());
        assert!(Selector::Named("123".into()).encode(AUDITED_MAIN).is_err());
        assert_eq!(
            Selector::parse("w[0]", BASELINE).encode(BASELINE).unwrap(),
            "w[0]"
        );
    }
}
