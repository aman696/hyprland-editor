//! Prepare a complete single-source review from graphical workspace drafts.
//! Source is never invoked and this module has no filesystem or compositor API.
use super::{named, parse, syntax_tree, text, LuaReport};
use crate::{
    workspace::{SourceLanguage, Workspace},
    Setting, MAX_DOCUMENT_BYTES,
};
use std::ops::Range;
use tree_sitter::Node;

struct Family {
    names: &'static [&'static str],
    canonical: String,
}

fn families(report: &LuaReport) -> Result<Vec<Family>, String> {
    let version = &report.draft.version;
    Ok(vec![
        Family {
            names: &["hl.monitor"],
            canonical: crate::monitor::export_lua(&report.monitors, version)?,
        },
        Family {
            names: &["hl.device"],
            canonical: crate::device::export_lua(&report.devices, version)?,
        },
        Family {
            names: &["hl.bind", "hl.define_submap"],
            canonical: report.bindings.export_lua(version)?,
        },
        Family {
            names: &["hl.workspace_rule"],
            canonical: crate::workspace_rule::export_lua(&report.workspace_rules, version)?,
        },
        Family {
            names: &["hl.window_rule", "hl.layer_rule"],
            canonical: crate::rule::export_lua(&report.rules, version)?,
        },
        Family {
            names: &["hl.curve", "hl.animation"],
            canonical: report.animations.export_lua(version)?,
        },
        Family {
            names: &["hl.gesture"],
            canonical: report.gestures.export_lua(version)?,
        },
    ])
}

fn selected<'a>(
    node: Node<'a>,
    source: &str,
    names: &[&str],
    output: &mut Vec<Node<'a>>,
) -> Result<(), String> {
    let mut pending = vec![node];
    let mut visited = 0;
    while let Some(node) = pending.pop() {
        visited += 1;
        if visited > 8192 {
            return Err("Source declaration traversal exceeds its limit".into());
        }
        match node.kind() {
            "chunk" | "block" => pending.extend(named(node).into_iter().rev()),
            "do_statement" => {
                if let Some(body) = node.child_by_field_name("body") {
                    pending.push(body);
                }
            }
            "function_call"
                if node
                    .child_by_field_name("name")
                    .is_some_and(|name| names.contains(&text(name, source))) =>
            {
                output.push(node)
            }
            _ => {}
        }
    }
    Ok(())
}

fn rendered_statements(source: &str, names: &[&str]) -> Result<Vec<String>, String> {
    let tree = syntax_tree(source)?;
    let mut calls = vec![];
    selected(tree.root_node(), source, names, &mut calls)?;
    if named(tree.root_node())
        .iter()
        .any(|node| node.kind() != "function_call")
    {
        return Err("A declaration serializer produced an unexpected statement".into());
    }
    if calls.len() != named(tree.root_node()).len() {
        return Err("A declaration serializer produced a different feature family".into());
    }
    Ok(calls
        .into_iter()
        .map(|node| text(node, source).to_owned())
        .collect())
}

fn scalar_copy(
    source: &str,
    before: &LuaReport,
    workspace: &Workspace,
    catalogue: &[Setting],
) -> Result<String, String> {
    use super::replay::scalar_edit::layout::erase_tokens;
    let draft = &workspace.draft;
    let tree = syntax_tree(source)?;
    let mut edits = Vec::<(Range<usize>, String)>::new();
    let mut separators = std::collections::BTreeSet::new();
    let mut inserted = String::new();
    let changed_presence = before
        .draft
        .values
        .keys()
        .any(|key| !draft.values.contains_key(key))
        || draft
            .values
            .keys()
            .any(|key| !before.draft.values.contains_key(key));
    if changed_presence && (before.opaque_effects || !before.diagnostics.is_empty()) {
        return Err("Adding or removing scalar source fields needs a fully recognized static file. Save the project to retain edits to unresolved source.".into());
    }
    for key in before
        .draft
        .values
        .keys()
        .filter(|key| !draft.values.contains_key(*key))
    {
        for declaration in before
            .declarations
            .iter()
            .filter(|declaration| &declaration.id == key)
        {
            if declaration.value.is_none() {
                return Err(
                    "An unresolved declaration cannot be removed from this source copy.".into(),
                );
            }
            let value = tree
                .root_node()
                .descendant_for_byte_range(declaration.span.start, declaration.span.end)
                .filter(|node| node.byte_range() == declaration.span)
                .ok_or("Scalar expression span is not an exact syntax node")?;
            let field = value
                .parent()
                .filter(|node| {
                    node.kind() == "field" && node.child_by_field_name("value") == Some(value)
                })
                .ok_or("Scalar expression has no independent table field")?;
            edits.push((field.byte_range(), erase_tokens(field, source)?));
            let mut following = field.next_sibling();
            while following.is_some_and(|node| node.kind() == "comment") {
                following = following.unwrap().next_sibling();
            }
            let mut previous = field.prev_sibling();
            while previous.is_some_and(|node| node.kind() == "comment") {
                previous = previous.unwrap().prev_sibling();
            }
            if let Some(separator) = following
                .filter(|node| matches!(node.kind(), "," | ";"))
                .or_else(|| previous.filter(|node| matches!(node.kind(), "," | ";")))
            {
                separators.insert((separator.start_byte(), separator.end_byte()));
            }
        }
    }
    for (key, value) in &draft.values {
        if serde_json::to_vec(&before.draft.values.get(key)).map_err(|error| error.to_string())?
            == serde_json::to_vec(&Some(value)).map_err(|error| error.to_string())?
        {
            continue;
        }
        if let Some(declaration) = before
            .declarations
            .iter()
            .rev()
            .find(|declaration| declaration.id == *key)
        {
            if declaration.value.is_none() {
                return Err(
                    "An unresolved scalar expression cannot be replaced in this source copy."
                        .into(),
                );
            }
            let node = tree
                .root_node()
                .descendant_for_byte_range(declaration.span.start, declaration.span.end)
                .filter(|node| node.byte_range() == declaration.span)
                .ok_or("Scalar expression span is not an exact syntax node")?;
            edits.push((
                declaration.span.clone(),
                format!("{}{}", value.lua(), erase_tokens(node, source)?),
            ));
        } else {
            let setting = catalogue
                .iter()
                .find(|setting| setting.version == draft.version && setting.id == *key)
                .ok_or("Unknown scalar setting")?;
            inserted.push_str(&format!(
                "    [{}] = {},\n",
                crate::quote_lua(&setting.lua_key),
                value.lua()
            ));
        }
    }
    edits.extend(
        separators
            .into_iter()
            .map(|(start, end)| (start..end, String::new())),
    );
    edits.sort_by_key(|(range, _)| range.start);
    if edits.windows(2).any(|pair| pair[0].0.end > pair[1].0.start) {
        return Err("Scalar source edits overlap; no copy was produced.".into());
    }
    let mut copy = source.to_owned();
    for (range, replacement) in edits.into_iter().rev() {
        copy.replace_range(range, &replacement);
    }
    if !inserted.is_empty() {
        copy.push_str(&format!("\nhl.config({{\n{inserted}}})\n"));
    }
    Ok(copy)
}

/// This prepares an immutable review only; it never publishes a configuration.
pub fn prepare(workspace: &Workspace, catalogue: &[Setting]) -> Result<String, String> {
    workspace.validate(catalogue)?;
    if workspace.source_graph.is_some() || workspace.sources.len() != 1 {
        return Err(
            "Combined source copies currently require a single retained source file.".into(),
        );
    }
    let root = workspace.root().ok_or("Source snapshot required")?;
    if root.language != SourceLanguage::Lua {
        return Err("Use the legacy source-copy workflow for .conf files.".into());
    }
    let version = &workspace.draft.version;
    // Independently reconstruct this report. Saved/UI reports cannot select spans.
    let original = parse(&root.content, version, catalogue)?;
    let before = families(&original)?;
    let mut target = original.clone();
    target.draft = workspace.draft.clone();
    target.monitors = workspace.monitors.clone();
    target.devices = workspace.devices.clone();
    target.bindings = workspace.bindings.clone();
    target.workspace_rules = workspace.workspace_rules.clone();
    target.rules = workspace.rules.clone();
    target.animations = workspace.animations.clone();
    target.gestures = workspace.gestures.clone();
    let after = families(&target)?;
    let structured_changed = before
        .iter()
        .zip(&after)
        .any(|(a, b)| a.canonical != b.canonical);
    if structured_changed && (original.opaque_effects || !original.diagnostics.is_empty()) {
        return Err("Structured source changes need a fully recognized static file. Unresolved source remains saved in the project; no incomplete copy was produced.".into());
    }
    let mut copy = scalar_copy(&root.content, &original, workspace, catalogue)?;
    if structured_changed {
        let tree = syntax_tree(&copy)?;
        let mut edits = Vec::<(Range<usize>, String)>::new();
        let mut appended = String::new();
        for (old, new) in before.iter().zip(&after) {
            if old.canonical == new.canonical {
                continue;
            }
            let mut calls = vec![];
            selected(tree.root_node(), &copy, old.names, &mut calls)?;
            let rendered = rendered_statements(&new.canonical, new.names)?;
            for (index, call) in calls.iter().enumerate() {
                let trivia = super::replay::scalar_edit::layout::erase_tokens(*call, &copy)?;
                let replacement = if let Some(statement) = rendered.get(index) {
                    format!("{statement}\n{trivia}")
                } else {
                    trivia
                };
                edits.push((call.byte_range(), replacement));
            }
            // Existing slots keep their positions relative to other families.
            // Additional declarations are placed after the final family slot.
            if rendered.len() > calls.len() {
                let extra = rendered[calls.len()..].join("\n") + "\n";
                if let Some(last) = calls.last() {
                    let (range, replacement) =
                        edits.last_mut().ok_or("Missing final declaration edit")?;
                    if last.byte_range() != *range {
                        return Err("Declaration edit order changed".into());
                    }
                    replacement.push_str(&extra);
                } else {
                    appended.push_str(&extra);
                }
            }
        }
        edits.sort_by_key(|(range, _)| range.start);
        if edits.windows(2).any(|pair| pair[0].0.end > pair[1].0.start) {
            return Err("Changed declaration scopes overlap; no copy was produced.".into());
        }
        for (range, replacement) in edits.into_iter().rev() {
            copy.replace_range(range, &replacement);
        }
        if !appended.is_empty() {
            copy.push('\n');
            copy.push_str(&appended);
        }
    }
    if copy.len() > MAX_DOCUMENT_BYTES {
        return Err("The edited source exceeds 1 MiB.".into());
    }
    let checked = parse(&copy, version, catalogue)?;
    // Reparse every family, including those not changed by this operation.
    // This catches nested/shared declarations that cannot be rewritten locally.
    if serde_json::to_vec(&checked.draft.values).map_err(|error| error.to_string())?
        != serde_json::to_vec(&target.draft.values).map_err(|error| error.to_string())?
        || families(&checked)?
            .iter()
            .zip(&after)
            .any(|(a, b)| a.canonical != b.canonical)
        || (structured_changed && (checked.opaque_effects || !checked.diagnostics.is_empty()))
    {
        return Err("The prepared source does not reproduce all requested drafts. Nested or shared declarations need explicit source editing; no incomplete copy was produced.".into());
    }
    Ok(copy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalogue, storage, AUDITED_MAIN, BASELINE};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn snapshot(source: &str, target: &str, version: &str) -> Workspace {
        let catalogue = catalogue();
        let parsed = parse(target, version, &catalogue).unwrap();
        assert!(
            !parsed.opaque_effects && parsed.diagnostics.is_empty(),
            "{:?}",
            parsed.diagnostics
        );
        let path = std::env::temp_dir().join(format!(
            "hye-source-copy-{}-{}.lua",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, source).unwrap();
        let captured = storage::read_source(&path).unwrap();
        fs::remove_file(path).unwrap();
        Workspace::new(parsed.draft)
            .with_root(captured.content, captured.identity)
            .with_monitors(parsed.monitors)
            .with_devices(parsed.devices)
            .with_bindings(parsed.bindings)
            .with_workspace_rules(parsed.workspace_rules)
            .with_rules(parsed.rules)
            .with_animations(parsed.animations)
            .with_gestures(parsed.gestures)
    }

    const SOURCE: &str = "-- café header\nhl.monitor{ -- keep monitor comment\n output='DP-1'}\nhl.config{general={border_size=2}}\nhl.device{name='test mouse',sensitivity=0.1}\nhl.bind('Q',hl.dsp.no_op())\nhl.workspace_rule{workspace='1'}\nhl.window_rule{match={class='term'},float=true}\nhl.layer_rule{match={namespace='panel'},blur=true}\nhl.curve('ease',{type='bezier',points={{0.1,0.2},{0.8,0.9}}})\nhl.animation{leaf='windows',enabled=true,speed=1,bezier='ease'}\nhl.gesture{fingers=3,direction='left',action={start=function()hl.dispatch(hl.dsp.no_op())end,finish=function()end}}\n-- kept footer\n";

    #[test]
    fn all_graphical_families_and_scalar_edits_roundtrip_together_without_original_files() {
        for version in [BASELINE, AUDITED_MAIN] {
            let target = SOURCE
                .replace("DP-1", "DP-2")
                .replace("border_size=2", "border_size=5")
                .replace("sensitivity=0.1", "sensitivity=0.3")
                .replace("'Q'", "'W'")
                .replace("workspace='1'", "workspace='2'")
                .replace("true", "false")
                .replace("fingers=3", "fingers=4");
            let workspace = snapshot(SOURCE, &target, version);
            let copy = prepare(&workspace, &catalogue()).unwrap();
            assert!(copy.contains("-- café header"));
            assert!(copy.contains("-- keep monitor comment"));
            assert!(copy.contains("-- kept footer"));
            assert!(copy.find("hl.monitor").unwrap() < copy.find("hl.config").unwrap());
            assert!(copy.find("hl.config").unwrap() < copy.find("hl.device").unwrap());
            assert_eq!(workspace.root().unwrap().content, SOURCE);
            let target = parse(&target, version, &catalogue()).unwrap();
            let checked = parse(&copy, version, &catalogue()).unwrap();
            for (a, b) in families(&target)
                .unwrap()
                .iter()
                .zip(families(&checked).unwrap())
            {
                assert_eq!(a.canonical, b.canonical);
            }
        }
    }

    #[test]
    fn unchanged_copies_are_byte_exact_and_all_families_can_be_added_or_removed() {
        for version in [BASELINE, AUDITED_MAIN] {
            let source = SOURCE.replace('\n', "\r\n");
            let unchanged = snapshot(&source, &source, version);
            assert_eq!(prepare(&unchanged, &catalogue()).unwrap(), source);
            let scalar_only = "hl.config{general={border_size=2}}\n";
            let added = snapshot(scalar_only, SOURCE, version);
            let copy = prepare(&added, &catalogue()).unwrap();
            assert!(copy.starts_with(scalar_only));
            let removed = snapshot(SOURCE, scalar_only, version);
            let copy = prepare(&removed, &catalogue()).unwrap();
            assert!(copy.contains("-- keep monitor comment"));
            assert!(copy.contains(scalar_only.trim()));
            assert!(!copy.contains("hl.monitor"));
            assert!(!copy.contains("hl.bind"));
        }
    }

    #[test]
    fn declaration_counts_order_submaps_and_do_scopes_are_preserved_in_the_result() {
        let source = "do hl.monitor{output='DP-1'};hl.config{general={border_size=2}};hl.monitor{output='DP-2'} end\nhl.define_submap('resize',function()hl.bind('Q',hl.dsp.no_op())end)\n";
        let target = "hl.monitor{output='DP-3'};hl.monitor{output='DP-4'};hl.monitor{output='DP-5'};hl.config{general={border_size=2}};hl.define_submap('resize',function()hl.bind('W',hl.dsp.no_op());hl.bind('E',hl.dsp.no_op())end)";
        let copy = prepare(&snapshot(source, target, BASELINE), &catalogue()).unwrap();
        assert!(copy.starts_with("do hl.monitor"));
        assert!(copy.find("DP-3").unwrap() < copy.find("hl.config").unwrap());
        assert!(copy.find("hl.config").unwrap() < copy.find("DP-4").unwrap());
        assert!(copy.find("DP-4").unwrap() < copy.find("DP-5").unwrap());
        assert!(copy.contains("hl.define_submap"));
    }

    #[test]
    fn unresolved_source_and_overlapping_nested_families_cannot_silently_lose_drafts() {
        let target = "hl.monitor{output='DP-2'}";
        let workspace = snapshot("hl.monitor{output='DP-1'};untrusted()", target, BASELINE);
        assert!(prepare(&workspace, &catalogue()).is_err());
        let nested = "hl.define_submap('map',function()hl.bind('Q',hl.dsp.no_op());hl.monitor{output='DP-1'}end)";
        let workspace = snapshot(nested,"hl.define_submap('map',function()hl.bind('W',hl.dsp.no_op())end);hl.monitor{output='DP-1'}",BASELINE);
        assert!(prepare(&workspace, &catalogue()).is_err());
        assert_eq!(workspace.root().unwrap().content, nested);
    }

    #[test]
    fn scalar_presence_duplicate_removal_and_computed_values_preserve_comments_and_other_fields() {
        for version in [BASELINE, AUDITED_MAIN] {
            let source = "-- retained\nlocal size=2\nhl.config{general={border_size=1, -- earlier\n gaps_in=3}}\nhl.config{general={border_size= -- expression\n size+1; -- separator\n gaps_out=4}}\n";
            let target = "hl.config{general={gaps_in=3,gaps_out=4},decoration={rounding=7}}";
            let copy = prepare(&snapshot(source, target, version), &catalogue()).unwrap();
            for comment in ["-- retained", "-- earlier", "-- expression", "-- separator"] {
                assert!(copy.contains(comment));
            }
            assert!(!copy.contains("border_size"));
            assert!(copy.contains("local size=2"));
            let source = "local size=2\nhl.config{general={border_size=size+1}}";
            let target = "hl.config{general={border_size=5}}";
            let copy = prepare(&snapshot(source, target, version), &catalogue()).unwrap();
            assert!(copy.contains("local size=2"));
            assert!(!copy.contains("size+1"));
            for source in [
                "hl.config{general={border_size=1,gaps_in=3}}",
                "hl.config{general={border_size=1;gaps_in=3;}}",
            ] {
                let copy = prepare(&snapshot(source, "", version), &catalogue()).unwrap();
                assert!(parse(&copy, version, &catalogue())
                    .unwrap()
                    .draft
                    .values
                    .is_empty());
            }
        }
    }
}
