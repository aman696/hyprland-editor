//! Pure candidate plans for pinned upstream initial module loaders. No filesystem access.
//! These plans do not resolve package.loaded/preload/searcher overrides or authorize reads.
use crate::{lua::source_hash, AUDITED_MAIN, BASELINE};
use serde::{Deserialize, Serialize};

pub const MAX_MODULE_BYTES: usize = 16_384;
pub const MAX_SEARCH_PATH_BYTES: usize = 65_536;
pub const MAX_CANDIDATES: usize = 512;
pub const MAX_PLAN_BYTES: usize = 1_048_576;
pub const MAX_BATCH_BYTES: usize = 2 * MAX_PLAN_BYTES;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequireMode {
    #[default]
    Safe,
    Original,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    /// Exact main configuration path, including the filename. Never a path capability.
    pub main_file: Vec<u8>,
    pub home: Option<Vec<u8>>,
    /// Runtime/build inherited package.path, before Hyprland prepends its main-directory pair.
    /// None preserves an unknown tail; it never silently assumes the host build's defaults.
    pub inherited_package_path: Option<Vec<u8>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Primary {
    ExplicitFiles {
        base: Vec<u8>,
        candidates: Vec<Vec<u8>>,
    },
    ExplicitWildcard {
        pattern: Vec<u8>,
        watcher_parent_candidate: Vec<u8>,
    },
    Named,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub primary: Primary,
    /// Original Lua-file searcher candidates; also the fallback after explicit file misses.
    /// Wildcards use the dedicated wrapper instead, so have no searcher fallback here.
    pub lua_file_candidates: Vec<Vec<u8>>,
    pub inherited_search_unknown: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePlan {
    pub primary: Primary,
    /// Exact current package.path templates, without another main-directory pair.
    pub lua_file_candidates: Vec<Vec<u8>>,
    /// The known prefix may be tried; exhausting it does not establish a miss.
    pub search_unknown: bool,
}
pub fn schema_hash() -> String {
    source_hash(include_str!("../../../data/source-graph-semantics.json"))
}
fn valid_bytes(bytes: &[u8], max: usize) -> Result<(), String> {
    if bytes.len() > max || bytes.contains(&0) {
        return Err("Module path exceeds its byte limit or contains NUL".into());
    }
    Ok(())
}
pub fn is_explicit(module: &[u8]) -> bool {
    [b"/".as_slice(), b"./", b"../", b"~/"]
        .iter()
        .any(|p| module.starts_with(p))
}
pub fn has_glob(module: &[u8]) -> bool {
    module.iter().any(|c| b"*?[".contains(c))
}
/// Pure Linux lexical spelling used only AFTER an explicit candidate's status
/// observation. This is never a containment check or evidence of file identity.
pub fn normalized_load_path(path: &[u8]) -> Result<Vec<u8>, String> {
    valid_bytes(path, 4096)?;
    if path.is_empty() || path.iter().all(|b| *b == b'/') {
        return Ok(path.to_vec());
    }
    let root = path.starts_with(b"/");
    let mut parts: Vec<&[u8]> = vec![];
    let mut trailing = path.ends_with(b"/");
    for part in path.split(|b| *b == b'/').filter(|p| !p.is_empty()) {
        if part == b"." {
            trailing = true;
            continue;
        }
        trailing = false;
        if part == b".." {
            if parts.last().is_some_and(|p| *p != b"..") {
                parts.pop();
                trailing = true;
            } else if !root {
                parts.push(part);
            }
        } else {
            parts.push(part);
        }
    }
    trailing |= path.ends_with(b"/");
    if parts.is_empty() {
        return Ok(if root { b"/".to_vec() } else { b".".to_vec() });
    }
    let mut out = if root { vec![b'/'] } else { vec![] };
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push(b'/');
        }
        out.extend(*part);
    }
    if trailing && parts.last() != Some(&b"..".as_slice()) {
        out.push(b'/');
    }
    Ok(out)
}
fn parent(bytes: &[u8]) -> &[u8] {
    bytes
        .iter()
        .rposition(|c| *c == b'/')
        .map_or(bytes, |i| &bytes[..i])
}
fn absolute(module: &[u8], context: &Context) -> Result<Vec<u8>, String> {
    let directory = parent(&context.main_file);
    let (prefix, remainder) = if module.starts_with(b"~/") {
        (
            context
                .home
                .as_deref()
                .ok_or("Home path is unknown for tilde resolution")?,
            &module[1..],
        )
    } else if module.starts_with(b"../") {
        (parent(directory), &module[2..])
    } else if module.starts_with(b"./") {
        (directory, &module[1..])
    } else {
        return Ok(module.to_vec());
    };
    Ok([prefix, remainder].concat())
}
fn lua_candidates(
    module: &[u8],
    directory: &[u8],
    inherited: Option<&[u8]>,
) -> Result<Vec<Vec<u8>>, String> {
    // ConfigManager builds this pair through filesystem::path::parent_path and '/'.
    // Its filename separator run is not retained as a trailing directory component.
    let directory = &directory[..directory
        .iter()
        .rposition(|c| *c != b'/')
        .map_or(0, |i| i + 1)];
    // Lua searchpath replaces every '.' with the directory separator before template expansion.
    let module: Vec<_> = module
        .iter()
        .map(|c| if *c == b'.' { b'/' } else { *c })
        .collect();
    let mut templates = vec![
        [directory, b"/?.lua"].concat(),
        [directory, b"/?/init.lua"].concat(),
    ];
    if let Some(path) = inherited {
        valid_bytes(path, MAX_SEARCH_PATH_BYTES)?;
        // Empty entries are real failed searchpath candidates; only a wholly empty
        // inherited path is omitted by the upstream ConfigManager constructor.
        if !path.is_empty() {
            append_templates(&mut templates, path)?;
        }
    }
    expand_templates(&module, templates)
}
fn append_templates(templates: &mut Vec<Vec<u8>>, path: &[u8]) -> Result<(), String> {
    for template in path.split(|b| *b == b';') {
        if templates.len() >= MAX_CANDIDATES {
            return Err("Module search has more than 512 templates".into());
        }
        templates.push(template.to_vec());
    }
    Ok(())
}
fn expand_templates(module: &[u8], templates: Vec<Vec<u8>>) -> Result<Vec<Vec<u8>>, String> {
    if templates.len() > MAX_CANDIDATES {
        return Err("Module search has more than 512 templates".into());
    }
    let mut total = 0usize;
    templates
        .into_iter()
        .map(|template| {
            let length = template
                .len()
                .checked_sub(template.iter().filter(|c| **c == b'?').count())
                .and_then(|length| {
                    length.checked_add(
                        template
                            .iter()
                            .filter(|c| **c == b'?')
                            .count()
                            .saturating_mul(module.len()),
                    )
                })
                .ok_or("Module candidate size overflow")?;
            total = total
                .checked_add(length)
                .ok_or("Module candidate total overflow")?;
            if total > MAX_PLAN_BYTES {
                return Err("Module candidate plan exceeds 1 MiB".into());
            }
            if length > MAX_SEARCH_PATH_BYTES {
                return Err("Expanded module candidate exceeds 64 KiB".into());
            }
            let mut out = Vec::with_capacity(length);
            for c in template {
                if c == b'?' {
                    out.extend_from_slice(module);
                } else {
                    out.push(c);
                }
            }
            Ok(out)
        })
        .collect()
}
/// Bounded batch transport for a restricted worker; no environment or filesystem is queried.
pub fn plan_requests(
    context: &Context,
    modules: &[Vec<u8>],
    version: &str,
) -> Result<Vec<Plan>, String> {
    plan_requests_for(context, modules, version, RequireMode::Safe)
}
pub fn plan_requests_for(
    context: &Context,
    modules: &[Vec<u8>],
    version: &str,
    mode: RequireMode,
) -> Result<Vec<Plan>, String> {
    if modules.len() > MAX_CANDIDATES {
        return Err("At most 512 module requests can be planned together".into());
    }
    context.validate(version)?;
    let mut plans = Vec::with_capacity(modules.len());
    let mut encoded = 0usize;
    for module in modules {
        let plan = context.plan_for(module, version, mode)?;
        encoded += serde_json::to_vec(&plan).map_err(|e| e.to_string())?.len() + 1;
        if encoded > MAX_BATCH_BYTES {
            return Err("Encoded module plans exceed 2 MiB".into());
        }
        plans.push(plan);
    }
    Ok(plans)
}
impl Context {
    /// The exact source-derived initial prefix. Unknown inherited entries remain
    /// separate; this function never reads host Lua defaults or environment.
    pub fn initial_path(&self, version: &str) -> Result<(Vec<u8>, bool), String> {
        self.validate(version)?;
        let directory = parent(&self.main_file);
        let directory = &directory[..directory
            .iter()
            .rposition(|c| *c != b'/')
            .map_or(0, |i| i + 1)];
        let mut path = [directory, b"/?.lua;", directory, b"/?/init.lua"].concat();
        if let Some(tail) = &self.inherited_package_path {
            if !tail.is_empty() {
                path.push(b';');
                path.extend(tail);
            }
        }
        valid_bytes(&path, MAX_SEARCH_PATH_BYTES)?;
        Ok((path, self.inherited_package_path.is_none()))
    }
    pub fn plan_current(
        &self,
        module: &[u8],
        version: &str,
        mode: RequireMode,
        package_path: Option<&[u8]>,
        unknown_tail: bool,
    ) -> Result<RuntimePlan, String> {
        if unknown_tail && package_path.is_none_or(|path| path.is_empty()) {
            return Err("Unknown path tail needs its known prefix".into());
        }
        // Only the primary path wrapper depends on the main-file context.
        let mut context = self.clone();
        context.inherited_package_path = Some(vec![]);
        let primary = context.plan_for(module, version, mode)?.primary;
        if matches!(primary, Primary::ExplicitWildcard { .. }) {
            return Ok(RuntimePlan {
                primary,
                lua_file_candidates: vec![],
                search_unknown: false,
            });
        }
        let candidates = if let Some(path) = package_path {
            valid_bytes(path, MAX_SEARCH_PATH_BYTES)?;
            let name: Vec<_> = module
                .iter()
                .map(|b| if *b == b'.' { b'/' } else { *b })
                .collect();
            let mut templates = vec![];
            append_templates(&mut templates, path)?;
            expand_templates(&name, templates)?
        } else {
            vec![]
        };
        Ok(RuntimePlan {
            primary,
            lua_file_candidates: candidates,
            search_unknown: unknown_tail || package_path.is_none(),
        })
    }
    fn validate(&self, version: &str) -> Result<(), String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version) {
            return Err("Module resolution needs an audited snapshot".into());
        }
        valid_bytes(&self.main_file, MAX_MODULE_BYTES)?;
        if !self.main_file.starts_with(b"/") || self.main_file.ends_with(b"/") {
            return Err(
                "Module planning requires the exact absolute main configuration filename".into(),
            );
        }
        if let Some(home) = &self.home {
            valid_bytes(home, MAX_MODULE_BYTES)?;
            if !home.starts_with(b"/") {
                return Err("Captured home path must be absolute".into());
            }
        }
        if let Some(path) = &self.inherited_package_path {
            valid_bytes(path, MAX_SEARCH_PATH_BYTES)?;
        }
        Ok(())
    }
    /// Candidate plan for the safe require wrapper; use plan_for for __require.
    pub fn plan(&self, module: &[u8], version: &str) -> Result<Plan, String> {
        self.plan_for(module, version, RequireMode::Safe)
    }
    pub fn plan_for(
        &self,
        module: &[u8],
        version: &str,
        mode: RequireMode,
    ) -> Result<Plan, String> {
        self.validate(version)?;
        valid_bytes(module, MAX_MODULE_BYTES)?;
        let primary = if is_explicit(module) && !(mode == RequireMode::Original && has_glob(module))
        {
            let base = absolute(module, self)?;
            if has_glob(module) {
                let meta = base
                    .iter()
                    .position(|c| b"*?[".contains(c))
                    .ok_or("Wildcard expansion context changed")?;
                let slash = base[..meta].iter().rposition(|c| *c == b'/');
                let watcher_parent_candidate = match slash {
                    Some(0) => b"/".to_vec(),
                    Some(n) => base[..n].to_vec(),
                    None => vec![],
                };
                Primary::ExplicitWildcard {
                    pattern: base,
                    watcher_parent_candidate,
                }
            } else {
                let mut candidates = vec![base.clone()];
                if !base.ends_with(b".lua") {
                    candidates.push([base.as_slice(), b".lua"].concat());
                }
                // std::filesystem::path(BASE) / init.lua adds no extra separator after '/'.
                candidates.push(
                    [
                        base.as_slice(),
                        if base.ends_with(b"/") {
                            b"init.lua"
                        } else {
                            b"/init.lua"
                        },
                    ]
                    .concat(),
                );
                Primary::ExplicitFiles { base, candidates }
            }
        } else {
            Primary::Named
        };
        let wildcard = matches!(primary, Primary::ExplicitWildcard { .. });
        Ok(Plan {
            primary,
            lua_file_candidates: if wildcard {
                vec![]
            } else {
                lua_candidates(
                    module,
                    parent(&self.main_file),
                    self.inherited_package_path.as_deref(),
                )?
            },
            inherited_search_unknown: !wildcard && self.inherited_package_path.is_none(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_search_and_lexical_load_names_match_independent_observations() {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/runtime-path-fixtures.json")).unwrap();
        assert_eq!(data["audit_manifest_sha256"], schema_hash());
        let c = Context {
            main_file: b"/fixture/config/main.lua".to_vec(),
            home: None,
            inherited_package_path: None,
        };
        let replace = |s: &str| s.replace("<root>", "/fixture").into_bytes();
        for row in data["search"].as_array().unwrap() {
            let module = replace(row["module"].as_str().unwrap());
            let path = replace(row["package_path"].as_str().unwrap());
            let plan = c
                .plan_current(&module, BASELINE, RequireMode::Original, Some(&path), false)
                .unwrap();
            assert!(!plan.search_unknown);
            if let Some(found) = row["selected"].as_str() {
                assert!(plan.lua_file_candidates.contains(&replace(found)), "{row}");
            } else {
                assert_eq!(
                    plan.lua_file_candidates,
                    row["missing_candidates"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|p| replace(p.as_str().unwrap()))
                        .collect::<Vec<_>>(),
                    "{row}"
                );
            }
        }
        for row in data["normalization"].as_array().unwrap() {
            assert_eq!(
                normalized_load_path(row["path"].as_str().unwrap().as_bytes()).unwrap(),
                row["normalized"].as_str().unwrap().as_bytes(),
                "{row}"
            );
        }
        let (prefix, unknown) = c.initial_path(BASELINE).unwrap();
        assert!(unknown);
        let partial = c
            .plan_current(b"part", BASELINE, RequireMode::Safe, Some(&prefix), unknown)
            .unwrap();
        assert_eq!(
            partial.lua_file_candidates,
            vec![
                b"/fixture/config/part.lua".to_vec(),
                b"/fixture/config/part/init.lua".to_vec()
            ]
        );
        assert!(partial.search_unknown);
        let replaced = c
            .plan_current(
                b"part",
                BASELINE,
                RequireMode::Safe,
                Some(b"/vendor/?.lua"),
                false,
            )
            .unwrap();
        assert_eq!(
            replaced.lua_file_candidates,
            vec![b"/vendor/part.lua".to_vec()]
        );
        assert!(!replaced.search_unknown);
    }
    fn context() -> Context {
        Context {
            main_file: b"<root>/config/hyprland.lua".to_vec(),
            home: Some(b"<root>/home".to_vec()),
            inherited_package_path: None,
        }
    }
    #[test]
    fn path_plans_match_independent_source_status_queries() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/source-graph-fixtures.json")).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            // Probe paths are made absolute; substitute a stable absolute fixture root in model inputs.
            let replace = |s: &str| s.replace("<root>", "/fixture").into_bytes();
            let mut c = context();
            c.main_file = replace(case["main"].as_str().unwrap());
            c.home = Some(replace("<root>/home"));
            let name = replace(case["module"].as_str().unwrap());
            let version = if case["version"] == "baseline" {
                BASELINE
            } else {
                AUDITED_MAIN
            };
            let plan = c.plan(&name, version).unwrap();
            assert_eq!(is_explicit(&name), case["explicit"].as_bool().unwrap());
            assert_eq!(has_glob(&name), case["wildcard"].as_bool().unwrap());
            for search in case["search_cases"].as_array().unwrap() {
                c.inherited_package_path = Some(replace(search["inherited"].as_str().unwrap()));
                // Query Lua's original file searcher independently of the explicit/wildcard wrapper.
                let candidates = lua_candidates(
                    &name,
                    parent(&c.main_file),
                    c.inherited_package_path.as_deref(),
                )
                .unwrap();
                if let Some(selected) = search["selected"].as_str() {
                    assert!(
                        candidates.contains(&replace(selected)),
                        "{name:?}: {selected}"
                    );
                } else {
                    let missing = search["missing_candidates"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|s| replace(s.as_str().unwrap()))
                        .collect::<Vec<_>>();
                    assert_eq!(candidates, missing, "Lua searchpath: {name:?}");
                }
            }
            match plan.primary {
                Primary::ExplicitFiles { base, candidates } => {
                    assert_eq!(base, replace(case["base"].as_str().unwrap()), "{name:?}");
                    let attempted = case["attempted"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|s| replace(s.as_str().unwrap()))
                        .collect::<Vec<_>>();
                    assert!(!attempted.is_empty());
                    assert_eq!(&candidates[..attempted.len()], attempted, "{name:?}");
                }
                Primary::ExplicitWildcard {
                    pattern,
                    watcher_parent_candidate,
                } => {
                    assert_eq!(pattern, replace(case["base"].as_str().unwrap()));
                    let attempts = case["attempted"].as_array().unwrap();
                    assert_eq!(
                        watcher_parent_candidate,
                        replace(attempts[0].as_str().unwrap())
                    );
                    assert!(plan.lua_file_candidates.is_empty());
                }
                Primary::Named => assert!(!case["explicit"].as_bool().unwrap()),
            }
        }
    }
    #[test]
    fn bytes_and_unknown_runtime_search_tail_are_not_host_defaults() {
        let mut c = Context {
            main_file: b"/config/\xff/main.lua".to_vec(),
            home: None,
            inherited_package_path: None,
        };
        let p = c.plan(b"colors.theme", BASELINE).unwrap();
        assert_eq!(
            p.lua_file_candidates,
            vec![
                b"/config/\xff/colors/theme.lua".to_vec(),
                b"/config/\xff/colors/theme/init.lua".to_vec()
            ]
        );
        assert!(p.inherited_search_unknown);
        c.inherited_package_path = Some(b";/vendor/?.lua;;/alternate/?/?.lua;".to_vec());
        let p = c.plan(b"colors.theme", BASELINE).unwrap();
        assert!(!p.inherited_search_unknown);
        assert_eq!(
            p.lua_file_candidates[5],
            b"/alternate/colors/theme/colors/theme.lua"
        );
        assert!(c.plan(b"~/theme", BASELINE).is_err());
        assert!(c.plan(b"./bad\0name", BASELINE).is_err());
        c.inherited_package_path = Some(b"?;".repeat(MAX_CANDIDATES));
        assert!(c.plan(b"x", BASELINE).is_err());
    }
    #[test]
    fn traversal_is_not_lexically_collapsed_before_source_file_status() {
        let c = Context {
            main_file: b"/config/main.lua".to_vec(),
            home: Some(b"/home/u".to_vec()),
            inherited_package_path: Some(vec![]),
        };
        let Primary::ExplicitFiles { candidates, .. } = c
            .plan(b"./symlink/../target.lua", BASELINE)
            .unwrap()
            .primary
        else {
            panic!()
        };
        assert_eq!(candidates[0], b"/config/symlink/../target.lua");
        let Primary::ExplicitFiles { base, .. } = c.plan(b"../../theme", BASELINE).unwrap().primary
        else {
            panic!()
        };
        assert_eq!(base, b"/../theme");
    }
    #[test]
    fn request_and_expansion_budgets_stop_aggregate_amplification() {
        let mut c = Context {
            main_file: b"/config/main.lua".to_vec(),
            home: None,
            inherited_package_path: Some(b"/x/?.lua;".repeat(500)),
        };
        assert!(c.plan(&vec![b'a'; 4096], BASELINE).is_err());
        assert!(plan_requests(&c, &[], "unreviewed").is_err());
        c.inherited_package_path = Some(b"/x/?.lua;".repeat(100));
        assert!(plan_requests(&c, &vec![vec![b'a'; 4096]; 20], BASELINE).is_err());
        assert!(plan_requests(&c, &vec![b"x".to_vec(); MAX_CANDIDATES + 1], BASELINE).is_err());
    }
    #[test]
    fn original_require_does_not_use_the_wildcard_wrapper() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/module-load-fixtures.json")).unwrap();
        for (version, label) in [(BASELINE, "baseline"), (AUDITED_MAIN, "current")] {
            let c = Context {
                main_file: b"/fixture/config/hyprland.lua".to_vec(),
                home: None,
                inherited_package_path: Some(vec![]),
            };
            let original = c
                .plan_for(b"./literal/*", version, RequireMode::Original)
                .unwrap();
            assert_eq!(original.primary, Primary::Named);
            let source = fixture["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| {
                    c["version"] == label
                        && c["case"] == "original_require_treats_glob_as_literal_filename"
                })
                .unwrap();
            let selected = source["trace"][1][1]
                .as_str()
                .unwrap()
                .replace("<root>", "/fixture")
                .into_bytes();
            assert!(original.lua_file_candidates.contains(&selected));
            let safe = c
                .plan_for(b"./literal/*", version, RequireMode::Safe)
                .unwrap();
            assert!(matches!(safe.primary, Primary::ExplicitWildcard { .. }));
        }
    }
}
