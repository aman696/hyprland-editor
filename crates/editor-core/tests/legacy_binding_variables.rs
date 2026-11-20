use codec::Entry;
use hyprland_editor_core::{catalogue, legacy, legacy_binding as codec, Value, BASELINE};
fn projection(draft: &codec::Draft) -> String {
    codec::export(draft).unwrap()
}
#[test]
fn preceding_locals_aliases_reassignments_flags_and_submaps_populate_the_view() {
    let source="$mainMod = SUPER\n$terminal = kitty\n$alias = $mainMod\nbind = $mainMod,R,exec,$terminal\n$mainMod = ALT\nbind = $mainMod,Q,exec,$terminal --class editor\nbind = $alias,T,exec,$terminal\n$flags = d\n$description = Launch app\nbind$flags = $mainMod,A,$description,exec,$terminal\n$map = resize\nsubmap = $map\nbind = ,catchall,submap,reset\nsubmap = reset\n";
    let report = codec::inspect(source).unwrap();
    assert_eq!(report.unresolved_count, 0);
    assert_eq!(report.draft.entries.len(), 7);
    assert_eq!(report.derived_sites.len(), 5);
    assert!(report.draft.local_variables);
    for (index, modifier) in [(0, "SUPER"), (1, "ALT"), (2, "SUPER")] {
        assert!(
            matches!(&report.draft.entries[index],Entry::Bind{chord,..}if chord.modifiers==[modifier])
        );
    }
    assert!(
        matches!(&report.draft.entries[3],Entry::Bind{description:Some(description),..}if description=="Launch app")
    );
    assert!(matches!(&report.draft.entries[4],Entry::Submap{name,..}if name=="resize"));
    assert_eq!(codec::edited_copy(source, &report.draft).unwrap(), source);
}
#[test]
fn field_edits_preserve_other_references_comments_and_combined_source_changes() {
    let source="$mainMod = SUPER\r\n$key = R\r\n$action = exec\r\n$terminal = kitty\r\nbind = $mainMod,$key,$action,$terminal --class editor,extra # keep shortcut\r\ngeneral:border_size = 2 # scalar\r\nenv = THEME,Nord # env\r\nmonitor = DP-1,transform,1\r\n";
    let doc = legacy::Document::new(source.into(), &catalogue()).unwrap();
    let mut target = doc.bindings.draft.clone();
    if let Entry::Bind {
        chord, description, ..
    } = &mut target.entries[0]
    {
        chord.keys = vec!["Q".into()];
        *description = Some("Launch #1".into());
    }
    let mut scalar = doc.report.draft.clone();
    scalar
        .values
        .insert("general:border_size".into(), Value::Int(5));
    let output = doc
        .edited_with_bindings(
            &scalar,
            &doc.session.draft,
            &doc.monitors.rules,
            &target,
            &catalogue(),
        )
        .unwrap();
    assert!(output.contains(
        "bindd = $mainMod,Q,Launch ##1,$action,$terminal --class editor,extra # keep shortcut\r\n"
    ));
    assert!(output.contains("$key = R\r\n") && output.contains("border_size = 5 # scalar"));
    assert_eq!(
        projection(&codec::inspect(&output).unwrap().draft),
        projection(&target)
    );
    // Editing only the command leaves the original modifier and key references.
    let mut target = doc.bindings.draft.clone();
    if let Entry::Bind { argument, .. } = &mut target.entries[0] {
        *argument = "other application".into();
    }
    let output = codec::edited_copy(source, &target).unwrap();
    assert!(output.contains("bind = $mainMod,$key,$action,other application # keep shortcut"));
}
#[test]
fn source_links_distinguish_equal_rows_and_follow_duplicate_reorder_and_undo() {
    let source="$first = application\n$second = application\nbind = SUPER,R,exec,$first # one\nbind = SUPER,R,exec,$second # two\n";
    let before = codec::inspect(source).unwrap().draft;
    assert_eq!(before.entries[0], before.entries[1]);
    let mut changed = before.clone();
    changed.swap(0, 1);
    assert_ne!(changed, before);
    let output = codec::edited_copy(source, &changed).unwrap();
    assert!(output.contains("bind = SUPER,R,exec,$second # one"));
    assert!(output.contains("bind = SUPER,R,exec,$first # two"));
    let mut duplicate = before.clone();
    duplicate.duplicate(0);
    let output = codec::edited_copy(source, &duplicate).unwrap();
    assert_eq!(output.matches("exec,$first").count(), 2);
    assert_eq!(output.matches("exec,$second").count(), 1);
    let a = hyprland_editor_core::binding::Draft {
        legacy: Some(before.clone()),
        ..Default::default()
    };
    let b = hyprland_editor_core::binding::Draft {
        legacy: Some(changed.clone()),
        ..Default::default()
    };
    let inverse = hyprland_editor_core::binding::Inverse::between(&a, &b);
    let mut restored = b;
    inverse.restore(&mut restored);
    assert_eq!(restored, a);
    let source="$mod = SUPER\nbind = $mod,R,exec,application\n$mod = ALT\nbind = $mod,Q,exec,application\n";
    let mut moved = codec::inspect(source).unwrap().draft;
    moved.swap(0, 1);
    assert!(codec::edited_copy(source, &moved).is_err());
    moved.materialize(0);
    moved.materialize(1);
    let output = codec::edited_copy(source, &moved).unwrap();
    assert!(output.contains("bind = ALT,Q,exec,application"));
    assert!(output.contains("bind = SUPER,R,exec,application"));
    assert_eq!(
        projection(&codec::inspect(&output).unwrap().draft),
        projection(&moved)
    );
}
#[test]
fn external_forward_expression_prefix_and_include_values_remain_unresolved() {
    let source="$ambient = $HOME\nbind = SUPER,R,exec,$ambient\n$forward = $later\n$later = app\nbind = SUPER,Q,exec,$forward\n$mod = SUPER\nsource = unknown.conf\nsubmap = reset\nbind = $mod,T,exec,unknown\n$mod = ALT\nbind = $mod,A,exec,known\n$expression = {{2+2}}\nbind = SUPER,X,exec,$expression\n$whole = SUPER,W,exec,app\nbind = $whole\n";
    let report = codec::inspect(source).unwrap();
    assert_eq!(report.draft.entries.len(), 2);
    assert_eq!(report.unresolved_count, 6);
    assert!(matches!(&report.draft.entries[1],Entry::Bind{chord,..}if chord.modifiers==["ALT"]));
    let bad_name =
        "$mod = SUPER\n$mod,R = something\nbind = $mod,R,exec,app\nbind = SUPER,Q,exec,literal\n";
    let report = codec::inspect(bad_name).unwrap();
    assert_eq!(report.unresolved_count, 1);
    assert_eq!(report.draft.entries.len(), 1);
    let prefix = "$mod = SUPER\nbind = $modSuffix,R,exec,app\n";
    assert!(codec::inspect(prefix).unwrap().draft.is_empty());
}
#[test]
fn literal_projects_upgrade_explicitly_and_expansion_amplification_is_bounded() {
    let source="$mod = SUPER\n$cmd = application\nbind = $mod,R,exec,$cmd # derived\nbind = SUPER,Q,exec,literal # literal\n";
    let mut literal = codec::inspect_mode(source, false).unwrap().draft;
    assert!(!literal.local_variables);
    assert_eq!(literal.entries.len(), 1);
    if let Entry::Bind { argument, .. } = &mut literal.entries[0] {
        *argument = "edited literal".into();
    }
    let copy = codec::edited_copy(source, &literal).unwrap();
    assert!(copy.contains("bind = $mod,R,exec,$cmd # derived"));
    let expanded = codec::expanded_view(source, &literal).unwrap();
    assert_eq!(expanded.entries.len(), 2);
    assert!(expanded.local_variables);
    let copy = codec::edited_copy(source, &expanded).unwrap();
    assert!(copy.contains("exec,edited literal # literal"));
    assert!(copy.contains("exec,$cmd # derived"));
    literal.entries.clear();
    let expanded = codec::expanded_view(source, &literal).unwrap();
    assert_eq!(expanded.entries.len(), 1);
    assert!(codec::accepts_schema(
        &codec::literal_schema_hash(),
        &literal
    ));
    assert!(!codec::accepts_schema(
        &codec::literal_schema_hash(),
        &expanded
    ));
    assert!(codec::accepts_schema(&codec::schema_hash(), &expanded));
    let old: codec::Draft = serde_json::from_str("{\"entries\":[]}").unwrap();
    assert!(!old.local_variables);
    assert_eq!(serde_json::to_string(&old).unwrap(), "{\"entries\":[]}");
    let source = format!(
        "$cmd = {}\n{}",
        "a".repeat(8000),
        "bind = SUPER,R,exec,$cmd\n".repeat(1000)
    );
    assert!(source.len() < 65536);
    assert!(codec::inspect(&source).is_err());
    assert!(legacy::Document::new_with_binding_mode(source.clone(), &catalogue(), false).is_ok());
    assert_eq!(
        codec::inspect_mode(&source, false)
            .unwrap()
            .unresolved_count,
        1000
    );
    let mut invalid = expanded;
    invalid.source_links = vec![Some(0)];
    assert!(invalid.validate(BASELINE).is_err());
}
